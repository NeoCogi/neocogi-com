//! Typed method plans selected from resolved IDL ownership, never reparsed Rust types.
use super::*;
use crate::{
    Parameter,
    model::{ParameterPassing, ReturnPassing},
};
use crate::codegen::model::{AbiArg, AbiArgKind, AbiMethod, AbiReceiver, AbiReturn, AbiReturnKind, InterfaceBorrow, InterfaceOwnership};
use syn::parse::Parser;

/// Lowers source methods once; traits, wrappers, thunks and metadata share this plan.
pub(super) fn methods(graph: &mut Generation<'_>, module: &ResolvedModule, methods: &[Method]) -> Result<Vec<AbiMethod>, GenerateError> {
    methods
        .iter()
        .map(|method| {
            let docs = syn::Attribute::parse_outer
                .parse2(origin_docs(&method.docs, &method.span))
                .map_err(syntax_error)?;
            Ok(AbiMethod {
                ident: rust_ident(&method.name)?,
                doc_attrs: docs,
                receiver: match method.receiver {
                    Receiver::Shared => AbiReceiver::Shared,
                    Receiver::Mutable => AbiReceiver::Mutable,
                },
                args: method
                    .parameters
                    .iter()
                    .map(|parameter| argument(graph, module, parameter))
                    .collect::<Result<_, _>>()?,
                ok: result(graph, module, method)?,
                projection: method.return_projection,
            })
        })
        .collect()
}

/// Parses emitted Rust syntax only; all semantic selection has already used IDL nodes.
fn rust_type(tokens: TokenStream) -> Result<syn::Type, GenerateError> {
    syn::parse2(tokens).map_err(syntax_error)
}

/// Reports an emitter bug without reclassifying it as an authored language error.
fn syntax_error(error: syn::Error) -> GenerateError {
    GenerateError::new(format!("invalid emitted Rust type: {error}"))
}

