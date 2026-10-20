//! Shared name, identity, and diagnostic helpers for macro expansion.
//!
//! These utilities select the canonical or explicitly supplied runtime path,
//! validate stable GUID and version literals, enforce ABI count widths, derive
//! collision-resistant generated identifiers, and combine independent `syn`
//! errors so callers see all actionable problems from one expansion.

use std::collections::HashSet;
use quote::{ToTokens, quote};
use syn::{Generics, Ident, LitStr, Path, Result};

use crate::args::InterfaceGuid;

/// Creates a private local binding with macro-definition hygiene.
///
/// Authored members and types keep their original tokens. Callers must also use
/// separate positional bindings rather than source names: checked-in generated
/// Rust is printed and reparsed, which cannot retain a token's syntax context.
pub(crate) fn private_ident(name: &str) -> Ident {
    Ident::new(name, proc_macro2::Span::mixed_site())
}

/// Private names for one generated lexical scope, disjoint from authored syntax.
///
/// A fixed prefix or mixed-site span does not protect a type parameter after
/// printing. Collect every identifier that will enter the scope, including path
/// segments and raw identifiers, before allocating implementation-owned names.
/// Private item definitions must additionally live in an anonymous scope: this
/// allocator cannot inspect unrelated items surrounding a macro invocation.
pub(crate) struct PrivateNames {
    /// Authored and already allocated identifiers, normalized without `r#`.
    occupied: HashSet<String>,
}

impl PrivateNames {
    /// Seeds one scope from its complete authored input and configured paths.
    pub(crate) fn new(authored: impl ToTokens) -> Self {
        fn collect(tokens: proc_macro2::TokenStream, occupied: &mut HashSet<String>) {
            for token in tokens {
                match token {
                    proc_macro2::TokenTree::Ident(ident) => {
                        occupied.insert(ident.to_string().trim_start_matches("r#").to_owned());
                    }
                    proc_macro2::TokenTree::Group(group) => collect(group.stream(), occupied),
                    _ => {}
                }
            }
        }
        let mut occupied = HashSet::new();
        collect(authored.into_token_stream(), &mut occupied);
        Self { occupied }
    }

    /// Allocates a deterministic spelling absent from this scope's authored syntax.
    pub(crate) fn fresh(&mut self, stem: &str) -> Ident {
        let mut candidate = stem.to_owned();
        let mut ordinal = 0usize;
        while !self.occupied.insert(candidate.clone()) {
            candidate = format!("{stem}_{ordinal}");
            ordinal += 1;
        }
        private_ident(&candidate)
    }
}

/// Uses an explicit facade path or the canonical public `neocogi-com` crate name.
pub(crate) fn runtime_crate_path(override_path: Option<&Path>) -> Path {
    override_path.cloned().unwrap_or_else(|| syn::parse_quote!(::neocogi_com))
}

