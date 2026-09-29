//! Native copied enums and their separate recursively checked COM wire codec.
//!
//! Application code matches the authored Rust enum. Only the explicit raw wire
//! record crosses COM; no public tag/union wrapper or conversion API is generated.
//! Nominal metadata identifies the wire contract without promising a Rust layout.

use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{Attribute, Expr, Fields, Ident, ItemEnum, Lit, LitInt, Result, UnOp};

use crate::args::EnumArgs;
use crate::plain_data::{PlainAbiTypeContext, validate_plain_abi_fields};
use crate::util::{abi_signature_static_tokens, checked_u32_len, push_error, runtime_crate_path, validate_no_generics};

/// Emits a native enum, its checked inline codec, and declaration identity.
/// Persistence and wire conversion share the native variants but preserve their
/// independent declaration-order ordinals and explicitly assigned wire tags.
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
    for (index, variant) in input.variants.iter_mut().enumerate() {
        checked_u32_len(variant.fields.len(), variant.ident.span(), "COM enum payload field count")?;
        let tag = variant_tag(&variant.attrs)?.unwrap_or_else(|| {
            let literal = LitInt::new(&(index + 1).to_string(), variant.ident.span());
            syn::parse_quote! { #literal }
        });
        variant.attrs = strip_com_enum_attrs(&variant.attrs);
        variants.push((variant.ident.clone(), variant.fields.clone(), tag));
    }
    validate_unique_literal_tags(&variants)?;
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
        crate::deserialize::expand_enum(name, &data, &nc, None)?
    } else {
        quote! {}
    };
    let tag_type = &args.tag;
    let tags = variants.iter().map(|(_, _, tag)| tag);
    Ok(quote! {
        #input
        #signature_static
        #codec
        #decoder

        // Keep source tag-range and duplicate checks even though no public tag
        // newtype is emitted. Constants are evaluated by Rust before any call.
        const _: () = {
            let tags: [#tag_type; #count] = [#(#tags),*];
            let mut index = 0;
            while index < tags.len() {
                let mut previous = 0;
                while previous < index {
                    if tags[index] == tags[previous] {
                        panic!("duplicate #[com_enum] tag value");
                    }
                    previous += 1;
                }
                index += 1;
            }
        };
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
fn validate_com_enum_payloads(variants: &[(Ident, Fields, Expr)]) -> Result<()> {
    let mut errors = None;

    for (_, fields, _) in variants {
        validate_plain_abi_fields(fields, PlainAbiTypeContext::EnumPayload, &mut errors);
    }

    if let Some(errors) = errors { Err(errors) } else { Ok(()) }
}

/// Diagnoses duplicate tags when both expressions can be evaluated syntactically.
fn validate_unique_literal_tags(variants: &[(Ident, Fields, Expr)]) -> Result<()> {
    let mut seen = Vec::new();

    for (variant_ident, _, tag_value) in variants {
        let Some(tag) = literal_tag_value(tag_value) else {
            continue;
        };
        if let Some((_, previous_variant)) = seen.iter().find(|(seen_tag, _)| *seen_tag == tag) {
            return Err(syn::Error::new(
                tag_value.span(),
                format!("duplicate COM enum tag value; `{previous_variant}` already uses tag {tag}"),
            ));
        }
        seen.push((tag, variant_ident));
    }

    Ok(())
}

/// Evaluates a supported signed or unsigned integer literal tag expression.
fn literal_tag_value(expr: &Expr) -> Option<i128> {
    match expr {
        Expr::Lit(expr) => literal_int_value(&expr.lit, false),
        Expr::Unary(expr) if matches!(expr.op, UnOp::Neg(_)) => {
            let Expr::Lit(lit) = expr.expr.as_ref() else {
                return None;
            };
            literal_int_value(&lit.lit, true)
        }
        _ => None,
    }
}

/// Converts one integer literal into the normalized signed comparison domain.
fn literal_int_value(lit: &Lit, negative: bool) -> Option<i128> {
    let Lit::Int(value) = lit else {
        return None;
    };
    let value = value.base10_parse::<i128>().ok()?;
    Some(if negative { -value } else { value })
}

/// Extracts at most one `#[com_tag(...)]` variant attribute.
fn variant_tag(attrs: &[Attribute]) -> Result<Option<Expr>> {
    let mut tag = None;
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("com_tag")) {
        if tag.is_some() {
            return Err(syn::Error::new_spanned(attr, "duplicate #[com_tag(...)] attribute"));
        }
        tag = Some(
            attr.parse_args::<Expr>()
                .map_err(|err| syn::Error::new(attr.span(), format!("expected #[com_tag(value)]: {err}")))?,
        );
    }
    Ok(tag)
}

/// Removes macro-only tag attributes before re-emitting the user's enum.
fn strip_com_enum_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs.iter().filter(|attr| !attr.path().is_ident("com_tag")).cloned().collect()
}
