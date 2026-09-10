//! Shared name, identity, visibility, and diagnostic helpers for macro expansion.
//!
//! These utilities select the canonical or explicitly supplied runtime path,
//! validate stable GUID and version literals, enforce ABI count widths, derive
//! collision-resistant generated identifiers, and combine independent `syn`
//! errors so callers see all actionable problems from one expansion.

use quote::{format_ident, quote};
use syn::{Generics, Ident, LitStr, Path, Result, Visibility};

use crate::args::InterfaceGuid;

/// Computes and emits the SHA-256 chaining state after the final-hash prefix.
///
/// Emitting these words avoids one interpreted compression block per generated
/// ABI fingerprint while leaving compiler-resolved values for const evaluation.
pub(crate) fn abi_hash_prefix_tokens(parts: &[neocogi_com_core::NcAbiHashPart<'_>], nc: &Path) -> proc_macro2::TokenStream {
    let schema = neocogi_com_core::NcAbiHash::__schema_from_abi_parts(parts);
    let prefix = neocogi_com_core::NcAbiHash::__prefix_from_schema(schema);
    let words = prefix.__words();
    quote! { #nc::__NcAbiHashPrefix::__from_words([#(#words),*]) }
}

/// Emits the native SHA-256 schema digest for one generated aggregate shape.
///
/// Field and variant values are deliberately placeholders: the schema encoder
/// consumes their ordered indices, while the generated const expression later
/// supplies sizes, offsets, tags, and nested ABI types.
pub(crate) fn aggregate_hash_prefix_tokens(
    kind: neocogi_com_core::NcAbiAggregateKind,
    field_count: usize,
    variant_count: usize,
    nc: &Path,
) -> proc_macro2::TokenStream {
    let mut parts = vec![
        neocogi_com_core::NcAbiHashPart::Aggregate { kind },
        neocogi_com_core::NcAbiHashPart::Layout { size: 0, align: 0 },
    ];
    parts.extend((0..field_count).map(|index| neocogi_com_core::NcAbiHashPart::Field {
        index: index as u32,
        offset: 0,
        ty: neocogi_com_core::NcAbiType::Void,
    }));
    parts.extend((0..variant_count).map(|index| neocogi_com_core::NcAbiHashPart::Variant {
        index: index as u32,
        tag: 0,
        ty: neocogi_com_core::NcAbiType::Void,
    }));
    abi_hash_prefix_tokens(&parts, nc)
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

#[cfg(test)]
/// Independent test-only SHA-256 implementation used for known-answer checks.
pub(crate) fn sha256(input: &[u8]) -> [u8; 32] {
    const H0: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];

    let bit_len = (input.len() as u64) * 8;
    let mut message = input.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    let mut h = H0;
    for chunk in message.chunks_exact(64) {
        let mut words = [0u32; 64];
        for (index, word) in words.iter_mut().take(16).enumerate() {
            let start = index * 4;
            *word = u32::from_be_bytes([chunk[start], chunk[start + 1], chunk[start + 2], chunk[start + 3]]);
        }
        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7) ^ words[index - 15].rotate_right(18) ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17) ^ words[index - 2].rotate_right(19) ^ (words[index - 2] >> 10);
            words[index] = words[index - 16].wrapping_add(s0).wrapping_add(words[index - 7]).wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[index]).wrapping_add(words[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (index, word) in h.iter().enumerate() {
        out[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
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
    let last = path.segments.last_mut().expect("class interface path has at least one segment");
    last.ident = ident;
    path
}

/// Derives a deterministic private storage-field name from a full interface path.
pub(crate) fn interface_field(path: &Path) -> Ident {
    let mut parts = Vec::new();
    if path.leading_colon.is_some() {
        parts.push("root".to_string());
    }
    parts.extend(path.segments.iter().map(|segment| segment.ident.to_string()));

    let mut name = String::from("__nc_interface");
    for part in parts {
        name.push('_');
        for ch in part.chars() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                name.push(ch);
            } else {
                name.push('_');
            }
        }
    }

    format_ident!("{name}")
}

/// Propagates the source class visibility to generated construction helpers.
pub(crate) fn create_visibility(vis: &Visibility) -> Visibility {
    // Constructors should be visible wherever the Rust class itself is visible.
    // Interface-generated ABI symbols use the interface trait visibility.
    vis.clone()
}
#[cfg(test)]
mod tests {
    use super::{checked_u32_len, sha256};

    #[test]
    fn sha256_matches_known_answer() {
        assert_eq!(
            sha256(b"abc"),
            [
                0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c,
                0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00, 0x15, 0xad,
            ]
        );
    }

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
