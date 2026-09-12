//! Validation and expansion for fixed-layout plain COM structs.
//!
//! The generator owns the representation attribute, ensures fields contain only
//! durable ABI values, adds `Clone`/`Copy` when absent, and emits compiler-derived
//! size, alignment, offset, and field-type metadata for inspection.

use quote::{format_ident, quote};
use syn::{Attribute, Fields, Ident, ItemStruct, Member, Result};

use crate::args::{StructArgs, StructReprArgs};
use crate::deserialize::expand_struct as expand_struct_deserialize;
use crate::plain_data::{PlainAbiTypeContext, validate_plain_abi_fields};
use crate::util::{abi_signature_static_tokens, checked_u32_len, push_error, runtime_crate_path, validate_no_generics};

/// Applies layout policy and emits `ComStruct`, `AbiType`, and plain-value impls.
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
        expand_struct_deserialize(struct_ident, &input.fields, &nc, args.deserialize_default, args.deserialize_sequence)?
    } else {
        quote! {}
    };
    let field_layouts = com_struct_field_layouts(&input.fields);
    let signature_bytes_ident = format_ident!("__nc_{struct_ident}_abi_signature_bytes");
    let mut field_type_expressions = Vec::new();
    for (_, _, ty) in &field_layouts {
        let field_type = if args.c_struct {
            // An external C field has no separately generated declaration
            // identity. Associate its diagnostic aggregate type with the
            // enclosing IDL contract, whose precomputed digest is authoritative.
            let _ = ty;
            quote! { #nc::NcAbiType::Aggregate(#nc::NcAbiSignature::from_static(&#signature_bytes_ident)) }
        } else {
            quote! { #nc::__nc_abi_plain_type::<#ty>() }
        };
        field_type_expressions.push(field_type);
    }

    let field_layout_parts = field_layouts
        .iter()
        .zip(&field_type_expressions)
        .map(|((index, member, _), field_type)| {
            quote! {
                #nc::NcAbiFieldLayout {
                    index: #index,
                    offset: core::mem::offset_of!(#struct_ident, #member),
                    ty: #field_type,
                }
            }
        })
        .collect::<Vec<_>>();
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

    Ok(quote! {
        #input
        #layout_check
        #signature_static

        // SAFETY: `#[com_struct]` generates the representation and layout
        // metadata from compiler layout facts for this struct.
        unsafe impl #nc::ComStruct for #struct_ident {
            const ABI_SIGNATURE: #nc::NcAbiSignature =
                #nc::NcAbiSignature::from_static(&#signature_bytes_ident);

            const ABI_STRUCT_LAYOUT: #nc::NcAbiStructLayout = #nc::NcAbiStructLayout {
                size: core::mem::size_of::<Self>(),
                align: core::mem::align_of::<Self>(),
                fields: &[
                    #(#field_layout_parts,)*
                ],
            };
        }

        // SAFETY: `#[com_struct]` rejects generics and pointer/reference/owned
        // fields, while `ComStruct` retains their compiled physical layout as
        // typed metadata. With `c_struct`, the compiler checks C layout and the
        // author guarantees durable plain-value semantics.
        unsafe impl #nc::AbiType for #struct_ident {
            const ABI_TYPE: #nc::NcAbiType = #nc::__nc_abi_com_struct_type::<Self>();
        }
        // SAFETY: The struct is a fixed-layout aggregate of plain ABI values
        // with no drop glue or release protocol.
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

/// Pairs each field index with its `offset_of!` member and source type.
fn com_struct_field_layouts(fields: &Fields) -> Vec<(u32, Member, &syn::Type)> {
    match fields {
        Fields::Unit => Vec::new(),
        Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .enumerate()
            .map(|(index, field)| (index as u32, Member::Unnamed(syn::Index::from(index)), &field.ty))
            .collect(),
        Fields::Named(fields) => fields
            .named
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let ident = field.ident.clone().expect("named field has an ident");
                (index as u32, Member::Named(ident), &field.ty)
            })
            .collect(),
    }
}

/// Returns the `Clone`/`Copy` derives not already present on the source struct.
fn missing_copy_derives(attrs: &[Attribute]) -> Vec<Ident> {
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
