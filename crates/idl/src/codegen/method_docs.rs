//! Method documentation derived from the same typed ownership plan as dispatch.
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::Ident;
use crate::codegen::model::{AbiMethod, AbiArg, AbiArgKind, AbiReturn, AbiReturnKind, InterfaceOwnership};

/// Backend-created prose is evaluated here; consumers receive only literal text.
/// Authored documentation/provenance attributes are retained separately by callers.
pub(crate) fn literal_doc(text: String, span: proc_macro2::Span) -> proc_macro2::TokenStream {
    let text = syn::LitStr::new(&text, span);
    quote! { #[doc = #text] }
}

/// Generates rustdoc describing one unsafe raw vtable slot and its contracts.
pub(crate) fn raw_vtable_method_docs(trait_ident: &Ident, method: &AbiMethod) -> Vec<proc_macro2::TokenStream> {
    let method_ident = &method.ident;
    let arg_docs = method.args.iter().flat_map(raw_vtable_arg_docs);
    let status_doc = match &method.ok.kind {
        AbiReturnKind::Void => quote! {
            #[doc = "Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer."]
        },
        AbiReturnKind::Plain | AbiReturnKind::ComPtr { .. } => quote! {
            #[doc = "Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful."]
        },
    };
    let safety_doc = match &method.ok.kind {
        AbiReturnKind::Void => quote! {
            #[doc = "`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null."]
        },
        AbiReturnKind::Plain | AbiReturnKind::ComPtr { .. } => quote! {
            #[doc = "`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null."]
        },
    };
    let return_docs = raw_vtable_return_docs(&method.ok);

    let leading_doc = literal_doc(format!("Raw COM ABI vtable slot for [`{trait_ident}::{method_ident}`]."), method_ident.span());

    vec![quote! {
        #leading_doc
        #[doc = ""]
        #status_doc
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #safety_doc
        #(#arg_docs)*
        #[doc = ""]
        #(#return_docs)*
    }]
}

/// Generates pointer, bounds, and ownership documentation for one raw argument.
fn raw_vtable_arg_docs(arg: &AbiArg) -> Vec<proc_macro2::TokenStream> {
    let ident = &arg.ident;
    // Physical storage comes from the dispatch plan, never reconstructed from
    // the application element/interface type. Native enums need checked wires.
    let physical = arg.abi_ty.to_token_stream();
    let native = arg.rust_ty.to_token_stream();
    let contract = match &arg.kind {
        AbiArgKind::ComBox { .. } => {
            "The writable owner slot transfers one exclusive owner. Success clears the slot. Every error restores the original owner unchanged; the caller decides whether to retry or destroy it. Providers must finish all fallible preparation before committing ownership."
        }
        AbiArgKind::BorrowedValue { .. } => "The pointed-to checked representation is borrowed; the receiver acquires owned fields before retaining data.",
        AbiArgKind::Plain => "Passed by value.",
        AbiArgKind::ComRef { .. } => {
            "This is a borrowed object/table pair. Both fields must be non-null. The callee must call `AddRef` before retaining the interface after the call returns."
        }
        AbiArgKind::CopyFromBufferOffset { .. } => "The readable buffer pointer is accompanied by `offset: u64` and `len: u64`.",
        AbiArgKind::OutputBuffer { .. } => {
            "The writable buffer pointer is accompanied by `offset: u64`, `capacity: u64`, and a separate `written: *mut u64`. Old contents are not input. Only the successfully reported initialized prefix may be read; count and element storage must not alias."
        }
    };
    vec![literal_doc(
        format!("- `{ident}` uses ABI storage `{physical}` for `{native}`. {contract}"),
        ident.span(),
    )]
}

/// Describes the existing result token category without reconstructing its wire.
/// Both raw and safe methods promise the same ownership with different type spelling.
fn returned_ownership(ownership: InterfaceOwnership) -> &'static str {
    match ownership {
        InterfaceOwnership::Shared => "The caller receives one shared owner and releases it when finished.",
        InterfaceOwnership::Exclusive => "The caller receives the sole exclusive owner; interface queries cannot duplicate it.",
        InterfaceOwnership::WeakExclusive => "The caller receives a weak observer that retains allocation storage, never the concrete value.",
        InterfaceOwnership::WeakShared => "The caller receives one shared weak control token; promotion is required before accessing the value.",
    }
}

