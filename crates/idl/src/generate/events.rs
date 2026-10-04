//! Reflects emitted COM interfaces and their actual getter results from the resolved IDL graph.
use super::*;
use std::collections::BTreeSet;

/// Emits event contracts in authored order without native payload associations or helper names.
pub(super) fn descriptions(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration) -> Result<Vec<TokenStream>, GenerateError> {
    let DeclarationKind::Interface(interface) = &declaration.kind else {
        return Ok(Vec::new());
    };
    interface
        .events
        .iter()
        .map(|name| {
            let event = resolve(graph, module, name)?;
            let abi = named_rust_type(graph, module, event, true)?;
            let qualified = Literal::string(&event.identity.qualified_name);
            let mut fields = Vec::new();
            getters(graph, event.module, event.declaration, &mut BTreeSet::new(), &mut fields)?;
            Ok(quote! { ::neocogi_com::design::IdlEvent {
                qualified_name: #qualified, abi: &#abi::INTERFACE_DESCRIPTOR, fields: &[#(#fields),*],
            } })
        })
        .collect()
}

/// Traverses inherited getter declarations once; each result keeps its original module scope.
fn getters(
    graph: &Generation<'_>,
    module: &ResolvedModule,
    declaration: &Declaration,
    visited: &mut BTreeSet<String>,
    fields: &mut Vec<TokenStream>,
) -> Result<(), GenerateError> {
    if !visited.insert(module.source.name.qualify(&declaration.name)) {
        return Ok(());
    }
    let DeclarationKind::Interface(interface) = &declaration.kind else {
        unreachable!("validated event interface")
    };
    for base in &interface.bases {
        let base = resolve(graph, module, base)?;
        getters(graph, base.module, base.declaration, visited, fields)?;
    }
    for method in &interface.methods {
        let name = Literal::string(&method.name);
        let ty = design_field_type(graph, module, &method.output)?;
        fields.push(quote! { ::neocogi_com::design::IdlField { name: #name, ty: #ty } });
    }
    Ok(())
}
