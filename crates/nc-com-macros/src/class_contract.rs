//! Expansion of shared callable class contracts.
//!
//! A class contract owns the primary callable surface of one registered class.
//! Its method lowering deliberately reuses the interface generator because both
//! surfaces have the same safe two-pointer header and C vtable representation.
//! The generated class marker adds registry identity, avoiding a user-declared
//! one-off interface with the same methods.

use quote::{format_ident, quote};
use syn::{ItemTrait, Result};

use crate::args::{ClassContractArgs, InterfaceArgs};
use crate::interface::expand_com_interface;
use crate::util::{runtime_crate_path, stable_guid, validate_version_literal};

/// Generates one primary class projection and its registry identity metadata.
pub(crate) fn expand_com_class_contract(
    args: ClassContractArgs,
    input: ItemTrait,
) -> Result<proc_macro2::TokenStream> {
    let nc = runtime_crate_path(args.runtime_crate.as_ref());
    let class_abi_ident = format_ident!("{}Abi", input.ident);
    let class_id = stable_guid(args.class_id.clone(), &args.name, &nc)?;
    validate_version_literal(&args.version)?;

    // Primary class calls and optional interface calls intentionally share one
    // lowering path. That keeps ownership, parameter checking, method hashes,
    // and safe wrappers identical without duplicating the vtable generator.
    let callable = expand_com_interface(
        InterfaceArgs {
            guid: args.class_id,
            qualified_name: args.name.clone(),
            runtime_crate: args.runtime_crate,
        },
        input,
    )?;
    let qualified_name = args.name;
    let version = args.version;

    Ok(quote! {
        #callable

        // SAFETY: The primary projection above is generated from this exact
        // class identity and method contract. A `contract(...)`
        // implementation automatically publishes the same projection and ID.
        unsafe impl #nc::ComClassContract for #class_abi_ident {
            const CLASS_ID: #nc::NcGuid = #class_id;
            const QUALIFIED_NAME: &'static str = #qualified_name;
            const VERSION: &'static str = #version;
        }

        impl #class_abi_ident {
            /// Stable registry identity of this callable class contract.
            pub const CLASS_ID: #nc::NcGuid =
                <Self as #nc::ComClassContract>::CLASS_ID;

            /// Hash of the complete primary class method surface.
            pub const CLASS_ABI_HASH: #nc::NcAbiHash =
                <Self as #nc::ComClassContract>::CLASS_ABI_HASH;

            /// Normative class name stored in plugin and runtime metadata.
            pub const CLASS_QUALIFIED_NAME: &'static str =
                <Self as #nc::ComClassContract>::QUALIFIED_NAME;

            /// Static class contract version stored in runtime metadata.
            pub const CLASS_VERSION: &'static str =
                <Self as #nc::ComClassContract>::VERSION;
        }
    })
}
