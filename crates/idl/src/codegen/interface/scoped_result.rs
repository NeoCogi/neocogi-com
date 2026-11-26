//! Scoped result emission from an already validated callable method.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Lifetime, Path, Type, ext::IdentExt};

use super::{AbiMethod, AbiReceiver, AbiReturnKind, PrivateNames, safe_arg_preparation, safe_vtable_call_args};
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

/// Emits a scoped getter accepting its proven physical prefix and source access.
pub(super) fn marker_method(nc: &Path, method: &AbiMethod, declaring_id: neocogi_com::NcGuid, names: &mut PrivateNames) -> Option<TokenStream> {
    let AbiReturnKind::ScopedLoan { interface_ty, mode_ty, mutable } = &method.ok.kind else {
        return None;
    };
    let mode = match mode_ty {
        Some(mode) => quote! { #mode },
        None => quote! { #nc::ThreadLocal },
    };
    let (output_handle, runtime_helper) = if *mutable {
        (quote! { ComMut }, quote! { __nc_with_scoped_mut_result })
    } else {
        (quote! { ComRef }, quote! { __nc_with_scoped_result })
    };
    let source_handle = match method.receiver {
        AbiReceiver::Shared => quote! { ComRef },
        AbiReceiver::Mutable => quote! { ComMut },
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
    let table = names.binding("__nc_scoped_table");
    let iid = proc_macro2::Literal::u128_unsuffixed(u128::from_be_bytes(declaring_id.to_uuid_bytes()));
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
        let #table = #pair.vtable.cast::<<Self as #nc::ComInterface>::VTable>();
        // SAFETY: The live receiver, proven first-base prefix and prepared inputs
        // last through this synchronous call. The raw slot admits the receiver and lends its result
        // through the installed continuation on the calling thread.
        unsafe {
            #nc::#runtime_helper::<#interface_ty, #mode, #result>(
                #visit,
                |#context, #callback| {
                    ((*#table).#method_ident)(
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
        /// The provider keeps the actual lender alive through the complete visitor.
        /// The visitor's output cannot carry that loan beyond the call.
        #[doc = #status_doc]
        ///
        /// # Panics
        ///
        /// An uncaught panic in the visitor terminates the process at the
        /// non-unwinding C callback boundary.
        #[allow(non_camel_case_types, reason = "Generated type parameters can need a numeric hygiene suffix.")]
        pub fn #helper<#source_mode: #nc::ThreadingModel, #result>(
            #source: #nc::#source_handle<'_, impl #nc::__NcScopedPrefix<#iid, Base = Self>, #source_mode>,
            #(#arguments,)*
            #visit: impl for<#loan> ::core::ops::FnOnce(#nc::#output_handle<#loan, #interface_ty, #mode>) -> #result,
        ) -> #return_ty {
            #body
        }
    })
}
