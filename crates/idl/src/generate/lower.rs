//! Typed method plans selected from resolved IDL ownership, never reparsed Rust types.
use super::*;
use crate::Parameter;
use neocogi_com_codegen::model::{AbiArg, AbiArgKind, AbiMethod, AbiReceiver, AbiReturn, AbiReturnKind, InterfaceOwnership};
use syn::parse::Parser;

/// Lowers source methods once; traits, clients, thunks and metadata share this plan.
pub(super) fn methods(graph: &Generation<'_>, module: &ResolvedModule, methods: &[Method]) -> Result<Vec<AbiMethod>, GenerateError> {
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
                ok: result(graph, module, &method.output)?,
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
pub(super) fn object(graph: &Generation<'_>, module: &ResolvedModule, target: &AbiType) -> Result<(syn::Type, Option<Box<syn::Type>>), GenerateError> {
    let AbiType::Named(name) = target else {
        return Err(GenerateError::new("resolved object target is not named"));
    };
    if let Some(abi) = builtin_object_type(name) {
        return Ok((rust_type(abi)?, None));
    }
    let resolved = resolve(graph, module, name)?;
    let DeclarationKind::Interface(interface) = &resolved.declaration.kind else {
        return Err(GenerateError::new("resolved object target is a value"));
    };
    let mode = match interface.threading {
        Threading::Local => None,
        Threading::Shared => Some(Box::new(syn::parse_quote!(::neocogi_com::ThreadSafe))),
    };
    Ok((rust_type(named_rust_type(graph, module, resolved, true)?)?, mode))
}

/// Builds a bounded readable/writable element plan without inspecting Rust wrappers.
fn buffer(element_ty: syn::Type, output: bool) -> (syn::Type, syn::Type, AbiArgKind) {
    if output {
        (
            syn::parse_quote!(::neocogi_com::NcOutputWriter<'_, #element_ty>),
            syn::parse_quote!(*mut <#element_ty as ::neocogi_com::ComValue>::Wire),
            AbiArgKind::OutputBuffer { element_ty: Box::new(element_ty) },
        )
    } else {
        (
            syn::parse_quote!(::neocogi_com::NcCopyFromBufferOffset<'_, #element_ty>),
            syn::parse_quote!(*const <#element_ty as ::neocogi_com::ComValue>::Wire),
            AbiArgKind::CopyFromBufferOffset { element_ty: Box::new(element_ty) },
        )
    }
}

/// Ownership alternatives are exhaustive over the validated IDL input grammar.
fn argument(graph: &Generation<'_>, module: &ResolvedModule, parameter: &Parameter) -> Result<AbiArg, GenerateError> {
    let (rust_ty, abi_ty, kind) = match &parameter.ty {
        AbiType::Scalar(_) | AbiType::Named(_) | AbiType::Array { .. } => {
            let ty = rust_type(value_type(graph, module, &parameter.ty)?)?;
            (ty.clone(), syn::parse_quote!(<#ty as ::neocogi_com::ComValue>::Wire), AbiArgKind::Plain)
        }
        AbiType::Slice(element) => buffer(rust_type(value_type(graph, module, element)?)?, false),
        AbiType::OutputSlice(element) => buffer(rust_type(value_type(graph, module, element)?)?, true),
        AbiType::Borrowed(target) => {
            let AbiType::Named(name) = target.as_ref() else {
                return Err(GenerateError::new("resolved borrow is neither value nor interface"));
            };
            if name.is_str() {
                buffer(syn::parse_quote!(u8), false)
            } else if builtin_object_type(name).is_some() || matches!(resolve(graph, module, name)?.declaration.kind, DeclarationKind::Interface(_)) {
                let (interface, mode_ty) = object(graph, module, target)?;
                let mode = mode_ty.as_ref().map(|mode| quote! { , #mode });
                (
                    syn::parse2(quote! { ::neocogi_com::ComRef<'_, #interface #mode> }).map_err(syntax_error)?,
                    syn::parse_quote!(::neocogi_com::ComRaw<#interface>),
                    AbiArgKind::ComRef {
                        interface_ty: Box::new(interface),
                        mode_ty,
                    },
                )
            } else {
                let value_ty = rust_type(named_rust_type(graph, module, resolve(graph, module, name)?, false)?)?;
                (
                    syn::parse_quote!(::neocogi_com::NcValueRef<'_, #value_ty>),
                    syn::parse_quote!(*const <#value_ty as ::neocogi_com::ComValue>::Wire),
                    AbiArgKind::BorrowedValue { value_ty: Box::new(value_ty) },
                )
            }
        }
        AbiType::ComBox(target) => {
            let (interface, mode_ty) = object(graph, module, target)?;
            let mode = mode_ty.as_ref().map(|mode| quote! { , #mode });
            (
                syn::parse2(quote! { ::neocogi_com::ComBox<#interface #mode> }).map_err(syntax_error)?,
                syn::parse_quote!(*mut ::neocogi_com::ComRaw<#interface>),
                AbiArgKind::ComBox {
                    interface_ty: Box::new(interface),
                    mode_ty,
                },
            )
        }
        AbiType::ComPtr(_)
        | AbiType::WeakComPtr(_)
        | AbiType::WeakComBox(_)
        | AbiType::Unit
        | AbiType::String
        | AbiType::Vector(_)
        | AbiType::UnsizedSlice(_) => {
            return Err(GenerateError::new("invalid input survived IDL validation"));
        }
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

/// Return ownership is explicit in the source graph, including both weak families.
fn result(graph: &Generation<'_>, module: &ResolvedModule, output: &AbiType) -> Result<AbiReturn, GenerateError> {
    if matches!(output, AbiType::Unit) {
        return Ok(AbiReturn {
            rust_ty: syn::parse_quote!(()),
            abi_ty: syn::parse_quote!(()),
            kind: AbiReturnKind::Void,
        });
    }
    let ownership = match output {
        AbiType::ComPtr(target) => Some((target, InterfaceOwnership::Shared)),
        AbiType::ComBox(target) => Some((target, InterfaceOwnership::Exclusive)),
        AbiType::WeakComBox(target) => Some((target, InterfaceOwnership::WeakExclusive)),
        AbiType::WeakComPtr(target) => Some((target, InterfaceOwnership::WeakShared)),
        _ => None,
    };
    if let Some((target, ownership)) = ownership {
        let (interface, mode_ty) = object(graph, module, target)?;
        let abi_ty = match ownership {
            InterfaceOwnership::WeakShared => syn::parse_quote!(::neocogi_com::NcWeakRaw),
            _ => syn::parse_quote!(::neocogi_com::ComRaw<#interface>),
        };
        return Ok(AbiReturn {
            rust_ty: rust_type(owned_object_type(graph, module, output)?)?,
            abi_ty,
            kind: AbiReturnKind::ComPtr {
                ownership,
                interface_ty: Box::new(interface),
                mode_ty,
            },
        });
    }
    let ty = rust_type(value_type(graph, module, output)?)?;
    Ok(AbiReturn {
        rust_ty: ty.clone(),
        abi_ty: syn::parse_quote!(<#ty as ::neocogi_com::ComValue>::Wire),
        kind: AbiReturnKind::Plain,
    })
}
