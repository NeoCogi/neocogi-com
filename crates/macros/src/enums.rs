//! Native copied enums and their separate recursively checked COM wire codec.
//!
//! Application code matches the authored Rust enum. Only the explicit raw wire
//! record crosses COM; no public tag/union wrapper or conversion API is generated.
//! Nominal metadata identifies the wire contract without promising a Rust layout.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Fields, Ident, ItemEnum, Result};

use crate::args::EnumArgs;
use crate::plain_data::{PlainAbiTypeContext, validate_plain_abi_fields};
use crate::util::{abi_signature_static_tokens, checked_u32_len, push_error, runtime_crate_path, validate_no_generics};

/// Emits a native enum, its checked inline codec, and declaration identity.
/// Persistence and wire conversion use the same zero-based declaration ordinals.
/// Only the active recursively checked payload crosses the foreign boundary.
pub(crate) fn expand_com_enum(args: EnumArgs, mut input: ItemEnum) -> Result<proc_macro2::TokenStream> {
    let nc = runtime_crate_path(args.runtime_crate.as_ref());
    validate_com_enum(&input)?;
    let derives = crate::structs::missing_copy_derives(&input.attrs);
    if !derives.is_empty() {
        input.attrs.push(syn::parse_quote!(#[derive(#(#derives),*)]));
    }
    let name = &input.ident;
    let vis = &input.vis;
    let count = checked_u32_len(input.variants.len(), name.span(), "COM enum variant count")? as usize;
    let mut variants = Vec::new();
    for (index, variant) in input.variants.iter().enumerate() {
        checked_u32_len(variant.fields.len(), variant.ident.span(), "COM enum payload field count")?;
        let tag = crate::value::ordinal_tag(index, variant)?;
        variants.push((variant.ident.clone(), variant.fields.clone(), tag));
    }
    validate_com_enum_payloads(&variants)?;
    let signature = args.assigned_signature.as_ref().ok_or_else(|| {
        syn::Error::new(
            name.span(),
            "com_enum requires an IDL-generator assigned_signature; define cross-component contracts in .ncidl",
        )
    })?;
    let signature_name = format_ident!("__nc_{name}_abi_signature_bytes", span = name.span());
    let signature_static = abi_signature_static_tokens(&signature_name, signature, &nc);
    let cases = variants
        .iter()
        .map(|(variant, fields, tag)| crate::value::ValueCase {
            variant: Some(variant.clone()),
            fields: fields.clone(),
            tag: quote! { #tag },
        })
        .collect::<Vec<_>>();
    let codec = crate::value::expand(name, vis, &cases, quote! { #nc::NcAbiSignature::from_static(&#signature_name) }, true, &nc);
    let decoder = if args.deserialize {
        let data = syn::DataEnum {
            enum_token: input.enum_token,
            brace_token: input.brace_token,
            variants: input.variants.clone(),
        };
        crate::deserialize::expand_enum(name, &data, &nc)?
    } else {
        quote! {}
    };
    let tag_type = &args.tag;
    Ok(quote! {
        #input
        #signature_static
        #codec
        #decoder

        // A const assertion cannot be disabled by overflow-lint settings in the
        // consuming crate. Uniqueness follows directly from ordinal assignment.
        const _: () = assert!(
            #count == 0 || (#count - 1) as u128 <= <#tag_type>::MAX as u128,
            "enum variant ordinals exceed the declared tag representation",
        );
        // SAFETY: this is the IDL identity of the generated checked wire codec.
        // Native Rust enum layout is local and never used as the COM layout.
        unsafe impl #nc::AbiType for #name {
            const ABI_TYPE: #nc::NcAbiType = #nc::NcAbiType::Aggregate(<Self as #nc::ComValueType>::VALUE_SIGNATURE);
        }
        // SAFETY: the inline codec statically checks every selected field for
        // self-contained wire storage; the native enum owns no drop obligation.
        unsafe impl #nc::AbiPlainValue for #name {}
    })
}

/// Rejects generics; IDL supplies already specialized copied enum declarations.
fn validate_com_enum(input: &ItemEnum) -> Result<()> {
    let mut errors = None;

    if let Err(err) = validate_no_generics(&input.generics, "COM enums") {
        push_error(&mut errors, err);
    }

    if let Some(errors) = errors { Err(errors) } else { Ok(()) }
}

/// Validates every payload field as a durable plain ABI value.
fn validate_com_enum_payloads(variants: &[(Ident, Fields, TokenStream)]) -> Result<()> {
    let mut errors = None;

    for (_, fields, _) in variants {
        validate_plain_abi_fields(fields, PlainAbiTypeContext::EnumPayload, &mut errors);
    }

    if let Some(errors) = errors { Err(errors) } else { Ok(()) }
}
