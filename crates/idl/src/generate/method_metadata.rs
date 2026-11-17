//! Ordinary callable introspection derived from bound interfaces, independent of UI policy.

use super::*;
use std::collections::HashSet;

/// Collects methods in base-first DFS completion order, with IID entry dedup.
/// Each borrowed frame retains its supplying declaration and next base index;
/// emitting after that frame completes preserves diamond and sibling ordering.
pub(super) fn descriptions(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration) -> Result<Vec<TokenStream>, GenerateError> {
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
                Receiver::Shared => quote! { ::neocogi_com::NcAbiReceiver::Shared },
                Receiver::Mutable => quote! { ::neocogi_com::NcAbiReceiver::Mutable },
            };
            let parameters = method
                .parameters
                .iter()
                .map(|parameter| {
                    let name = Literal::string(&parameter.name);
                    let ty = call_type(graph, value.module, &parameter.ty)?;
                    Ok(quote! { ::neocogi_com::idl_metadata::ParameterMetadata { name: #name, ty: #ty } })
                })
                .collect::<Result<Vec<_>, GenerateError>>()?;
            let returns = call_type(graph, value.module, &method.output)?;
            methods.push(quote! {
                ::neocogi_com::idl_metadata::MethodMetadata {
                    name: #name, receiver: #receiver, parameters: &[#(#parameters),*], returns: #returns,
                }
            });
        }
    }
    Ok(methods)
}

/// Retains call-only ownership instead of presenting borrowed/transferred values as data.
fn call_type(graph: &Generation<'_>, module: &ResolvedModule, ty: &IdlType) -> Result<TokenStream, GenerateError> {
    let (kind, value) = match ty {
        IdlType::ComBox(target) => {
            let IdlType::Named(name) = target.as_ref() else {
                return Err(GenerateError::new("exclusive method target is not an interface"));
            };
            let name = match builtin_object_type(name) {
                Some(_) => name.segments.join("."),
                None => resolve(graph, module, name)?.identity.qualified_name.clone(),
            };
            let name = Literal::string(&name);
            return Ok(quote! { ::neocogi_com::idl_metadata::MethodType::ExclusiveObject(#name) });
        }
        IdlType::Borrowed(value) => ("Borrowed", value.as_ref()),
        IdlType::Slice(value) | IdlType::UnsizedSlice(value) => ("InputBuffer", value.as_ref()),
        IdlType::OutputSlice(value) => ("OutputBuffer", value.as_ref()),
        _ => ("Value", ty),
    };
    let kind = rust_ident(kind)?;
    let value = value_type_metadata(graph, module, value)?;
    Ok(quote! { ::neocogi_com::idl_metadata::MethodType::#kind(#value) })
}
