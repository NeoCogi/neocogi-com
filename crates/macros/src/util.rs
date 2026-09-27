//! Shared name, identity, visibility, and diagnostic helpers for macro expansion.
//!
//! These utilities select the canonical or explicitly supplied runtime path,
//! validate stable GUID and version literals, enforce ABI count widths, derive
//! collision-resistant generated identifiers, and combine independent `syn`
//! errors so callers see all actionable problems from one expansion.

use quote::quote;
use syn::{Generics, Ident, LitByteStr, LitStr, Path, Result, Visibility};

use crate::args::InterfaceGuid;

/// Emits exact static storage for a generator-computed ABI signature.
///
/// This helper deliberately accepts bytes rather than logical ABI records.
/// Contract resolution and SHA-256 belong to the IDL build tool; the compiler
/// only type-checks and stores the resulting fixed-size array.
pub(crate) fn abi_signature_static_tokens(bytes_ident: &Ident, assigned_signature: &LitByteStr, nc: &Path) -> proc_macro2::TokenStream {
    quote! {
        /// Exact signature bytes assigned by the IDL compiler.
        #[allow(non_upper_case_globals)]
        static #bytes_ident: [u8; #nc::NcAbiSignature::SHA256_LEN] = *#assigned_signature;
    }
}

/// Uses an explicit facade path or the canonical public `neocogi-com` crate name.
pub(crate) fn runtime_crate_path(override_path: Option<&Path>) -> Path {
    override_path.cloned().unwrap_or_else(|| syn::parse_quote!(::neocogi_com))
}

/// Resolves and validates the stable GUID for an interface declaration.
pub(crate) fn interface_guid(attr: InterfaceGuid, qualified_name: &syn::LitStr, nc: &Path) -> Result<proc_macro2::TokenStream> {
    stable_guid(attr, qualified_name, nc)
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
        InterfaceGuid::Path(path) => Ok(quote! {{
            const PROVIDED: #nc::NcGuid = #path;
            const DERIVED: #nc::NcGuid = #nc::guid_from_qualified_name(#qualified_name);
            const _: () = assert!(
                PROVIDED.data1 == DERIVED.data1
                    && PROVIDED.data2 == DERIVED.data2
                    && PROVIDED.data3 == DERIVED.data3
                    && PROVIDED.data4[0] == DERIVED.data4[0]
                    && PROVIDED.data4[1] == DERIVED.data4[1]
                    && PROVIDED.data4[2] == DERIVED.data4[2]
                    && PROVIDED.data4[3] == DERIVED.data4[3]
                    && PROVIDED.data4[4] == DERIVED.data4[4]
                    && PROVIDED.data4[5] == DERIVED.data4[5]
                    && PROVIDED.data4[6] == DERIVED.data4[6]
                    && PROVIDED.data4[7] == DERIVED.data4[7],
                "GUID constant does not match the descriptor qualified name",
            );
            PROVIDED
        }}),
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

/// Propagates the source object visibility to generated construction helpers.
pub(crate) fn create_visibility(vis: &Visibility) -> Visibility {
    // Constructors should be visible wherever the Rust object itself is visible.
    // Interface-generated ABI symbols use the interface trait visibility.
    vis.clone()
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
