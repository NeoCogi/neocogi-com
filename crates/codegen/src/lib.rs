//! Typed Rust emission shared by the IDL compiler and implementation macros.
//!
//! IDL owns semantic binding and canonical identities. This backend owns one
//! method/value emission model, including wire conversion, ownership, call gates
//! and generated-name scopes. Core never depends on this build-time package.
#![deny(missing_docs)]

mod abi_method;
mod args;
mod deserialize;
mod enums;
mod interface;
/// Typed ownership and wire plans supplied by the resolved IDL compiler.
pub mod model;
mod object;
mod plain_data;
mod plugin;
mod serialize;
mod structs;
mod util;
mod value;

pub use args::{EnumArgs, InterfaceArgs, InterfaceGuid, InterfaceThreading, ObjectArgs, ObjectInterface, ObjectOwnership, StructArgs, StructReprArgs};
pub use plugin::{PluginArgs, expand_plugin};
pub use object::expand_com_object;
pub use interface::{InterfaceExpansion, expand_com_interface, expand_com_interface_with_methods, expand_interface_parts};
pub use structs::expand_com_struct;
pub use enums::expand_com_enum;
pub use deserialize::{expand_derive as expand_com_deserialize_derive, expand_struct as emit_record_decoder, expand_enum as emit_enum_decoder};
pub use serialize::expand_derive as expand_com_serialize_derive;
pub use value::{ValueCase, expand_owned};

/// Derives a provider trait declaration from the same typed plan as its ABI.
/// Ownership-bearing errors return original exclusive inputs in parameter order.
pub fn method_declaration(method: &model::AbiMethod) -> proc_macro2::TokenStream {
    use quote::quote;
    let name = &method.ident;
    let docs = &method.doc_attrs;
    let receiver = match method.receiver {
        model::AbiReceiver::Shared => quote! { &self },
        model::AbiReceiver::Mutable => quote! { &mut self },
    };
    let parameters = method.args.iter().map(|arg| {
        let name = &arg.ident;
        let ty = &arg.rust_ty;
        quote! { #name: #ty }
    });
    let output = &method.ok.rust_ty;
    let owners = method
        .args
        .iter()
        .filter_map(|arg| match arg.kind {
            model::AbiArgKind::ComBox { .. } => Some(&arg.rust_ty),
            _ => None,
        })
        .collect::<Vec<_>>();
    let error = if owners.is_empty() {
        quote! { ::neocogi_com::NcStatus }
    } else {
        quote! { (::neocogi_com::NcStatus, #(#owners),*) }
    };
    quote! { #(#docs)* fn #name(#receiver, #(#parameters),*) -> ::core::result::Result<#output, #error>; }
}

/// Emits a checked codec from already classified value alternatives.
/// `copied` requires self-contained wires; `nominal_fields` also requires their
/// declaration metadata. Native application layout is never a wire substitute.
pub fn emit_value(
    name: &syn::Ident,
    vis: &syn::Visibility,
    cases: &[ValueCase],
    signature: proc_macro2::TokenStream,
    copied: bool,
    nominal_fields: bool,
    nc: &syn::Path,
) -> proc_macro2::TokenStream {
    value::expand(name, vis, cases, signature, copied, nominal_fields, nc)
}

#[cfg(test)]
mod identifier_hygiene_tests;
#[cfg(test)]
mod method_name_tests;
#[cfg(test)]
mod name_ownership_tests;
#[cfg(test)]
mod navigation_tests;
#[cfg(test)]
mod persistence_codegen;
#[cfg(test)]
mod support_resolution_tests;
#[cfg(test)]
mod test_consumer;