/// Validates a qualified name and emits its matching explicit, derived, or path GUID.
pub(crate) fn stable_guid(attr: InterfaceGuid, qualified_name: &syn::LitStr, nc: &Path) -> Result<proc_macro2::TokenStream> {
    let name = qualified_name.value();
    neocogi_com_core::validate_qualified_name(&name)
        .map_err(|error| syn::Error::new(qualified_name.span(), format!("invalid descriptor qualified name: {error}")))?;
    let expected = neocogi_com_core::guid_from_qualified_name(&name);
    match attr {
        InterfaceGuid::Explicit(value) => {
            let actual = value
                .value()
                .parse::<neocogi_com_core::NcGuid>()
                .map_err(|err| syn::Error::new(value.span(), err))?;
            if actual != expected {
                return Err(syn::Error::new(
                    value.span(),
                    format!("explicit UUID does not match qualified name `{name}`; expected {expected}"),
                ));
            }
            guid_tokens(actual, nc)
        }
        InterfaceGuid::AutoGuid { value } => {
            if value.value() != name {
                return Err(syn::Error::new(value.span(), "auto_guid must exactly match the descriptor qualified name"));
            }
            Ok(quote! { #nc::guid_from_qualified_name(#qualified_name) })
        }
        InterfaceGuid::Path(path) => {
            let mut names = PrivateNames::new(quote! { #path #nc });
            let provided = names.fresh("__NC_PROVIDED_GUID");
            let derived = names.fresh("__NC_DERIVED_GUID");
            Ok(quote! {{
                const #provided: #nc::NcGuid = #path;
                const #derived: #nc::NcGuid = #nc::guid_from_qualified_name(#qualified_name);
                const _: () = ::core::assert!(
                    #provided.data1 == #derived.data1
                        && #provided.data2 == #derived.data2
                        && #provided.data3 == #derived.data3
                        && #provided.data4[0] == #derived.data4[0]
                        && #provided.data4[1] == #derived.data4[1]
                        && #provided.data4[2] == #derived.data4[2]
                        && #provided.data4[3] == #derived.data4[3]
                        && #provided.data4[4] == #derived.data4[4]
                        && #provided.data4[5] == #derived.data4[5]
                        && #provided.data4[6] == #derived.data4[6]
                        && #provided.data4[7] == #derived.data4[7],
                    "GUID constant does not match the descriptor qualified name",
                );
                #provided
            }})
        }
        InterfaceGuid::Assigned(value) => {
            let actual = value
                .value()
                .parse::<neocogi_com_core::NcGuid>()
                .map_err(|err| syn::Error::new(value.span(), err))?;
            guid_tokens(actual, nc)
        }
    }
}

/// Applies core descriptor version policy at the literal's source span.
pub(crate) fn validate_version_literal(version: &LitStr) -> Result<()> {
    neocogi_com_core::validate_version_text(&version.value()).map_err(|error| syn::Error::new(version.span(), format!("invalid descriptor version: {error}")))
}

/// Converts a generated table length to the ABI's `u32` count domain.
pub(crate) fn checked_u32_len(len: usize, span: proc_macro2::Span, description: &str) -> Result<u32> {
    u32::try_from(len).map_err(|_| syn::Error::new(span, format!("{description} exceeds the ABI limit of {}", u32::MAX)))
}

/// Emits a field-wise GUID literal independent of host memory byte order.
fn guid_tokens(value: neocogi_com_core::NcGuid, nc: &Path) -> Result<proc_macro2::TokenStream> {
    let (data1, data2, data3, data4) = value.fields();
    let bytes = data4.iter().map(|byte| quote! { #byte });
    Ok(quote! {
        #nc::NcGuid {
            data1: #data1,
            data2: #data2,
            data3: #data3,
            data4: [#(#bytes),*],
        }
    })
}

/// Collects successful values while combining every `syn::Error` encountered.
pub(crate) fn collect_syn_results<T>(values: impl IntoIterator<Item = Result<T>>) -> Result<Vec<T>> {
    let mut items = Vec::new();
    let mut errors = None;

    for value in values {
        match value {
            Ok(item) => items.push(item),
            Err(err) => push_error(&mut errors, err),
        }
    }

    if let Some(errors) = errors { Err(errors) } else { Ok(items) }
}

/// Inserts or combines one error into an optional aggregate diagnostic.
pub(crate) fn push_error(errors: &mut Option<syn::Error>, err: syn::Error) {
    if let Some(errors) = errors {
        errors.combine(err);
    } else {
        *errors = Some(err);
    }
}

/// Rejects generic parameters and where clauses for fixed ABI declarations.
pub(crate) fn validate_no_generics(generics: &Generics, subject: &str) -> Result<()> {
    let mut errors = None;

    if !generics.params.is_empty() {
        push_error(&mut errors, syn::Error::new_spanned(generics, format!("{subject} cannot be generic")));
    }
    if let Some(where_clause) = &generics.where_clause {
        push_error(
            &mut errors,
            syn::Error::new_spanned(where_clause, format!("{subject} cannot have where clauses")),
        );
    }

    if let Some(errors) = errors { Err(errors) } else { Ok(()) }
}

/// Clones a path and replaces its final identifier while retaining qualification.
pub(crate) fn path_with_last_ident(path: &Path, ident: Ident) -> Path {
    let mut path = path.clone();
    let last = path.segments.last_mut().expect("object interface path has at least one segment");
    last.ident = ident;
    path
}

#[cfg(test)]
mod tests {
    use super::checked_u32_len;

    #[test]
    fn u32_lengths_are_checked_before_codegen() {
        assert_eq!(checked_u32_len(u32::MAX as usize, proc_macro2::Span::call_site(), "test").unwrap(), u32::MAX);

        #[cfg(target_pointer_width = "64")]
        assert_eq!(
            checked_u32_len(u32::MAX as usize + 1, proc_macro2::Span::call_site(), "test count",)
                .unwrap_err()
                .to_string(),
            "test count exceeds the ABI limit of 4294967295"
        );
    }
}
