//! Method documentation derived from the same typed ownership plan as dispatch.
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::Ident;
use crate::model::{AbiMethod, AbiArg, AbiArgKind, AbiReturn, AbiReturnKind};

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
        #(#return_docs)*
    }]
}

/// Generates pointer, bounds, and ownership documentation for one raw argument.
fn raw_vtable_arg_docs(arg: &AbiArg) -> Vec<proc_macro2::TokenStream> {
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
            let abi_ty = &arg.abi_ty;
            vec![literal_doc(
                format!("- `{ident}` is passed by value as `{abi_ty}`.", abi_ty = abi_ty.to_token_stream()),
                ident.span(),
            )]
        }
        AbiArgKind::ComRef { interface_ty, .. } => {
            let interface_ty = interface_ty.as_ref();
            vec![literal_doc(
                format!(
                    "- `{ident}` is a borrowed `{interface_ty}` object/table pair. Both fields must be non-null. The callee must call `AddRef` before retaining a present interface after the call returns.",
                    interface_ty = interface_ty.to_token_stream()
                ),
                ident.span(),
            )]
        }
        AbiArgKind::CopyFromBufferOffset { element_ty } => {
            let element_ty = element_ty.as_ref();
            vec![literal_doc(
                format!(
                    "- `{ident}` lowers to `offset: u64`, `len: u64`, and a readable `*const {element_ty}` buffer.",
                    element_ty = element_ty.to_token_stream()
                ),
                ident.span(),
            )]
        }
        AbiArgKind::OutputBuffer { element_ty } => {
            let element_ty = element_ty.as_ref();
            vec![literal_doc(
                format!(
                    "- `{ident}` lowers to `offset: u64`, `capacity: u64`, writable `*mut <{element_ty} as ComValue>::Wire`, and a separate `written: *mut u64`. Old contents are not input. Only the successfully reported initialized prefix may be read; count and element storage must not alias.",
                    element_ty = element_ty.to_token_stream()
                ),
                ident.span(),
            )]
        }
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
        AbiReturnKind::ComPtr { interface_ty, .. } => {
            let interface_ty = interface_ty.as_ref();
            vec![literal_doc(
                format!(
                    "On success, `out` receives an owned `{interface_ty}` object/table pair transferred to the caller.",
                    interface_ty = interface_ty.to_token_stream()
                ),
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
        AbiReturnKind::ComPtr { interface_ty, .. } => {
            let rust_ty = &ok.rust_ty;
            let interface_ty = interface_ty.as_ref();
            vec![literal_doc(
                format!(
                    "On success, adopts the owned `{interface_ty}` interface pointer returned through `out` as `{rust_ty}`.",
                    interface_ty = interface_ty.to_token_stream(),
                    rust_ty = rust_ty.to_token_stream()
                ),
                ok.rust_ty.span(),
            )]
        }
    }
}
