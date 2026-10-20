//! Typed Rust emission shared by the IDL compiler and implementation macros.
//!
//! IDL owns semantic binding and canonical identities. This backend owns one
//! method/value emission model, including wire conversion, ownership, call gates
//! and generated-name scopes. Core never depends on this build-time package.
#![deny(missing_docs)]

mod args;
mod deserialize;
mod interface;
mod method_docs;
/// Typed ownership and wire plans supplied by the resolved IDL compiler.
pub mod model;
mod object;
mod plugin;
mod serialize;
mod util;
mod value;

pub use args::{InterfaceThreading, ObjectArgs, ObjectInterface, ObjectOwnership};
pub use plugin::{PluginArgs, expand_plugin};
pub use object::expand_com_object;
pub use interface::{InterfaceExpansion, expand_interface_parts};
pub use deserialize::{expand_derive as expand_com_deserialize_derive, expand_struct as emit_record_decoder, expand_enum as emit_enum_decoder};
pub use serialize::expand_derive as expand_com_serialize_derive;
pub use value::{ValueCase};

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
mod navigation_tests;
