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

/// Collects derived-first query order with lazy, borrowed base iterators.
/// Full signatures are compared before IID deduplication. Heap frames retain
/// active nominal ancestry; only bounded emitted syntax uses native recursion.
pub(super) fn ancestry(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration) -> Result<Vec<TokenStream>, GenerateError> {
    if !matches!(declaration.kind, DeclarationKind::Interface(_)) {
        return Err(GenerateError::new("interface ancestry target is not an interface"));
    }
    let root = ResolvedDeclaration {
        module,
        declaration,
        identity: module
            .declaration(&declaration.name)
            .ok_or_else(|| GenerateError::new("bound interface lost its identity"))?,
    };
    let mut seen = HashMap::new();
    let mut chain = Vec::new();
    let mut current = Some(root);
    let mut bases = Vec::new();
    while let Some(value) = current {
        let DeclarationKind::Interface(interface) = &value.declaration.kind else {
            return Err(GenerateError::new("interface ancestry target is not an interface"));
        };
        if let Some(previous) = seen.get(&value.identity.id) {
            if *previous != value.identity.abi_signature {
                return Err(GenerateError::new(format!(
                    "conflicting inherited interface contracts for {}",
                    value.identity.qualified_name
                )));
            }
        } else {
            seen.insert(value.identity.id, value.identity.abi_signature);
            chain.push(identity(value.identity));
            bases.push((value.module, interface.bases.iter()));
        }
        // Resolve only the next visited edge, after the previous subtree ends.
        // A seen diamond ancestor adds no frame and leaves pending siblings intact.
        current = loop {
            let Some((module, remaining)) = bases.last_mut() else {
                break None;
            };
            if let Some(base) = remaining.next() {
                break Some(resolve(graph, module, base)?);
            }
            bases.pop();
        };
    }
    Ok(chain)
}
