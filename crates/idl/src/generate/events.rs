//! Reflects emitted COM interfaces and their actual getter results from the resolved IDL graph.
use super::*;
use neocogi_com_core::NcGuid;
use std::collections::HashSet;

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
            getters(graph, event, &mut HashSet::new(), &mut fields)?;
            Ok(quote! { ::neocogi_com::design::IdlEvent {
                qualified_name: #qualified, abi: &#abi::INTERFACE_DESCRIPTOR, fields: &[#(#fields),*],
            } })
        })
        .collect()
}

/// Visits each canonical ancestor once, retaining its local declaration for type lookup.
/// Independently closed generic projections share an IID despite different Rust locations.
/// The resolved graph and generated COM ancestry enforce their compatible contracts.
fn getters(graph: &Generation<'_>, event: ResolvedDeclaration<'_>, visited: &mut HashSet<NcGuid>, fields: &mut Vec<TokenStream>) -> Result<(), GenerateError> {
    // Match event validation's semantic identity rather than counting paths or
    // consumer-local generic declarations. Traversal order still follows bases.
    if !visited.insert(event.identity.id) {
        return Ok(());
    }
    let module = event.module;
    let declaration = event.declaration;
    let DeclarationKind::Interface(interface) = &declaration.kind else {
        unreachable!("validated event interface")
    };
    for base in &interface.bases {
        let base = resolve(graph, module, base)?;
        getters(graph, base, visited, fields)?;
    }
    for method in &interface.methods {
        let name = Literal::string(&method.name);
        let ty = design_field_type(graph, module, &method.output)?;
        fields.push(quote! { ::neocogi_com::design::IdlField { name: #name, ty: #ty } });
    }
    Ok(())
}
