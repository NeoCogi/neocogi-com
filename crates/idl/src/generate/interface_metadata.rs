//! Lifetime-bearing local interface facts emitted from the bound declaration graph.

use super::*;
use crate::DeclarationIdentity;
use std::collections::HashMap;

/// Emits a required digest borrow directly, without reading a physical descriptor.
pub(super) fn identity(value: &DeclarationIdentity) -> TokenStream {
    let iid = guid_literal(value.id);
    let signature = signature_literal(value.abi_signature);
    quote! {
        ::neocogi_com::idl_metadata::InterfaceIdentity {
            iid: #iid,
            abi_signature: #signature,
        }
    }
}

/// Collects the derived-first query order, independent of base-first method order.
/// The output vector preserves declaration order; the map only checks identities.
pub(super) fn ancestry(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration) -> Result<Vec<TokenStream>, GenerateError> {
    let mut seen = HashMap::new();
    let mut chain = Vec::new();
    collect(graph, module, declaration, &mut seen, &mut chain)?;
    Ok(chain)
}

/// Visits bound bases in their supplying scope and checks signatures before dedup.
/// The implicit IUnknown object projection is not part of this interface graph.
fn collect(
    graph: &Generation<'_>,
    module: &ResolvedModule,
    declaration: &Declaration,
    seen: &mut HashMap<neocogi_com_core::NcGuid, [u8; 32]>,
    chain: &mut Vec<TokenStream>,
) -> Result<(), GenerateError> {
    let DeclarationKind::Interface(interface) = &declaration.kind else {
        return Err(GenerateError::new("interface ancestry target is not an interface"));
    };
    let value = module
        .declaration(&declaration.name)
        .ok_or_else(|| GenerateError::new("bound interface lost its identity"))?;
    if let Some(previous) = seen.get(&value.id) {
        if *previous != value.abi_signature {
            return Err(GenerateError::new(format!(
                "conflicting inherited interface contracts for {}",
                value.qualified_name
            )));
        }
        return Ok(());
    }
    seen.insert(value.id, value.abi_signature);
    chain.push(identity(value));
    for base in &interface.bases {
        let bound = resolve(graph, module, base)?;
        collect(graph, bound.module, bound.declaration, seen, chain)?;
    }
    Ok(())
}
