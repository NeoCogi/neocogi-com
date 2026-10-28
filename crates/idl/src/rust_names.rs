//! Public Rust names shared by source admission and both export surfaces.

use neocogi_com_core::NcComInterfaceCompanion;
use crate::{Declaration, DeclarationKind};

/// Projects one final declaration's existing public Rust surface, authored name
/// first and then interface companions in core inventory order. Specialization
/// already removed open declarations/imports; native values have no companions.
/// This is naming policy only, not another declaration or compatibility owner.
pub(crate) fn declaration_names(declaration: &Declaration) -> impl Iterator<Item = String> + '_ {
    let companions = NcComInterfaceCompanion::ALL
        .into_iter()
        .filter(move |_| matches!(declaration.kind, DeclarationKind::Interface(_)))
        .map(move |companion| format!("{}{}", declaration.name, companion.suffix()));
    std::iter::once(declaration.name.clone()).chain(companions)
}
