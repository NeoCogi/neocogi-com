//! Generation of format-neutral, statically typed decoding implementations.
//!
//! The generated code constructs concrete Rust values field by field. It does
//! not inspect ABI offsets, query an object registry, or create an erased value
//! graph. Generated support types and constructors use absolute core paths so
//! authored declarations such as `Option` cannot capture them. COM attribute macros call the helpers in this module directly; the
//! derive entry point exists for ordinary local types that are not ABI values.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{Attribute, Data, DeriveInput, Fields, Ident, LitStr, Path, Result, Type};

use crate::util::{push_error, validate_no_generics};

/// Parsed behavior requested by the `ComDeserialize` derive helper attribute.
#[derive(Default)]
pub(crate) struct DeriveOptions {
    /// Retain `Default` values for omitted named fields.
    pub(crate) default: bool,
    /// Decode struct fields positionally from a sequence.
    pub(crate) sequence: bool,
    /// Rename enum variants using the supported source naming convention.
    pub(crate) rename_all: Option<LitStr>,
}

/// Expands `#[derive(ComDeserialize)]` for an ordinary struct or fieldless enum.
pub(crate) fn expand_derive(input: DeriveInput) -> Result<TokenStream> {
    validate_no_generics(&input.generics, "ComDeserialize derives")?;
    let options = parse_options(&input.attrs)?;
    let ident = &input.ident;
    let nc: Path = syn::parse_quote!(::neocogi_com);

    match &input.data {
        Data::Struct(data) => expand_struct(ident, &data.fields, &nc, options.default, options.sequence),
        Data::Enum(data) => {
            if options.default || options.sequence {
                return Err(syn::Error::new(
                    ident.span(),
                    "default and sequence are only valid when deriving ComDeserialize for a struct",
                ));
            }
            if data.variants.iter().all(|variant| matches!(variant.fields, Fields::Unit)) {
                expand_unit_enum(ident, data.variants.iter().map(|variant| &variant.ident), &nc, options.rename_all.as_ref())
            } else {
                expand_enum(ident, data, &nc, options.rename_all.as_ref())
            }
        }
        Data::Union(data) => Err(syn::Error::new_spanned(
            data.union_token,
            "ComDeserialize cannot derive safe construction for a union",
        )),
    }
}

/// Generates decoding for a COM struct after its layout has been validated.
pub(crate) fn expand_struct(ident: &Ident, fields: &Fields, nc: &Path, default: bool, sequence: bool) -> Result<TokenStream> {
    if default && sequence {
        return Err(syn::Error::new(ident.span(), "default is only supported for named-record decoding"));
    }

    match fields {
        Fields::Named(fields) if !sequence => expand_named_record(ident, &fields.named, nc, default),
        Fields::Named(fields) => expand_positional_struct(
            ident,
            fields.named.iter().map(|field| {
                let member = field.ident.clone().expect("named field has an identifier");
                (member, &field.ty)
            }),
            nc,
            true,
        ),
        Fields::Unnamed(fields) if !default => expand_positional_struct(
            ident,
            fields
                .unnamed
                .iter()
                .enumerate()
                .map(|(index, field)| (Ident::new(&format!("__nc_value_{index}"), field.span()), &field.ty)),
            nc,
            false,
        ),
        Fields::Unnamed(_) => Err(syn::Error::new(ident.span(), "default requires a named struct")),
        Fields::Unit => Err(syn::Error::new(ident.span(), "ComDeserialize requires a named or tuple struct")),
    }
}

/// Generates decoding for a fieldless enum.
pub(crate) fn expand_unit_enum<'a>(ident: &Ident, variants: impl Iterator<Item = &'a Ident>, nc: &Path, rename_all: Option<&LitStr>) -> Result<TokenStream> {
    let convention = match rename_all {
        None => None,
        Some(value) if value.value() == "snake_case" => Some("snake_case"),
        Some(value) => {
            return Err(syn::Error::new(
                value.span(),
                "ComDeserialize currently supports only rename_all = \"snake_case\"",
            ));
        }
    };
    let variants = variants.collect::<Vec<_>>();
    let names = variants.iter().map(|variant| {
        let source = variant.to_string();
        let name = if convention == Some("snake_case") { to_snake_case(&source) } else { source };
        LitStr::new(&name, variant.span())
    });
    let arms = variants.iter().enumerate().map(|(index, variant)| {
        let index = index as u32;
        quote! {
            #index => {
                value.unit()?;
                ::core::result::Result::Ok(Self::#variant)
            }
        }
    });

    Ok(quote! {
        impl #nc::ComDeserialize for #ident {
            fn com_deserialize<'__nc_de, __NcDecoder>(decoder: __NcDecoder) -> ::core::result::Result<Self, __NcDecoder::Error>
            where
                __NcDecoder: #nc::ComDeserializer<'__nc_de>,
            {
                decoder.deserialize_enum::<Self>()
            }
        }

        impl #nc::ComEnum for #ident {
            const NAME: &'static str = stringify!(#ident);
            const VARIANTS: &'static [&'static str] = &[#(#names),*];

            fn read_variant<'__nc_de, __NcVariant>(variant: u32, value: __NcVariant) -> ::core::result::Result<Self, __NcVariant::Error>
            where
                __NcVariant: #nc::ComVariantAccess<'__nc_de>,
            {
                match variant {
                    #(#arms,)*
                    _ => ::core::result::Result::Err(value.invalid_variant(variant, Self::VARIANTS)),
                }
            }
        }
    })
}

