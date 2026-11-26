//! Typed Rust emission shared by the IDL compiler and implementation macros.
//!
//! IDL owns semantic binding and canonical identities. This backend owns one
//! method/value emission model, including wire conversion, ownership, call gates
//! and generated-name scopes. It is also the ordinary-library expansion API
//! called by the thin `neocogi-com-macros` procedural entrypoints. Core never
//! depends on this build-time compiler. No runtime state is created by emission.

mod args;
mod deserialize;
mod host_main;
mod interface;
mod method_docs;
/// Typed ownership and wire plans supplied by the resolved IDL compiler.
pub mod model;
mod object;
mod plugin;
mod scalar;
mod serialize;
mod util;
mod value;

pub use args::{ObjectArgs, ObjectInterface, ObjectOwnership};
pub use plugin::{PluginArgs, expand_plugin};
pub use host_main::{MainArgs, expand_com_main};
pub use object::expand_com_object;
pub(crate) use interface::{expand_interface_parts_with_path, expand_scoped_interface_parts};
pub(crate) use util::{sdk_crate_path, threading_marker};
pub use interface::{InterfaceVTablePrefix, InterfaceExpansion, expand_interface_parts};
pub use deserialize::{expand_derive as expand_com_deserialize_derive, expand_struct as emit_record_decoder, expand_enum as emit_enum_decoder};
pub use serialize::expand_derive as expand_com_serialize_derive;
pub(crate) use value::ValueCase;
pub use scalar::{ScalarDomain, ScalarMember, ScalarPlan, emit_scalar};

/// Derives a provider trait declaration from the same typed plan as its ABI.
/// Ownership-bearing errors return original exclusive inputs in parameter order.
pub fn method_declaration(method: &model::AbiMethod) -> proc_macro2::TokenStream {
    method_declaration_with_path(method, &sdk_crate_path(None))
}

/// Emits the same provider declaration against the support selected by its assembly.
fn method_declaration_with_path(method: &model::AbiMethod, nc: &syn::Path) -> proc_macro2::TokenStream {
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
    if let model::AbiReturnKind::ScopedLoan { interface_ty, mode_ty, mutable } = &method.ok.kind {
        let parameters = parameters.collect::<Vec<_>>();
        let mut private = util::PrivateNames::new(quote! { #name #(#parameters)* #interface_ty #nc });
        let result = private.fresh("__NcVisitorResult");
        let lend = private.binding("__nc_lend");
        let lifetime = private.fresh("__nc_loan");
        let lifetime = syn::Lifetime::new(&format!("'{lifetime}"), lifetime.span());
        let handle = if *mutable {
            quote! { ComMut }
        } else {
            quote! { ComRef }
        };
        let mode = mode_ty.as_deref().map_or_else(|| quote! { #nc::ThreadLocal }, |mode| quote! { #mode });
        let loan = quote! { ::core::option::Option<#nc::#handle<#lifetime, #interface_ty, #mode>> };
        let delivery = match method.projection {
            crate::ReturnProjection::Direct => loan,
            crate::ReturnProjection::Fallible => quote! { ::core::result::Result<#loan, #nc::NcStatus> },
        };
        return quote! {
            #(#docs)*
            fn #name<#result>(
                #receiver, #(#parameters,)*
                #lend: impl for<#lifetime> ::core::ops::FnOnce(#delivery) -> #result,
            ) -> #result;
        };
    }
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
        quote! { #nc::NcStatus }
    } else {
        quote! { (#nc::NcStatus, #(#owners),*) }
    };
    let returns = match (method.projection, &method.ok.kind) {
        (crate::ReturnProjection::Fallible, _) => quote! { -> ::core::result::Result<#output, #error> },
        (crate::ReturnProjection::Direct, model::AbiReturnKind::Void) => quote! {},
        (crate::ReturnProjection::Direct, _) => quote! { -> #output },
    };
    quote! { #(#docs)* fn #name(#receiver, #(#parameters),*) #returns; }
}

/// Emits a checked codec from compiler-bound IDL value alternatives.
/// This internal entrypoint accepts no arbitrary native Rust field syntax.
/// `copied` requires self-contained wires; `nominal_fields` also requires their
/// declaration metadata. Native application layout is never a wire substitute.
pub(crate) fn emit_value(
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
