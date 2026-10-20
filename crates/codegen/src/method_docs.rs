//! Method documentation derived from the same typed ownership plan as dispatch.
use quote::quote;
use syn::Ident;
use crate::model::{AbiMethod, AbiArg, AbiArgKind, AbiReturn, AbiReturnKind};

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

    vec![quote! {
        #[doc = concat!("Raw COM ABI vtable slot for [`", stringify!(#trait_ident), "::", stringify!(#method_ident), "`].")]
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
        AbiArgKind::ComBox { .. } => vec![quote! {
            #[doc = concat!("- `", stringify!(#ident), "` transfers one exclusive owner. Success clears the writable owner slot. Every error restores the original owner unchanged; the caller decides whether to retry or destroy it. Providers must finish all fallible preparation before committing ownership.")]
        }],
        AbiArgKind::BorrowedValue { .. } => vec![quote! {
            #[doc = concat!("- `", stringify!(#ident), "` borrows its exact generated message representation; the receiver acquires owned fields before retaining data.")]
        }],
        AbiArgKind::Plain => {
            let abi_ty = &arg.abi_ty;
            vec![quote! {
                #[doc = concat!("- `", stringify!(#ident), "` is passed by value as `", stringify!(#abi_ty), "`.")]
            }]
        }
        AbiArgKind::ComRef { interface_ty, .. } => {
            let interface_ty = interface_ty.as_ref();
            vec![quote! {
                #[doc = concat!("- `", stringify!(#ident), "` is a borrowed `", stringify!(#interface_ty), "` object/table pair.", " Both fields must be non-null. The callee must call `AddRef` before retaining a present interface after the call returns.")]
            }]
        }
        AbiArgKind::CopyFromBufferOffset { element_ty } => {
            let element_ty = element_ty.as_ref();
            vec![quote! {
                #[doc = concat!("- `", stringify!(#ident), "` lowers to `offset: u64`, `len: u64`, and a readable `*const ", stringify!(#element_ty), "` buffer.")]
            }]
        }
        AbiArgKind::OutputBuffer { element_ty } => {
            let element_ty = element_ty.as_ref();
            vec![quote! {
                #[doc = concat!("- `", stringify!(#ident), "` lowers to `offset: u64`, `capacity: u64`, writable `*mut <", stringify!(#element_ty), " as ComValue>::Wire`, and a separate `written: *mut u64`. Old contents are not input. Only the successfully reported initialized prefix may be read; count and element storage must not alias.")]
            }]
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
            vec![quote! {
                #[doc = concat!("On success, `out` receives `", stringify!(#abi_ty), "`.")]
            }]
        }
        AbiReturnKind::ComPtr { interface_ty, .. } => {
            let interface_ty = interface_ty.as_ref();
            vec![quote! {
                #[doc = concat!("On success, `out` receives an owned `", stringify!(#interface_ty), "` object/table pair transferred to the caller.")]
            }]
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

    vec![quote! {
        #[doc = ""]
        #[doc = concat!("Calls [`", stringify!(#trait_ident), "::", stringify!(#method_ident), "`] through the generated COM ABI vtable.")]
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
        AbiArgKind::ComBox { .. } => vec![quote! {
            #[doc = concat!("- `", stringify!(#ident), "` transfers one exclusive owner. Success clears the writable owner slot. Every error restores the original owner unchanged; the caller decides whether to retry or destroy it. Providers must finish all fallible preparation before committing ownership.")]
        }],
        AbiArgKind::BorrowedValue { .. } => vec![quote! {
            #[doc = concat!("- `", stringify!(#ident), "` borrows its exact generated message representation; the receiver acquires owned fields before retaining data.")]
        }],
        AbiArgKind::Plain => {
            let rust_ty = &arg.rust_ty;
            vec![quote! {
                #[doc = concat!("- `", stringify!(#ident), "` is passed by value as `", stringify!(#rust_ty), "`.")]
            }]
        }
        AbiArgKind::ComRef { interface_ty, .. } => {
            let interface_ty = interface_ty.as_ref();
            vec![quote! {
                #[doc = concat!("- `", stringify!(#ident), "` borrows a `", stringify!(#interface_ty), "` interface for the duration of the call.", " An interface is required. The callee must call `AddRef` before retaining a present interface.")]
            }]
        }
        AbiArgKind::CopyFromBufferOffset { element_ty } => {
            let element_ty = element_ty.as_ref();
            vec![quote! {
                #[doc = concat!("- `", stringify!(#ident), "` is passed as a safe readable `", stringify!(#element_ty), "` buffer plus offset.")]
            }]
        }
        AbiArgKind::OutputBuffer { element_ty } => {
            let element_ty = element_ty.as_ref();
            vec![quote! {
                #[doc = concat!("- `", stringify!(#ident), "` takes `NcOutput<", stringify!(#element_ty), ">` from caller-owned output storage. Preparation discards prior results and reserves before dispatch. Only the successfully validated prefix is published; the provider receives `NcOutputWriter` and cannot read old destination values.")]
            }]
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
            vec![quote! {
                #[doc = concat!("On success, returns `", stringify!(#rust_ty), "` from the ABI out slot.")]
            }]
        }
        AbiReturnKind::ComPtr { interface_ty, .. } => {
            let rust_ty = &ok.rust_ty;
            let interface_ty = interface_ty.as_ref();
            vec![quote! {
                #[doc = concat!("On success, adopts the owned `", stringify!(#interface_ty), "` interface pointer returned through `out` as `", stringify!(#rust_ty), "`.")]
            }]
        }
    }
}