/// Generates a strict or default-backed named record constructor.
fn expand_named_record(ident: &Ident, fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>, nc: &Path, default: bool) -> Result<TokenStream> {
    let members = fields
        .iter()
        .map(|field| field.ident.clone().expect("named field has an identifier"))
        .collect::<Vec<_>>();
    let types = fields.iter().map(|field| &field.ty).collect::<Vec<_>>();
    let names = members
        .iter()
        .map(|member| LitStr::new(member.to_string().trim_start_matches("r#"), member.span()))
        .collect::<Vec<_>>();
    let seen = members.iter().map(|member| format_ident!("__nc_seen_{member}")).collect::<Vec<_>>();
    let arms = members
        .iter()
        .zip(types.iter())
        .zip(names.iter())
        .zip(seen.iter())
        .enumerate()
        .map(|(index, (((member, ty), name), seen))| {
            let index = index as u32;
            if default {
                quote! {
                    #index => {
                        if #seen {
                            return ::core::result::Result::Err(record.duplicate_field(#name));
                        }
                        #seen = true;
                        value.#member = record.next_value::<#ty>()?;
                    }
                }
            } else {
                quote! {
                    #index => {
                        if #seen.is_some() {
                            return ::core::result::Result::Err(record.duplicate_field(#name));
                        }
                        #seen = ::core::option::Option::Some(record.next_value::<#ty>()?);
                    }
                }
            }
        });
    let initializers = if default {
        quote! {
            let mut value = Self::default();
            #(let mut #seen = false;)*
        }
    } else {
        quote! {
            #(let mut #seen: ::core::option::Option<#types> = ::core::option::Option::None;)*
        }
    };
    let construct = if default {
        quote! { ::core::result::Result::Ok(value) }
    } else {
        quote! {
            ::core::result::Result::Ok(Self {
                #(#members: #seen.ok_or_else(|| record.missing_field(#names))?,)*
            })
        }
    };

    Ok(quote! {
        impl #nc::ComDeserialize for #ident {
            fn com_deserialize<'__nc_de, __NcDecoder>(decoder: __NcDecoder) -> ::core::result::Result<Self, __NcDecoder::Error>
            where
                __NcDecoder: #nc::ComDeserializer<'__nc_de>,
            {
                decoder.deserialize_record::<Self>()
            }
        }

        impl #nc::ComRecord for #ident {
            const NAME: &'static str = stringify!(#ident);
            const FIELDS: &'static [&'static str] = &[#(#names),*];

            fn read_record<'__nc_de, __NcRecord>(record: &mut __NcRecord) -> ::core::result::Result<Self, __NcRecord::Error>
            where
                __NcRecord: #nc::ComRecordAccess<'__nc_de>,
            {
                #initializers
                while let ::core::option::Option::Some(field) = record.next_field(Self::FIELDS)? {
                    match field {
                        #(#arms,)*
                        _ => return ::core::result::Result::Err(record.invalid_field_index(field)),
                    }
                }
                #construct
            }
        }
    })
}

