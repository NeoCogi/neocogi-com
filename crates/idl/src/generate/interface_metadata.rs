//! Lifetime-bearing local interface facts emitted from the bound declaration graph.

use super::*;
use crate::{DeclarationIdentity, codegen::model::ParentProjection};
use std::collections::HashMap;

/// Emits a required digest borrow directly, without reading a physical descriptor.
pub(super) fn identity(nc: &syn::Path, value: &DeclarationIdentity) -> TokenStream {
    let iid = guid_literal(nc, value.id);
    let signature = signature_literal(value.abi_signature);
    quote! {
        #nc::idl_metadata::InterfaceIdentity {
            iid: #iid,
            abi_signature: #signature,
        }
    }
}

/// One row in the resolved derived-first projection order.
pub(super) struct Projection<'a> {
    /// Exact identity used by local inspection metadata.
    pub(super) identity: &'a DeclarationIdentity,
    /// Supplying direct parent and position. The first row belongs to the interface itself.
    pub(super) parent: Option<ParentProjection>,
}

/// Collects the same distinct order for physical tables and local inspection.
/// Each parent's own order determines its indices, even for rows a prior parent
/// already supplied. Heap frames preserve the bound ancestry without recursive
/// Rust syntax or a retained transitive graph.
pub(super) fn ancestry<'a>(graph: &Generation<'a>, module: &'a ResolvedModule, declaration: &'a Declaration) -> Result<Vec<Projection<'a>>, GenerateError> {
    let DeclarationKind::Interface(interface) = &declaration.kind else {
        return Err(GenerateError::new("interface ancestry target is not an interface"));
    };
    let identity = module
        .declaration(&declaration.name)
        .ok_or_else(|| GenerateError::new("bound interface lost its identity"))?;
    let mut seen = HashMap::from([(identity.id, identity)]);
    let mut projections = vec![Projection { identity, parent: None }];
    let mut parent_seen = HashMap::new();
    let mut bases = Vec::new();
    for (direct, base) in interface.bases.iter().enumerate() {
        parent_seen.clear();
        let mut current = Some(resolve(graph, module, base)?);
        while let Some(value) = current {
            let DeclarationKind::Interface(interface) = &value.declaration.kind else {
                return Err(GenerateError::new("interface ancestry target is not an interface"));
            };
            let entry = parent_seen.len();
            if record_identity(&mut parent_seen, value.identity)? {
                if record_identity(&mut seen, value.identity)? {
                    projections.push(Projection {
                        identity: value.identity,
                        parent: Some(ParentProjection { direct, entry }),
                    });
                }
                bases.push((value.module, interface.bases.iter()));
            }
            // Repeating an exact ancestor does not need another descent frame.
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
    }
    Ok(projections)
}

/// Reports a new IID or verifies its full digest before skipping a repeated row.
fn record_identity<'a>(seen: &mut HashMap<neocogi_com::NcGuid, &'a DeclarationIdentity>, identity: &'a DeclarationIdentity) -> Result<bool, GenerateError> {
    use std::collections::hash_map::Entry;
    match seen.entry(identity.id) {
        Entry::Vacant(entry) => {
            entry.insert(identity);
            Ok(true)
        }
        Entry::Occupied(entry) if entry.get().abi_signature == identity.abi_signature => Ok(false),
        Entry::Occupied(_) => Err(GenerateError::new(format!(
            "conflicting inherited interface contracts for {}",
            identity.qualified_name
        ))),
    }
}

/// Lists scoped getter declarers reachable only through physical first-base edges.
/// Queryable secondary branches are excluded at every level of the walk.
pub(super) fn scoped_prefixes(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration) -> Result<Vec<neocogi_com::NcGuid>, GenerateError> {
    let DeclarationKind::Interface(interface) = &declaration.kind else {
        return Err(GenerateError::new("scoped prefix target is not an interface"));
    };
    let mut prefixes = Vec::new();
    let mut next = interface.bases.first().map(|base| resolve(graph, module, base)).transpose()?;
    while let Some(parent) = next {
        let DeclarationKind::Interface(interface) = &parent.declaration.kind else {
            return Err(GenerateError::new("scoped prefix target is not an interface"));
        };
        if interface
            .methods
            .iter()
            .any(|method| method.return_passing == crate::model::ReturnPassing::ScopedLoan)
        {
            prefixes.push(parent.identity.id);
        }
        next = interface.bases.first().map(|base| resolve(graph, parent.module, base)).transpose()?;
    }
    Ok(prefixes)
}
