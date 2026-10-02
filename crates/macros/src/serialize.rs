//! Static serialization generation for explicitly selected persistent values.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, LitStr, Result};
use crate::util::validate_no_generics;

/// Generates field-by-field writes, never ABI-offset inspection or type erasure.
pub(crate) fn expand_derive(input: DeriveInput) -> Result<TokenStream> {
    validate_no_generics(&input.generics, "ComSerialize derives")?;
    let name = &input.ident;
    // Record labels come from the declaration, just like field and variant
    // labels. Named enum payloads retain the enclosing enum's record label.
    let record_name = LitStr::new(name.to_string().trim_start_matches("r#"), name.span());
    let nc: syn::Path = syn::parse_quote!(::neocogi_com);
    let options = crate::deserialize::parse_options(&input.attrs)?;
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
            fields(&data.fields, &values, &record_name, options.sequence, &nc)
        }
        Data::Enum(data) => {
            if options.sequence || options.default {
                return Err(syn::Error::new_spanned(name, "record policies cannot apply to an enum"));
            }
            let mut arms = Vec::new();
            for (index, variant) in data.variants.iter().enumerate() {
                let index = index as u32;
                let ident = &variant.ident;
                let label = match &options.rename_all {
                    Some(rule) if rule.value() == "snake_case" => crate::deserialize::to_snake_case(&ident.to_string()),
                    Some(rule) => return Err(syn::Error::new_spanned(rule, "only snake_case is supported")),
                    None => ident.to_string(),
                };
                let label = LitStr::new(&label, ident.span());
                let bindings = variant.fields.iter().enumerate().map(|(i, _)| format_ident!("__field_{i}")).collect::<Vec<_>>();
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
                        quote! { #nc::ComSerialize::com_serialize(#field, serializer)?; }
                    }
                    _ => fields(&variant.fields, &values, &record_name, false, &nc),
                };
                arms.push(quote! { #pattern => {
                    serializer.begin_variant(stringify!(#name), #index, #label)?;
                    #payload
                    serializer.end_variant()?;
                } });
            }
            quote! { match self { #(#arms),* } }
        }
        Data::Union(_) => return Err(syn::Error::new_spanned(name, "unions require an explicit persistent representation")),
    };
    Ok(quote! {
        impl #nc::ComSerialize for #name {
            /// Emits the declared persistent members without native memory layout.
            fn com_serialize<__S: #nc::ComSerializer>(&self, serializer: &mut __S) -> ::core::result::Result<(), __S::Error> {
                #body
                ::core::result::Result::Ok(())
            }
        }
    })
}

/// Emits record or tuple members through their statically known serializers.
/// Named records use the enclosing declaration's unqualified literal label.
fn fields(fields: &Fields, values: &[TokenStream], record_name: &LitStr, sequence: bool, nc: &syn::Path) -> TokenStream {
    let len = values.len();
    match fields {
        Fields::Unit => quote! { serializer.serialize_unit()?; },
        Fields::Named(named) if !sequence => {
            let names = named
                .named
                .iter()
                .map(|field| field.ident.as_ref().unwrap().to_string().trim_start_matches("r#").to_owned());
            quote! {
                serializer.begin_record(#record_name, #len)?;
                #(serializer.field(#names)?; #nc::ComSerialize::com_serialize(#values, serializer)?;)*
                serializer.end_record()?;
            }
        }
        _ => quote! {
            serializer.begin_sequence(#len)?;
            #(#nc::ComSerialize::com_serialize(#values, serializer)?;)*
            serializer.end_sequence()?;
        },
    }
}
