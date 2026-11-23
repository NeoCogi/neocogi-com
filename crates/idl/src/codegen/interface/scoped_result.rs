//! Scoped result emission from an already validated callable method.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Lifetime, Path, Type, ext::IdentExt};

use super::{AbiMethod, AbiReturnKind, PrivateNames, safe_arg_preparation, safe_vtable_call_args};
use crate::ReturnProjection;

/// Appends the same two native arguments to the vtable slot and its thunk.
pub(super) fn raw_parameters(nc: &Path, interface: &Type, context: &Ident, callback: &Ident) -> TokenStream {
    quote! {
        #context: *mut ::core::ffi::c_void,
        #callback: ::core::option::Option<#nc::NcScopedResultCallback<#interface>>,
    }
}

/// Rejects null receiver or callback before argument decoding and provider admission.
pub(super) fn pointer_check(nc: &Path, receiver: &Ident, callback: &Ident) -> TokenStream {
    quote! {
        if #receiver.is_null() {
            return #nc::NC_E_POINTER;
        }
        let ::core::option::Option::Some(#callback) = #callback else {
            return #nc::NC_E_POINTER;
        };
    }
}

/// Delivers the caller's pair or canonical absence while the receiver guard lives.
pub(super) fn success_arm(nc: &Path, value: &Ident, context: &Ident, callback: &Ident, raw: &Ident) -> TokenStream {
    quote! {
        ::core::result::Result::Ok(#value) => {
            let #raw = #value.map_or_else(#nc::ComRaw::null, |#value| #value.as_raw());
            // SAFETY: The caller supplies the callback and opaque context. The
            // provider's lender and the receiver guard cover its complete use.
            unsafe { #callback(#context, #raw) };
            #nc::NC_OK
        },
    }
}

/// Emits a shared-only scoped getter on the exact live interface marker.
pub(super) fn marker_method(nc: &Path, method: &AbiMethod, names: &mut PrivateNames) -> Option<TokenStream> {
    let AbiReturnKind::ScopedLoan { interface_ty, mode_ty } = &method.ok.kind else {
        return None;
    };
    let mode = match mode_ty {
        Some(mode) => quote! { #mode },
        None => quote! { #nc::ThreadLocal },
    };
    let method_ident = &method.ident;
    let helper = format_ident!("with_{}", method_ident.unraw(), span = method_ident.span());
    let source_mode = names.fresh("__NcScopedSourceThreading");
    let result = names.fresh("__NcScopedResult");
    let loan = names.fresh("__nc_scoped_loan");
    let loan = Lifetime::new(&format!("'{loan}"), loan.span());
    let source = names.binding("__nc_scoped_source");
    let visit = names.binding("__nc_scoped_visit");
    let context = names.binding("__nc_scoped_context");
    let callback = names.binding("__nc_scoped_callback");
    let pair = names.binding("__nc_scoped_pair");
    let arguments = method.args.iter().map(|argument| {
        let binding = &argument.binding;
        let ty = &argument.rust_ty;
        quote! { #binding: #ty }
    });
    let preparation = method.args.iter().filter_map(|argument| safe_arg_preparation(nc, argument)).collect::<Vec<_>>();
    let wire_arguments = method.args.iter().flat_map(safe_vtable_call_args);
    let docs = &method.doc_attrs;
    let checked = quote! {
        #(#preparation)*
        let #pair = #source.as_raw();
        // SAFETY: The exact live receiver and prepared inputs last through this
        // synchronous call. The raw slot admits the receiver and lends its result
        // through the installed continuation on the calling thread.
        unsafe {
            #nc::__nc_with_scoped_result::<#interface_ty, #mode, #result>(
                #visit,
                |#context, #callback| {
                    ((*#pair.vtable).#method_ident)(
                        #pair.object,
                        #(#wire_arguments,)*
                        #context,
                        ::core::option::Option::Some(#callback),
                    )
                },
            )
        }
    };
    let (return_ty, body, status_doc) = match method.projection {
        ReturnProjection::Direct => {
            // A fallible preparation needs a local Result scope so its error
            // is projected along with any error from native dispatch.
            let checked = if preparation.is_empty() {
                quote! { { #checked } }
            } else {
                quote! { (|| { #checked })() }
            };
            (
                quote! { ::core::option::Option<#result> },
                quote! { #checked.ok().flatten() },
                "Returns None when the getter has no value or the COM call fails.",
            )
        }
        ReturnProjection::Fallible => (
            quote! { ::core::result::Result<::core::option::Option<#result>, #nc::NcStatus> },
            checked,
            "Returns Ok(None) for successful absence and Err for a failed COM call.",
        ),
    };
    Some(quote! {
        #(#docs)*
        /// Runs the visitor once for a present interface loan from this live receiver.
        /// The receiver's shared admission protects the loan through the visitor.
        /// The visitor's output cannot carry that loan beyond the call.
        #[doc = #status_doc]
        #[allow(non_camel_case_types, reason = "Generated type parameters can need a numeric hygiene suffix.")]
        pub fn #helper<#source_mode: #nc::ThreadingModel, #result>(
            #source: #nc::ComRef<'_, Self, #source_mode>,
            #(#arguments,)*
            #visit: impl for<#loan> ::core::ops::FnOnce(#nc::ComRef<#loan, #interface_ty, #mode>) -> #result,
        ) -> #return_ty {
            #body
        }
    })
}
