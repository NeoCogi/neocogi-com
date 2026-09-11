//! Expansion of constructor-free IDL class and factory descriptions.
//!
//! The generated primary projection reuses the interface ABI machinery, while
//! the marker implementation records identity, version, required interfaces,
//! and threading without publishing a runtime constructor.

use quote::{format_ident, quote};
use syn::{ItemTrait, PathArguments, Result, TraitBoundModifier, TypeParamBound};

use crate::abi_method::{parse_abi_trait_item, validate_class_contract_trait};
use crate::args::{DescriptionThreading, InterfaceArgs, InterfaceGuid, ObjectDescriptionArgs};
use crate::interface::{expand_com_interface_with_methods, validate_callable_methods};
use crate::util::{collect_syn_results, push_error, runtime_crate_path};

/// Semantic category attached to the generated description marker.
#[derive(Clone, Copy)]
pub(crate) enum ObjectDescriptionKind {
    /// An ordinary object shape created by a typed factory method.
    Class,
    /// A typed object provider, including a plugin's root factory.
    Factory,
}

/// Generates one primary projection and its constructor-free description.
pub(crate) fn expand_object_description(args: ObjectDescriptionArgs, input: ItemTrait, kind: ObjectDescriptionKind) -> Result<proc_macro2::TokenStream> {
    let nc = runtime_crate_path(args.runtime_crate.as_ref());
    let abi_ident = format_ident!("{}Abi", input.ident);
    let required_interface_abis = input
        .supertraits
        .iter()
        .skip(1)
        .map(|bound| match bound {
            TypeParamBound::Trait(bound)
                if bound.modifier == TraitBoundModifier::None
                    && bound.lifetimes.is_none()
                    && bound.path.segments.iter().all(|segment| matches!(segment.arguments, PathArguments::None)) =>
            {
                let mut abi = bound.path.clone();
                let last = abi.segments.last_mut().expect("validated trait paths have a final segment");
                last.ident = format_ident!("{}Abi", last.ident);
                Ok(abi)
            }
            _ => Err(syn::Error::new_spanned(bound, "expected a nongeneric generated COM interface")),
        })
        .collect::<Result<Vec<_>>>()?;

    let mut errors = None;
    validate_class_contract_trait(&input, &mut errors);
    let methods = match collect_syn_results(input.items.iter().map(parse_abi_trait_item)) {
        Ok(methods) => methods,
        Err(error) => {
            push_error(&mut errors, error);
            Vec::new()
        }
    };
    validate_callable_methods(&input, &methods, &mut errors);
    if let Some(errors) = errors {
        return Err(errors);
    }

    let id = args.id;
    let name = args.name;
    let version = args.version;
    let threading = match args.threading {
        DescriptionThreading::Local => quote! { #nc::ThreadLocal },
        DescriptionThreading::Shared => quote! { #nc::ThreadSafe },
    };
    let callable = expand_com_interface_with_methods(
        InterfaceArgs {
            guid: InterfaceGuid::Assigned(id),
            qualified_name: name.clone(),
            runtime_crate: args.runtime_crate,
        },
        input,
        methods,
    )?;
    let factory_marker = matches!(kind, ObjectDescriptionKind::Factory).then(|| {
        quote! {
            // SAFETY: This marker is emitted only for an IDL `factory`
            // declaration after semantic validation of the complete module.
            unsafe impl #nc::ComFactoryDescription for #abi_ident {}
        }
    });

    Ok(quote! {
        #callable

        // SAFETY: Every identity and required projection below comes from one
        // semantically resolved IDL declaration. The callable expansion above
        // generates the matching primary ABI header and interface chains.
        unsafe impl #nc::ComObjectDescription for #abi_ident {
            type Threading = #threading;

            const OBJECT_ID: #nc::NcGuid = <Self as #nc::ComInterface>::IID;
            const QUALIFIED_NAME: &'static str = #name;
            const VERSION: u32 = #version;
            const INTERFACE_CHAINS: &'static [&'static [&'static #nc::NcInterfaceDescriptor]] = &[
                &[&#nc::IUnknown::INTERFACE_DESCRIPTOR],
                &#abi_ident::INTERFACE_CHAIN,
                #(&#required_interface_abis::INTERFACE_CHAIN,)*
            ];
        }

        #factory_marker

        impl #abi_ident {
            /// Stable IDL identity of this class or factory description.
            pub const OBJECT_ID: #nc::NcGuid = <Self as #nc::ComObjectDescription>::OBJECT_ID;

            /// Exact dotted IDL name of this class or factory description.
            pub const OBJECT_QUALIFIED_NAME: &'static str = <Self as #nc::ComObjectDescription>::QUALIFIED_NAME;

            /// Positive IDL declaration version.
            pub const OBJECT_VERSION: u32 = <Self as #nc::ComObjectDescription>::VERSION;
        }
    })
}
