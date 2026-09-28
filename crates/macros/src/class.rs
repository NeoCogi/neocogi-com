//! Concrete IDL class allocation without event installation or UI policy.
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    ItemStruct, Path,
    parse::{Parse, ParseStream},
};

/// Exact generated class contract; its ancestry determines COM projections.
pub(crate) struct ClassArgs {
    /// Public class trait generated from the ordinary IDL declaration.
    contract: Path,
}
impl Parse for ClassArgs {
    /// Parses one class path. Constructor receivers are ordinary owned fields.
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let contract = Path::parse_mod_style(input)?;
        if !input.is_empty() {
            return Err(input.error("expected only the class contract; supply event receivers through construction"));
        }
        Ok(Self { contract })
    }
}
/// Builds the ordinary COM allocation and checks its declared class role.
pub(crate) fn expand(args: ClassArgs, input: ItemStruct) -> syn::Result<TokenStream> {
    let contract = &args.contract;
    let mut abi = contract.clone();
    abi.segments.last_mut().unwrap().ident = quote::format_ident!("{}Abi", contract.segments.last().unwrap().ident);
    let storage = crate::object::expand_com_object(syn::parse2(quote! { #contract })?, input, false);
    Ok(quote! {
        #storage
        const _: fn() = || {
            // Ordinary interfaces and factory contracts cannot stand in for a class.
            fn assert_class<T: ::neocogi_com::ComClassInterface>() {}
            assert_class::<#abi>();
        };
    })
}
