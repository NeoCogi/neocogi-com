//! Validation and expansion for fixed-layout plain COM structs.
//!
//! The generator owns the representation attribute, ensures fields contain only
//! durable ABI values, adds `Clone`/`Copy` when absent, and publishes the assigned
//! nominal identity. The checked value codec owns wire layout and conversion;
//! no application-memory offset table is generated.

use quote::{format_ident, quote};
use syn::{Attribute, Fields, Ident, ItemStruct, Result};

use crate::args::{StructArgs, StructReprArgs};
use crate::deserialize::expand_struct as expand_struct_deserialize;
use crate::plain_data::{PlainAbiTypeContext, validate_plain_abi_fields};
use crate::util::{abi_signature_static_tokens, checked_u32_len, push_error, runtime_crate_path, validate_no_generics};

/// Applies local representation policy and emits nominal identity and checked codecs.
pub(crate) fn expand_com_struct(args: StructArgs, mut input: ItemStruct) -> Result<proc_macro2::TokenStream> {
    let nc = runtime_crate_path(args.runtime_crate.as_ref());
    validate_com_struct(&input)?;
    checked_u32_len(fields_len(&input.fields), input.ident.span(), "COM struct field count")?;
    let assigned_signature = args.assigned_signature.as_ref().ok_or_else(|| {
        syn::Error::new(
            input.ident.span(),
            "com_struct requires an IDL-generator assigned_signature; define cross-component contracts in .ncidl",
        )
    })?;

    let repr_attr = com_struct_repr_attr(&args.repr);
    input.attrs.push(repr_attr);

    let missing_derives = missing_copy_derives(&input.attrs);
    if !missing_derives.is_empty() {
        let derive_attr: Attribute = syn::parse_quote!(#[derive(#(#missing_derives),*)]);
        input.attrs.push(derive_attr);
    }

    let struct_ident = &input.ident;
    let deserialize_impl = if args.deserialize {
        expand_struct_deserialize(struct_ident, &input.fields, &nc, args.deserialize_default)?
    } else {
        quote! {}
    };
    let signature_bytes_ident = format_ident!("__nc_{struct_ident}_abi_signature_bytes", span = struct_ident.span());
    // Preserve the ordinary field identity/ownership requirement independently
    // of reflection. External C fields supply their own checked codecs instead.
    let field_checks = (!args.c_struct).then(|| {
        let types = input.fields.iter().map(|field| &field.ty);
        quote! {
            const _: () = {
                /// Requires durable copied identity for every ordinary record field.
                fn require<T: #nc::AbiPlainValue>() {}
                /// Type-checks fields without introducing a runtime call or table.
                fn fields() { #(require::<#types>();)* }
            };
        }
    });
    let signature_static = abi_signature_static_tokens(&signature_bytes_ident, assigned_signature, &nc);

    let layout_check = args.c_struct.then(|| {
        quote! {
            const _: extern "C" fn(#struct_ident) = {
                #[deny(improper_ctypes_definitions)]
                extern "C" fn check(_: #struct_ident) {}
                check
            };
        }
    });

    let value_codec = crate::value::expand(
        struct_ident,
        &input.vis,
        &[crate::value::ValueCase {
            variant: None,
            tag: quote! { 0 },
            fields: input.fields.clone(),
        }],
        quote! { #nc::NcAbiSignature::from_static(&#signature_bytes_ident) },
        true,
        &nc,
    );
    Ok(quote! {
        #input
        #value_codec
        #layout_check
        #signature_static

        #field_checks
        // SAFETY: IDL assigns the exact declaration identity; the field checks
        // and generated codec establish copied value and wire requirements.
        unsafe impl #nc::AbiType for #struct_ident {
            const ABI_TYPE: #nc::NcAbiType =
                #nc::NcAbiType::Aggregate(#nc::NcAbiSignature::from_static(&#signature_bytes_ident));
        }
        // SAFETY: Fields are copied and have no release protocol. Ordinary fields
        // also have nominal identities; external C fields supply checked codecs.
        // The generated ComInlineValue codec handles their foreign layout;
        // nested native enums must never be reinterpreted as wire storage.
        unsafe impl #nc::AbiPlainValue for #struct_ident {}

        #deserialize_impl
    })
}

/// Returns the physical field count for unit, tuple, or named struct syntax.
fn fields_len(fields: &Fields) -> usize {
    match fields {
        Fields::Unit => 0,
        Fields::Unnamed(fields) => fields.unnamed.len(),
        Fields::Named(fields) => fields.named.len(),
    }
}

/// Rejects generics, unit structs, user `repr`, and Rust-only field types.
fn validate_com_struct(input: &ItemStruct) -> Result<()> {
    let mut errors = None;

    if let Err(err) = validate_no_generics(&input.generics, "COM structs") {
        push_error(&mut errors, err);
    }
    if matches!(input.fields, Fields::Unit) {
        push_error(
            &mut errors,
            syn::Error::new_spanned(&input.ident, "COM structs must have named or tuple fields"),
        );
    }
    for attr in input.attrs.iter().filter(|attr| attr.path().is_ident("repr")) {
        push_error(
            &mut errors,
            syn::Error::new_spanned(
                attr,
                "COM struct layout is controlled by #[com_struct], #[com_struct(packed)], #[com_struct(packed(N))], or #[com_struct(align(N))]; remove #[repr(...)]",
            ),
        );
    }

    validate_plain_abi_fields(&input.fields, PlainAbiTypeContext::StructField, &mut errors);

    if let Some(errors) = errors { Err(errors) } else { Ok(()) }
}

/// Converts parsed layout options into the single generated `repr` attribute.
fn com_struct_repr_attr(repr: &StructReprArgs) -> Attribute {
    match repr {
        StructReprArgs::C => syn::parse_quote!(#[repr(C)]),
        StructReprArgs::Packed(None) => syn::parse_quote!(#[repr(C, packed)]),
        StructReprArgs::Packed(Some(value)) => syn::parse_quote!(#[repr(C, packed(#value))]),
        StructReprArgs::Align(value) => syn::parse_quote!(#[repr(C, align(#value))]),
    }
}

/// Returns the `Clone`/`Copy` derives not already present on the source struct.
pub(crate) fn missing_copy_derives(attrs: &[Attribute]) -> Vec<Ident> {
    let mut missing = Vec::new();
    if !has_derive(attrs, "Clone") {
        missing.push(format_ident!("Clone"));
    }
    if !has_derive(attrs, "Copy") {
        missing.push(format_ident!("Copy"));
    }
    missing
}

/// Searches all derive attributes for one exact trait name.
fn has_derive(attrs: &[Attribute], derive_name: &str) -> bool {
    attrs.iter().filter(|attr| attr.path().is_ident("derive")).any(|attr| {
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(derive_name) {
                found = true;
            }
            Ok(())
        });
        found
    })
}
