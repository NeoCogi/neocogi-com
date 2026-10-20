//! Static serialization generation for explicitly selected persistent values.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Result};
use crate::util::{PrivateNames, validate_no_generics};

/// Generates field-by-field writes, never ABI-offset inspection or type erasure.
pub fn expand_derive(input: DeriveInput) -> Result<TokenStream> {
    validate_no_generics(&input.generics, "ComSerialize derives")?;
    // Deriving field borrows requires ordinary aligned Rust storage. Packed
    // application records need an explicit codec that first copies each chosen
    // field into aligned local storage; no byte-layout policy belongs to COM.
    for attribute in input.attrs.iter().filter(|attribute| attribute.path().is_ident("repr")) {
        let representations = attribute.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)?;
        if representations.iter().any(|representation| representation.path().is_ident("packed")) {
            return Err(syn::Error::new_spanned(
                attribute,
                "ComSerialize requires aligned fields; implement an explicit codec for a packed native record",
            ));
        }
    }
    let name = &input.ident;
    let nc: syn::Path = syn::parse_quote!(::neocogi_com);
    let options = crate::deserialize::parse_options(&input.attrs)?;
    let mut names = PrivateNames::new(quote! { #input #nc });
    let serializer = names.binding("__nc_serializer");
    let body = match &input.data {
        Data::Struct(data) => {
            let values = data
                .fields
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    let member = field
                        .ident
                        .clone()
                        .map(syn::Member::Named)
                        .unwrap_or_else(|| syn::Member::Unnamed(index.into()));
                    quote! { &self.#member }
                })
                .collect::<Vec<_>>();
            fields(&data.fields, &values, &nc, &serializer)
        }
        Data::Enum(data) => {
            if options.default {
                return Err(syn::Error::new_spanned(name, "record policies cannot apply to an enum"));
            }
            let mut arms = Vec::new();
            for (index, variant) in data.variants.iter().enumerate() {
                let index = index as u32;
                let ident = &variant.ident;
                let bindings = variant
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(i, _)| names.binding(&format!("__nc_field_{i}")))
                    .collect::<Vec<_>>();
                let pattern = match &variant.fields {
                    Fields::Unit => quote! { Self::#ident },
                    Fields::Unnamed(_) => quote! { Self::#ident(#(#bindings),*) },
                    Fields::Named(fields) => {
                        let names = fields.named.iter().map(|field| &field.ident);
                        quote! { Self::#ident { #(#names: #bindings),* } }
                    }
                };
                let values = bindings.iter().map(|field| quote! { #field }).collect::<Vec<_>>();
                let payload = match &variant.fields {
                    // A unit case has no payload value. The enum's discriminant
                    // already identifies it; emitting serialize_unit would add
                    // a logical child that the matching decoder never visits.
                    Fields::Unit => quote! {},
                    Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                        let field = &values[0];
                        quote! { #nc::ComSerialize::com_serialize(#field, #serializer)?; }
                    }
                    _ => fields(&variant.fields, &values, &nc, &serializer),
                };
                arms.push(quote! { #pattern => {
                    #serializer.begin_variant(#index)?;
                    #payload
                    #serializer.end_variant()?;
                } });
            }
            quote! { match self { #(#arms),* } }
        }
        Data::Union(_) => return Err(syn::Error::new_spanned(name, "unions require an explicit persistent representation")),
    };
    let binding_imports = names.binding_imports();
    Ok(quote! {
        const _: () = {
        #binding_imports
        impl #nc::ComSerialize for #name {
            /// Emits the declared persistent members without native memory layout.
            fn com_serialize(&self, #serializer: &mut #nc::binary::Writer<'_>) -> ::core::result::Result<(), #nc::binary::Error> {
                #body
                ::core::result::Result::Ok(())
            }
        }
        };
    })
}

/// Emits ordered record or tuple members through the same counted NCB representation.
fn fields(fields: &Fields, values: &[TokenStream], nc: &syn::Path, serializer: &syn::Ident) -> TokenStream {
    let len = values.len();
    match fields {
        Fields::Unit => quote! { #serializer.serialize_unit()?; },
        _ => quote! {
            #serializer.begin_sequence(#len)?;
            #(#nc::ComSerialize::com_serialize(#values, #serializer)?;)*
            #serializer.end_sequence()?;
        },
    }
}