/// Generates output-slot documentation for one lowered return value.
fn raw_vtable_return_docs(ok: &AbiReturn) -> Vec<proc_macro2::TokenStream> {
    match &ok.kind {
        AbiReturnKind::Void => {
            vec![quote! {
                #[doc = "On success, no output value is written."]
            }]
        }
        AbiReturnKind::Plain => {
            let abi_ty = &ok.abi_ty;
            vec![literal_doc(
                format!("On success, `out` receives `{abi_ty}`.", abi_ty = abi_ty.to_token_stream()),
                ok.rust_ty.span(),
            )]
        }
        AbiReturnKind::ComPtr { ownership, .. } => {
            let physical = ok.abi_ty.to_token_stream();
            let native = ok.rust_ty.to_token_stream();
            let contract = returned_ownership(*ownership);
            vec![literal_doc(
                format!("On success, `out` receives `{physical}` for `{native}`. {contract}"),
                ok.rust_ty.span(),
            )]
        }
    }
}

/// Generates rustdoc for a safe trait method or smart-pointer forwarding method.
pub(crate) fn safe_method_docs(trait_ident: &Ident, method: &AbiMethod) -> Vec<proc_macro2::TokenStream> {
    let method_ident = &method.ident;
    let arg_docs = method.args.iter().flat_map(safe_arg_docs);
    let wrapper_doc = match &method.ok.kind {
        AbiReturnKind::Void => quote! {
            #[doc = "This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`."]
        },
        AbiReturnKind::Plain | AbiReturnKind::ComPtr { .. } => quote! {
            #[doc = "This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful."]
        },
    };
    let return_docs = safe_return_docs(&method.ok);

    let leading_doc = literal_doc(
        format!("Calls [`{trait_ident}::{method_ident}`] through the generated COM ABI vtable."),
        method_ident.span(),
    );

    vec![quote! {
        #[doc = ""]
        #leading_doc
        #[doc = ""]
        #wrapper_doc
        #[doc = "Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`."]
        #(#arg_docs)*
        #[doc = ""]
        #(#return_docs)*
    }]
}

/// Describes safe-wrapper ownership and bounds for one source-level argument.
fn safe_arg_docs(arg: &AbiArg) -> Vec<proc_macro2::TokenStream> {
    let ident = &arg.ident;
    match &arg.kind {
        AbiArgKind::ComBox { .. } => vec![literal_doc(
            format!(
                "- `{ident}` transfers one exclusive owner. Success clears the writable owner slot. Every error restores the original owner unchanged; the caller decides whether to retry or destroy it. Providers must finish all fallible preparation before committing ownership."
            ),
            ident.span(),
        )],
        AbiArgKind::BorrowedValue { .. } => vec![literal_doc(
            format!("- `{ident}` borrows its exact generated message representation; the receiver acquires owned fields before retaining data."),
            ident.span(),
        )],
        AbiArgKind::Plain => {
            let rust_ty = &arg.rust_ty;
            vec![literal_doc(
                format!("- `{ident}` is passed by value as `{rust_ty}`.", rust_ty = rust_ty.to_token_stream()),
                ident.span(),
            )]
        }
        AbiArgKind::ComRef { interface_ty, .. } => {
            let interface_ty = interface_ty.as_ref();
            vec![literal_doc(
                format!(
                    "- `{ident}` borrows a `{interface_ty}` interface for the duration of the call. An interface is required. The callee must call `AddRef` before retaining a present interface.",
                    interface_ty = interface_ty.to_token_stream()
                ),
                ident.span(),
            )]
        }
        AbiArgKind::CopyFromBufferOffset { element_ty } => {
            let element_ty = element_ty.as_ref();
            vec![literal_doc(
                format!(
                    "- `{ident}` is passed as a safe readable `{element_ty}` buffer plus offset.",
                    element_ty = element_ty.to_token_stream()
                ),
                ident.span(),
            )]
        }
        AbiArgKind::OutputBuffer { element_ty } => {
            let element_ty = element_ty.as_ref();
            vec![literal_doc(
                format!(
                    "- `{ident}` supplies output storage for `{element_ty}` through [`::neocogi_com::NcOutput`]. See its lifecycle contract for acquisition, preparation and publication.",
                    element_ty = element_ty.to_token_stream()
                ),
                ident.span(),
            )]
        }
    }
}

/// Describes status-only, copied, or transferred successful return values.
fn safe_return_docs(ok: &AbiReturn) -> Vec<proc_macro2::TokenStream> {
    match &ok.kind {
        AbiReturnKind::Void => {
            vec![quote! {
                #[doc = "On success, returns `()`."]
            }]
        }
        AbiReturnKind::Plain => {
            let rust_ty = &ok.rust_ty;
            vec![literal_doc(
                format!("On success, returns `{rust_ty}` from the ABI out slot.", rust_ty = rust_ty.to_token_stream()),
                ok.rust_ty.span(),
            )]
        }
        AbiReturnKind::ComPtr { ownership, .. } => {
            let rust_ty = &ok.rust_ty;
            let contract = returned_ownership(*ownership);
            vec![literal_doc(
                format!("On success, returns `{rust_ty}`. {contract}", rust_ty = rust_ty.to_token_stream()),
                ok.rust_ty.span(),
            )]
        }
    }
}