/// Selects one exact interface marker and threading proof from its bound target.
pub(super) fn object(graph: &mut Generation<'_>, module: &ResolvedModule, target: &IdlType) -> Result<(syn::Type, Option<Box<syn::Type>>), GenerateError> {
    let IdlType::Named(name) = target else {
        return Err(GenerateError::new("resolved object target is not named"));
    };
    let nc = &graph.support;
    if let Some(abi) = builtin_object_type(nc, name) {
        return Ok((rust_type(abi)?, None));
    }
    let resolved = resolve(graph, module, name)?;
    let DeclarationKind::Interface(interface) = &resolved.declaration.kind else {
        return Err(GenerateError::new("resolved object target is a value"));
    };
    let mode = match interface.threading {
        Threading::Local => None,
        Threading::Shared => Some(Box::new(syn::parse_quote!(#nc::ThreadSafe))),
        Threading::Send => Some(Box::new(syn::parse_quote!(#nc::ThreadSend))),
        Threading::Sync => Some(Box::new(syn::parse_quote!(#nc::ThreadSync))),
    };
    Ok((rust_type(named_rust_type(graph, module, resolved, true)?)?, mode))
}

/// Builds a bounded readable/writable element plan without inspecting Rust wrappers.
fn buffer(element_ty: syn::Type, output: bool, nc: &syn::Path) -> (syn::Type, syn::Type, AbiArgKind) {
    if output {
        (
            syn::parse_quote!(#nc::NcOutputWriter<'_, #element_ty>),
            syn::parse_quote!(*mut <#element_ty as #nc::ComValue>::AbiRepr),
            AbiArgKind::OutputBuffer { element_ty: Box::new(element_ty) },
        )
    } else {
        (
            syn::parse_quote!(#nc::NcCopyFromBufferOffset<'_, #element_ty>),
            syn::parse_quote!(*const <#element_ty as #nc::ComValue>::AbiRepr),
            AbiArgKind::CopyFromBufferOffset { element_ty: Box::new(element_ty) },
        )
    }
}

/// Ownership alternatives are exhaustive over the validated IDL input grammar.
fn argument(graph: &mut Generation<'_>, module: &ResolvedModule, parameter: &Parameter) -> Result<AbiArg, GenerateError> {
    let ty = parameter.payload();
    let (rust_ty, abi_ty, kind) = match parameter.passing {
        ParameterPassing::Copied => {
            let ty = rust_type(value_type(graph, module, ty)?)?;
            let nc = &graph.support;
            (ty.clone(), syn::parse_quote!(<#ty as #nc::ComValue>::AbiRepr), AbiArgKind::Plain)
        }
        ParameterPassing::InputBuffer => buffer(rust_type(value_type(graph, module, ty)?)?, false, &graph.support),
        ParameterPassing::OutputBuffer => buffer(rust_type(value_type(graph, module, ty)?)?, true, &graph.support),
        ParameterPassing::BorrowedText => buffer(syn::parse_quote!(::core::primitive::u8), false, &graph.support),
        ParameterPassing::BorrowedValue => {
            let value_ty = rust_type(value_type(graph, module, ty)?)?;
            let nc = &graph.support;
            (
                syn::parse_quote!(#nc::NcValueRef<'_, #value_ty>),
                syn::parse_quote!(*const <#value_ty as #nc::ComValue>::AbiRepr),
                AbiArgKind::BorrowedValue { value_ty: Box::new(value_ty) },
            )
        }
        ParameterPassing::SharedLoan { .. } | ParameterPassing::MutableLoan { .. } => {
            let (interface, mode_ty) = object(graph, module, ty)?;
            let nc = &graph.support;
            let mode = mode_ty.as_ref().map(|mode| quote! { , #mode });
            let (access, rust_ty) = match parameter.passing {
                ParameterPassing::SharedLoan { optional: false } => (InterfaceBorrow::Shared, quote! { #nc::ComRef<'_, #interface #mode> }),
                ParameterPassing::SharedLoan { optional: true } => (
                    InterfaceBorrow::OptionalShared,
                    quote! { ::core::option::Option<#nc::ComRef<'_, #interface #mode>> },
                ),
                ParameterPassing::MutableLoan { optional: false } => (InterfaceBorrow::Mutable, quote! { #nc::ComMut<'_, #interface #mode> }),
                ParameterPassing::MutableLoan { optional: true } => (
                    InterfaceBorrow::OptionalMutable,
                    quote! { ::core::option::Option<#nc::ComMut<'_, #interface #mode>> },
                ),
                _ => unreachable!("filtered interface loans"),
            };
            (
                rust_type(rust_ty)?,
                syn::parse_quote!(#nc::ComRaw<#interface>),
                AbiArgKind::ComRef {
                    interface_ty: Box::new(interface),
                    mode_ty,
                    access,
                },
            )
        }
        ParameterPassing::ConsumingBox => {
            let (interface, mode_ty) = object(graph, module, ty)?;
            let nc = &graph.support;
            let mode = mode_ty.as_ref().map(|mode| quote! { , #mode });
            (
                rust_type(quote! { #nc::ComBox<#interface #mode> })?,
                syn::parse_quote!(*mut #nc::ComRaw<#interface>),
                AbiArgKind::ComBox {
                    interface_ty: Box::new(interface),
                    mode_ty,
                },
            )
        }
        ParameterPassing::Unresolved | ParameterPassing::Invalid => return Err(GenerateError::new("input slot lacks its resolved passing rule")),
    };
    let ident = rust_ident(&parameter.name)?;
    // The backend assigns collision-free positional bindings before emission.
    Ok(AbiArg {
        binding: ident.clone(),
        ident,
        rust_ty,
        abi_ty,
        kind,
    })
}

/// Uses the bound return slot to emit exactly one transfer or loan contract.
fn result(graph: &mut Generation<'_>, module: &ResolvedModule, method: &Method) -> Result<AbiReturn, GenerateError> {
    let ty = method.return_payload();
    let (ownership, wrapper, optional) = match method.return_passing {
        ReturnPassing::Unit => {
            return Ok(AbiReturn {
                rust_ty: syn::parse_quote!(()),
                abi_ty: syn::parse_quote!(()),
                kind: AbiReturnKind::Void,
            });
        }
        ReturnPassing::Copied => {
            let ty = rust_type(value_type(graph, module, ty)?)?;
            let nc = &graph.support;
            return Ok(AbiReturn {
                rust_ty: ty.clone(),
                abi_ty: syn::parse_quote!(<#ty as #nc::ComValue>::AbiRepr),
                kind: AbiReturnKind::Plain,
            });
        }
        ReturnPassing::SharedOwner { optional } => (InterfaceOwnership::Shared, quote! { ComPtr }, optional),
        ReturnPassing::BoxOwner => (InterfaceOwnership::Exclusive, quote! { ComBox }, false),
        ReturnPassing::SharedObserver => (InterfaceOwnership::WeakShared, quote! { WeakComPtr }, false),
        ReturnPassing::BorrowedLoan { mutable, .. } | ReturnPassing::ScopedLoan { mutable } => {
            let (interface_ty, mode_ty) = object(graph, module, ty)?;
            let nc = &graph.support;
            let mode = mode_ty.as_ref().map(|mode| quote! { , #mode });
            let wrapper = if mutable {
                quote! { ComMut }
            } else {
                quote! { ComRef }
            };
            let loan = quote! { #nc::#wrapper<'_, #interface_ty #mode> };
            let optional = !matches!(method.return_passing, ReturnPassing::BorrowedLoan { optional: false, .. });
            let rust_ty = if optional {
                quote! { ::core::option::Option<#loan> }
            } else {
                loan
            };
            let abi_ty = syn::parse_quote!(#nc::ComRaw<#interface_ty>);
            let interface_ty = Box::new(interface_ty);
            let kind = if matches!(method.return_passing, ReturnPassing::ScopedLoan { .. }) {
                AbiReturnKind::ScopedLoan { interface_ty, mode_ty, mutable }
            } else {
                AbiReturnKind::BorrowedLoan { interface_ty, mode_ty, mutable, optional }
            };
            return Ok(AbiReturn {
                rust_ty: rust_type(rust_ty)?,
                abi_ty,
                kind,
            });
        }
        ReturnPassing::Unresolved | ReturnPassing::Invalid => return Err(GenerateError::new("result slot lacks its resolved passing rule")),
    };
    let (interface, mode_ty) = object(graph, module, ty)?;
    let nc = &graph.support;
    let mode = mode_ty.as_ref().map(|mode| quote! { , #mode });
    let owner = quote! { #nc::#wrapper<#interface #mode> };
    let rust_ty = if optional {
        quote! { ::core::option::Option<#owner> }
    } else {
        owner
    };
    let abi_ty = match ownership {
        InterfaceOwnership::WeakShared => syn::parse_quote!(#nc::NcSharedWeakRaw),
        _ => syn::parse_quote!(#nc::ComRaw<#interface>),
    };
    Ok(AbiReturn {
        rust_ty: rust_type(rust_ty)?,
        abi_ty,
        kind: AbiReturnKind::ComPtr {
            optional,
            ownership,
            interface_ty: Box::new(interface),
            mode_ty,
        },
    })
}