/// Generates positional construction for tuple structs and sequence records.
fn expand_positional_struct<'a>(ident: &Ident, fields: impl Iterator<Item = (Ident, &'a Type)>, nc: &Path, named: bool) -> Result<TokenStream> {
    let fields = fields.collect::<Vec<_>>();
    let length = fields.len();
    let bindings = fields.iter().enumerate().map(|(index, (_, ty))| {
        let binding = format_ident!("__nc_element_{index}");
        quote! {
            let #binding: #ty = sequence
                .next_element()?
                .ok_or_else(|| sequence.invalid_length(#index, #length))?;
        }
    });
    let values = (0..length).map(|index| format_ident!("__nc_element_{index}")).collect::<Vec<_>>();
    let construct = if named {
        let members = fields.iter().map(|(member, _)| member);
        quote! { Self { #(#members: #values),* } }
    } else {
        quote! { Self(#(#values),*) }
    };

    Ok(quote! {
        impl #nc::ComDeserialize for #ident {
            fn com_deserialize<'__nc_de, __NcDecoder>(decoder: __NcDecoder) -> ::core::result::Result<Self, __NcDecoder::Error>
            where
                __NcDecoder: #nc::ComDeserializer<'__nc_de>,
            {
                decoder.deserialize_sequence::<Self>()
            }
        }

        impl #nc::ComSequence for #ident {
            fn read_sequence<'__nc_de, __NcSequence>(sequence: &mut __NcSequence) -> ::core::result::Result<Self, __NcSequence::Error>
            where
                __NcSequence: #nc::ComSequenceAccess<'__nc_de>,
            {
                #(#bindings)*
                sequence.finish(#length)?;
                ::core::result::Result::Ok(#construct)
            }
        }
    })
}

/// Parses the derive's optional `#[com_deserialize(...)]` behavior.
pub(crate) fn parse_options(attrs: &[Attribute]) -> Result<DeriveOptions> {
    let mut options = DeriveOptions::default();
    let mut errors = None;
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("com_deserialize")) {
        if let Err(error) = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("default") {
                if options.default {
                    return Err(meta.error("duplicate default"));
                }
                options.default = true;
                Ok(())
            } else if meta.path.is_ident("sequence") {
                if options.sequence {
                    return Err(meta.error("duplicate sequence"));
                }
                options.sequence = true;
                Ok(())
            } else if meta.path.is_ident("rename_all") {
                if options.rename_all.is_some() {
                    return Err(meta.error("duplicate rename_all"));
                }
                options.rename_all = Some(meta.value()?.parse()?);
                Ok(())
            } else {
                Err(meta.error("expected default, sequence, or rename_all = \"snake_case\""))
            }
        }) {
            push_error(&mut errors, error);
        }
    }
    if options.default && options.sequence {
        push_error(
            &mut errors,
            syn::Error::new(proc_macro2::Span::call_site(), "default and sequence cannot be combined"),
        );
    }
    if let Some(errors) = errors { Err(errors) } else { Ok(options) }
}

/// Converts a Rust variant identifier to the JSON-style snake-case spelling.
pub(crate) fn to_snake_case(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    for (index, character) in source.chars().enumerate() {
        if character.is_uppercase() {
            if index != 0 {
                result.push('_');
            }
            result.extend(character.to_lowercase());
        } else {
            result.push(character);
        }
    }
    result
}

/// Constructs copied and owned native enums through one persistence protocol.
/// Variant ordinals follow declaration order, independently of COM wire tags.
/// Tuple/record helpers exist only inside the decoder and never become public
/// application representations or participate in a COM calling convention.
pub(crate) fn expand_enum(ident: &Ident, data: &syn::DataEnum, nc: &Path, rename: Option<&LitStr>) -> Result<TokenStream> {
    let mut helpers = Vec::new();
    let mut names = Vec::new();
    let mut arms = Vec::new();
    for (index, variant) in data.variants.iter().enumerate() {
        let index = index as u32;
        let variant_name = &variant.ident;
        let label = match rename {
            Some(rule) if rule.value() == "snake_case" => to_snake_case(&variant_name.to_string()),
            Some(rule) => return Err(syn::Error::new_spanned(rule, "only snake_case is supported")),
            None => variant_name.to_string(),
        };
        names.push(label);
        let body = match &variant.fields {
            Fields::Unit => quote! { value.unit()?; ::core::result::Result::Ok(Self::#variant_name) },
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => quote! { ::core::result::Result::Ok(Self::#variant_name(value.newtype()?)) },
            fields => {
                let helper = format_ident!("__Nc{}{}Decode", ident, variant_name);
                let members = fields
                    .iter()
                    .enumerate()
                    .map(|(i, field)| field.ident.clone().unwrap_or_else(|| format_ident!("field_{i}")))
                    .collect::<Vec<_>>();
                let types = fields.iter().map(|field| &field.ty).collect::<Vec<_>>();
                let definition: syn::ItemStruct = syn::parse_quote! {
                    /// Private typed fields acquired while decoding this one variant.
                    struct #helper {
                        #(/// Acquired field used to construct the selected native variant.
                        #members: #types),*
                    }
                };
                let sequence = matches!(fields, Fields::Unnamed(_));
                let decoder = expand_struct(&helper, &definition.fields, nc, false, sequence)?;
                helpers.push(quote! { #definition #decoder });
                match fields {
                    Fields::Named(_) => quote! { let payload: #helper = value.record()?; ::core::result::Result::Ok(Self::#variant_name { #(#members: payload.#members),* }) },
                    _ => {
                        let len = members.len();
                        quote! { let payload: #helper = value.tuple(#len)?; ::core::result::Result::Ok(Self::#variant_name(#(payload.#members),*)) }
                    }
                }
            }
        };
        arms.push(quote! { #index => { #body } });
    }
    Ok(quote! {
        impl #nc::ComDeserialize for #ident {
            /// Selects a native variant using the shared enum persistence protocol.
            fn com_deserialize<'__de, __D: #nc::ComDeserializer<'__de>>(decoder: __D) -> ::core::result::Result<Self, __D::Error> { decoder.deserialize_enum::<Self>() }
        }
        impl #nc::ComEnum for #ident {
            const NAME: &'static str = stringify!(#ident);
            const VARIANTS: &'static [&'static str] = &[#(#names),*];
            /// Acquires only the selected payload before constructing its native variant.
            fn read_variant<'__de, __V: #nc::ComVariantAccess<'__de>>(variant: u32, value: __V) -> ::core::result::Result<Self, __V::Error> {
                #(#helpers)*
                match variant { #(#arms,)* _ => ::core::result::Result::Err(value.invalid_variant(variant, Self::VARIANTS)) }
            }
        }
    })
}
