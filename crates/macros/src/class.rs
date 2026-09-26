//! Concrete IDL class storage and delegation of its one declared event stream.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Ident, ItemStruct, Path, Token,
    parse::{Parse, ParseStream},
};

/// The class contract and optional explicitly owned producer field.
pub(crate) struct ClassArgs {
    /// IDL-generated concrete class trait; its ancestry owns the projection set.
    contract: Path,
    /// Field containing the generated class EventPort alias.
    event: Option<Ident>,
}

impl Parse for ClassArgs {
    /// Accepts `Class` or `Class, event = events`; no duplicate interface lists.
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let contract = Path::parse_mod_style(input)?;
        let event = if input.is_empty() {
            None
        } else {
            input.parse::<Token![,]>()?;
            let key: Ident = input.parse()?;
            if key != "event" {
                return Err(syn::Error::new(key.span(), "expected event = field"));
            }
            input.parse::<Token![=]>()?;
            Some(input.parse()?)
        };
        if !input.is_empty() {
            return Err(input.error("unexpected class option"));
        }
        Ok(Self { contract, event })
    }
}

/// Builds ordinary COM storage and one attach implementation delegating to owned state.
pub(crate) fn expand(args: ClassArgs, input: ItemStruct) -> syn::Result<TokenStream> {
    let contract = &args.contract;
    let mut abi = contract.clone();
    abi.segments.last_mut().unwrap().ident = quote::format_ident!("{}Abi", contract.segments.last().unwrap().ident);
    let delegate = match args.event {
        Some(field) => {
            if !input.fields.iter().any(|value| value.ident.as_ref() == Some(&field)) {
                return Err(syn::Error::new(field.span(), "event storage must name a field on the class"));
            }
            let mut stream = contract.clone();
            stream.segments.last_mut().unwrap().ident = quote::format_ident!("{}Stream", contract.segments.last().unwrap().ident);
            let mut sink = contract.clone();
            sink.segments.last_mut().unwrap().ident = quote::format_ident!("{}SinkAbi", contract.segments.last().unwrap().ident);
            let ident = &input.ident;
            let (implementation, types, clause) = input.generics.split_for_impl();
            quote! {
                impl #implementation #stream for #ident #types #clause {
                    /// Reserves this object's sole consumer slot for its behavioral parent.
                    fn attach(&mut self, consumer: ::neocogi_com::ComRef<'_, #sink>) -> Result<(), ::neocogi_com::NcStatus> {
                        self.#field.attach(consumer)
                    }
                }
            }
        }
        None => quote! {},
    };
    let storage = crate::object::expand_com_object(syn::parse2(quote! { #contract })?, input, false);
    Ok(quote! {
        #storage
        const _: fn() = || {
            // Reject an ordinary interface or factory used as a concrete class contract.
            fn assert_class<T: ::neocogi_com::ComClassInterface>() {}
            assert_class::<#abi>();
        };
        #delegate
    })
}
