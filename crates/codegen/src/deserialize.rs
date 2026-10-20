//! Generates direct, declaration-ordered NCB construction for concrete values.
//!
//! Named records and tuple payloads have the same counted binary representation.
//! Only named records support omitted trailing fields through an explicit Default
//! policy. Enums select a declaration ordinal and acquire only its typed payload;
//! no format-adapter traits, arbitrary field dispatch or helper payload types remain.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Fields, Ident, Path, Result};

use crate::util::{PrivateNames, application_fields, push_error, validate_no_generics};

/// The sole decoding policy beyond the declared value shape.
#[derive(Default)]
pub(crate) struct DeriveOptions {
    /// Retains Default values only for an absent suffix of a named record.
    pub(crate) default: bool,
}

/// Expands a local value's NCB constructor; live ABI layout is handled separately.
pub fn expand_derive(input: DeriveInput) -> Result<TokenStream> {
    validate_no_generics(&input.generics, "ComDeserialize derives")?;
    let options = parse_options(&input.attrs)?;
    let nc: Path = syn::parse_quote!(::neocogi_com);
    match &input.data {
        Data::Struct(record) => expand_struct(&input.ident, &record.fields, &nc, options.default),
        Data::Enum(enumeration) if !options.default => expand_enum(&input.ident, enumeration, &nc),
        Data::Enum(_) => Err(syn::Error::new_spanned(&input.ident, "default requires a named struct")),
        Data::Union(data) => Err(syn::Error::new_spanned(
            data.union_token,
            "ComDeserialize cannot derive safe construction for a union",
        )),
    }
}

/// Wraps one ordered constructor in the single concrete persistence trait.
fn implementation(
    ident: &Ident,
    nc: &Path,
    reader: &Ident,
    names: &PrivateNames,
    native_fields: &Ident,
    owners: &[TokenStream],
    body: TokenStream,
) -> TokenStream {
    let field_scope = (!owners.is_empty()).then(|| {
        quote! {
            /// Private native-field resolution preserves Self and field visibility.
            trait #native_fields<const INDEX: usize> {
                /// Authored native field type, with application Self and visibility.
                type Value;
            }
        }
    });
    let binding_imports = names.binding_imports();
    quote! {
        const _: () = {
        #field_scope
        #(#owners)*
        const _: () = {
        #binding_imports
        impl #nc::ComDeserialize for #ident {
            /// Acquires this value's declared NCB fields under the shared document budget.
            fn com_deserialize(#reader: &mut #nc::binary::Reader<'_>) -> ::core::result::Result<Self, #nc::binary::Error> {
                #body
            }
        }
        };
        };
    }
}

