//! Concrete IDL class allocation without event installation or UI policy.
use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;
use crate::args::ObjectArgs;

/// Builds the ordinary COM allocation and checks its declared class role.
/// Uses the object parser so ownership has one syntax and validation policy.
pub(crate) fn expand(args: ObjectArgs, input: ItemStruct) -> syn::Result<TokenStream> {
    let abi = args.primary_interface.clone().ok_or_else(|| {
        syn::Error::new_spanned(&input.ident, "com_class requires a primary class contract")
    })?;
    let nc = crate::util::runtime_crate_path(args.runtime_crate.as_ref());
    let storage = crate::object::expand_com_object(args, input, false);
    Ok(quote! {
        #storage
        const _: fn() = || {
            // Ordinary interfaces and factory contracts cannot stand in for a class.
            fn assert_class<T: #nc::ComClassInterface>() {}
            assert_class::<#abi>();
        };
    })
}
