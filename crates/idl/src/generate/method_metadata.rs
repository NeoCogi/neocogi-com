//! Ordinary callable introspection derived from bound interfaces, independent of UI policy.

use super::*;
use std::collections::HashSet;
use crate::model::{ParameterPassing, ReturnPassing};

/// Collects methods in base-first DFS completion order, with IID entry dedup.
/// Each borrowed frame retains its supplying declaration and next base index;
/// emitting after that frame completes preserves diamond and sibling ordering.
pub(super) fn descriptions(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration) -> Result<Vec<TokenStream>, GenerateError> {
    let nc = &graph.support;
    let root = ResolvedDeclaration {
        module,
        declaration,
        identity: module.declaration(&declaration.name).expect("resolved interface identity"),
    };
    let mut visited = HashSet::new();
    visited.insert(root.identity.id);
    let mut frames = vec![(root, 0usize)];
    let mut methods = Vec::new();
    while let Some((value, next_base)) = frames.last_mut() {
        let value = *value;
        let DeclarationKind::Interface(interface) = &value.declaration.kind else {
            return Err(GenerateError::new("method metadata target is not an interface"));
        };
        if let Some(base) = interface.bases.get(*next_base) {
            *next_base += 1;
            let base = resolve(graph, value.module, base)?;
            if visited.insert(base.identity.id) {
                frames.push((base, 0));
            }
            continue;
        }
        frames.pop();
        for method in &interface.methods {
            let name = Literal::string(&method.name);
            let receiver = match method.receiver {
                Receiver::Shared => quote! { #nc::NcAbiReceiver::Shared },
                Receiver::Mutable => quote! { #nc::NcAbiReceiver::Mutable },
            };
            let parameters = method
                .parameters
                .iter()
                .map(|parameter| {
                    let name = Literal::string(&parameter.name);
                    let ty = parameter_type(graph, value.module, parameter)?;
                    Ok(quote! { #nc::idl_metadata::ParameterMetadata { name: #name, ty: #ty } })
                })
                .collect::<Result<Vec<_>, GenerateError>>()?;
            let returns = return_type(graph, value.module, method)?;
            let projection = match method.return_projection {
                crate::ReturnProjection::Fallible => quote! { #nc::idl_metadata::ReturnProjection::Fallible },
                crate::ReturnProjection::Direct => quote! { #nc::idl_metadata::ReturnProjection::Direct },
            };
            methods.push(quote! {
                #nc::idl_metadata::MethodMetadata {
                    name: #name, receiver: #receiver, parameters: &[#(#parameters),*],
                    returns: #returns, return_projection: #projection,
                }
            });
        }
    }
    Ok(methods)
}

/// Describes each input using its resolved direction and payload.
fn parameter_type(graph: &Generation<'_>, module: &ResolvedModule, parameter: &crate::Parameter) -> Result<TokenStream, GenerateError> {
    let ty = parameter.payload();
    let kind = match parameter.passing {
        ParameterPassing::Copied => "Value",
        ParameterPassing::BorrowedValue | ParameterPassing::BorrowedText | ParameterPassing::SharedLoan { optional: false } => "Borrowed",
        ParameterPassing::SharedLoan { optional: true } => "OptionalBorrowedInterface",
        ParameterPassing::MutableLoan { optional: false } => "MutableBorrowedInterface",
        ParameterPassing::MutableLoan { optional: true } => "OptionalMutableBorrowedInterface",
        ParameterPassing::InputBuffer => "InputBuffer",
        ParameterPassing::OutputBuffer => "OutputBuffer",
        ParameterPassing::ConsumingBox => return exclusive_type(graph, module, ty),
        ParameterPassing::Unresolved | ParameterPassing::Invalid => return Err(GenerateError::new("input slot lacks its resolved passing rule")),
    };
    method_type(graph, module, kind, ty)
}

/// Describes output ownership from the same slot that selects the wire.
fn return_type(graph: &Generation<'_>, module: &ResolvedModule, method: &Method) -> Result<TokenStream, GenerateError> {
    let kind = match method.return_passing {
        ReturnPassing::Unit | ReturnPassing::Copied | ReturnPassing::SharedOwner { optional: false } | ReturnPassing::SharedObserver => "Value",
        ReturnPassing::SharedOwner { optional: true } => "OptionalOwnedInterface",
        ReturnPassing::BorrowedLoan { mutable: false, optional: false } => "BorrowedInterfaceResult",
        ReturnPassing::BorrowedLoan { mutable: false, optional: true } => "OptionalBorrowedInterfaceResult",
        ReturnPassing::BorrowedLoan { mutable: true, optional: false } => "MutableBorrowedInterfaceResult",
        ReturnPassing::BorrowedLoan { mutable: true, optional: true } => "OptionalMutableBorrowedInterfaceResult",
        ReturnPassing::ScopedLoan { mutable: false } => "ScopedBorrowedInterface",
        ReturnPassing::ScopedLoan { mutable: true } => "ScopedMutableBorrowedInterface",
        ReturnPassing::BoxOwner => return exclusive_type(graph, module, method.return_payload()),
        ReturnPassing::Unresolved | ReturnPassing::Invalid => return Err(GenerateError::new("result slot lacks its resolved passing rule")),
    };
    let ty = match method.return_passing {
        ReturnPassing::SharedObserver => &method.output,
        _ => method.return_payload(),
    };
    method_type(graph, module, kind, ty)
}

/// Packages structural payload metadata with the selected call-only passing rule.
fn method_type(graph: &Generation<'_>, module: &ResolvedModule, kind: &str, ty: &IdlType) -> Result<TokenStream, GenerateError> {
    let nc = &graph.support;
    let kind = rust_ident(kind)?;
    let value = value_type_metadata(graph, module, ty)?;
    Ok(quote! { #nc::idl_metadata::MethodType::#kind(#value) })
}

/// Records the declared interface of a consumed or produced exclusive owner.
fn exclusive_type(graph: &Generation<'_>, module: &ResolvedModule, target: &IdlType) -> Result<TokenStream, GenerateError> {
    let nc = &graph.support;
    let IdlType::Named(name) = target else {
        return Err(GenerateError::new("exclusive method target is not an interface"));
    };
    let name = match builtin_object_type(nc, name) {
        Some(_) => name.segments.join("."),
        None => resolve(graph, module, name)?.identity.qualified_name.clone(),
    };
    let name = Literal::string(&name);
    Ok(quote! { #nc::idl_metadata::MethodType::ExclusiveObject(#name) })
}