/// Generates construction after COM value-layout validation, when used by an attribute macro.
pub fn expand_struct(ident: &Ident, fields: &Fields, nc: &Path, default: bool) -> Result<TokenStream> {
    let mut names = PrivateNames::new(quote! { #ident #fields #nc });
    let reader = names.binding("__nc_reader");
    let field_reader = names.binding("__nc_fields");
    let value = names.binding("__nc_value");
    let field_value = names.binding("__nc_field");
    let mut owners = Vec::new();
    let native_fields = names.fresh("__NcNativeFields");
    let fields = application_fields(ident, fields, &native_fields, &mut owners);
    let count = fields.len();
    let body = match &fields {
        Fields::Named(_) if default => {
            let assignments = fields.iter().map(|field| {
                let member = field.ident.as_ref().expect("named field");
                let ty = &field.ty;
                quote! {
                    if let ::core::option::Option::Some(#field_value) = #field_reader.next::<#ty>()? {
                        #value.#member = #field_value;
                    }
                }
            });
            quote! {
                #reader.record(#count, |#field_reader| {
                    // The persistence policy requires the trait's defaults,
                    // independently of any same-named inherent application method.
                    let mut #value = <Self as ::core::default::Default>::default();
                    #(#assignments)*
                    ::core::result::Result::Ok(#value)
                })
            }
        }
        Fields::Named(_) => {
            let construct = construction(&fields, quote!(Self), &field_reader);
            quote! { #reader.record(#count, |#field_reader| ::core::result::Result::Ok(#construct)) }
        }
        Fields::Unnamed(_) if !default => {
            let construct = construction(&fields, quote!(Self), &field_reader);
            quote! { #reader.sequence(|#field_reader| ::core::result::Result::Ok(#construct)) }
        }
        Fields::Unnamed(_) => return Err(syn::Error::new_spanned(ident, "default requires a named struct")),
        Fields::Unit => return Err(syn::Error::new_spanned(ident, "ComDeserialize requires a named or tuple struct")),
    };
    Ok(implementation(ident, nc, &reader, &names, &native_fields, &owners, body))
}

/// Builds fields directly in their destination; previously acquired fields drop on failure.
/// Authored member names occur only as member labels, never generated local bindings.
fn construction(fields: &Fields, destination: TokenStream, field_reader: &Ident) -> TokenStream {
    let missing = match fields {
        Fields::Named(_) => "missing field",
        _ => "invalid sequence length",
    };
    let values = fields.iter().map(|field| {
        let ty = &field.ty;
        let read = quote! { #field_reader.next::<#ty>()?.ok_or_else(|| #field_reader.error(#missing))? };
        match &field.ident {
            Some(member) => quote! { #member: #read },
            None => read,
        }
    });
    match fields {
        Fields::Named(_) => quote! { #destination { #(#values),* } },
        Fields::Unnamed(_) => quote! { #destination(#(#values),*) },
        Fields::Unit => destination,
    }
}

/// Generates one ordinal switch for unit, newtype, tuple and record cases.
/// Aggregate payloads enter one additional value scope, preserving NCB budgets;
/// newtypes read one child, and unit variants have no payload value at all.
pub fn expand_enum(ident: &Ident, data: &syn::DataEnum, nc: &Path) -> Result<TokenStream> {
    let variants = &data.variants;
    let mut names = PrivateNames::new(quote! { #ident #variants #nc });
    let reader = names.binding("__nc_reader");
    let field_reader = names.binding("__nc_fields");
    let mut owners = Vec::new();
    let native_fields = names.fresh("__NcNativeFields");
    let count = data.variants.len();
    let arms = data
        .variants
        .iter()
        .enumerate()
        .map(|(index, variant)| {
            let index = index as u32;
            let name = &variant.ident;
            let fields = application_fields(ident, &variant.fields, &native_fields, &mut owners);
            let count = fields.len();
            let construct = construction(&fields, quote!(Self::#name), &field_reader);
            let body = match &fields {
                Fields::Unit => quote! { ::core::result::Result::Ok(Self::#name) },
                Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    quote! { ::core::result::Result::Ok(Self::#name(#reader.read()?)) }
                }
                Fields::Unnamed(_) => quote! {
                    #reader.value(|#reader| #reader.sequence(|#field_reader| ::core::result::Result::Ok(#construct)))
                },
                Fields::Named(_) => quote! {
                    #reader.value(|#reader| #reader.record(#count, |#field_reader| ::core::result::Result::Ok(#construct)))
                },
            };
            quote! { #index => { #body } }
        })
        .collect::<Vec<_>>();
    Ok(implementation(
        ident,
        nc,
        &reader,
        &names,
        &native_fields,
        &owners,
        quote! {
            match #reader.variant(#count)? {
                #(#arms,)*
                _ => ::core::result::Result::Err(#reader.error("unknown enum variant")),
            }
        },
    ))
}

/// Parses explicit trailing-default policy and rejects retired format-adapter options.
/// Names and record/sequence selection do not affect NCB, so accepting them would
/// falsely promise a second schema mode to callers and the IDL compiler.
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
            } else {
                Err(meta.error("expected default; NCB fields are always positional and variants use ordinals"))
            }
        }) {
            push_error(&mut errors, error);
        }
    }
    match errors {
        Some(errors) => Err(errors),
        None => Ok(options),
    }
}
