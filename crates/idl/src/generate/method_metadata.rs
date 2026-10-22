//! Ordinary callable introspection derived from bound interfaces, independent of UI policy.

use super::*;
use std::collections::HashSet;

/// Collects inherited methods in base-first order, preserving each supplying type scope.
pub(super) fn descriptions(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration) -> Result<Vec<TokenStream>, GenerateError> {
    let mut methods = Vec::new();
    collect(graph, module, declaration, &mut HashSet::new(), &mut methods)?;
    Ok(methods)
}

/// Visits each canonical ancestor once; distinct interfaces retain distinct methods.
fn collect(
    graph: &Generation<'_>,
    module: &ResolvedModule,
    declaration: &Declaration,
    visited: &mut HashSet<neocogi_com_core::NcGuid>,
    methods: &mut Vec<TokenStream>,
) -> Result<(), GenerateError> {
    let identity = module.declaration(&declaration.name).expect("resolved interface identity");
    if !visited.insert(identity.id) {
        return Ok(());
    }
    let DeclarationKind::Interface(interface) = &declaration.kind else {
        return Err(GenerateError::new("method metadata target is not an interface"));
    };
    for base in &interface.bases {
        let base = resolve(graph, module, base)?;
        collect(graph, base.module, base.declaration, visited, methods)?;
    }
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
                let ty = call_type(graph, module, &parameter.ty)?;
                Ok(quote! { ::neocogi_com::design::IdlParameter { name: #name, ty: #ty } })
            })
            .collect::<Result<Vec<_>, GenerateError>>()?;
        let returns = call_type(graph, module, &method.output)?;
        methods.push(quote! {
            ::neocogi_com::design::IdlMethod {
                name: #name, receiver: #receiver, parameters: &[#(#parameters),*], returns: #returns,
            }
        });
    }
    Ok(())
}

/// Retains call-only ownership instead of presenting borrowed/transferred values as data.
fn call_type(graph: &Generation<'_>, module: &ResolvedModule, ty: &AbiType) -> Result<TokenStream, GenerateError> {
    let (kind, value) = match ty {
        AbiType::ComBox(target) => {
            let AbiType::Named(name) = target.as_ref() else {
                return Err(GenerateError::new("exclusive method target is not an interface"));
            };
            let name = match builtin_object_type(name) {
                Some(_) => name.segments.join("."),
                None => resolve(graph, module, name)?.identity.qualified_name.clone(),
            };
            let name = Literal::string(&name);
            return Ok(quote! { ::neocogi_com::design::IdlMethodType::ExclusiveObject(#name) });
        }
        AbiType::Borrowed(value) => ("Borrowed", value.as_ref()),
        AbiType::Slice(value) | AbiType::UnsizedSlice(value) => ("InputBuffer", value.as_ref()),
        AbiType::OutputSlice(value) => ("OutputBuffer", value.as_ref()),
        _ => ("Value", ty),
    };
    let kind = rust_ident(kind)?;
    let value = design_field_type(graph, module, value)?;
    Ok(quote! { ::neocogi_com::design::IdlMethodType::#kind(#value) })
}
