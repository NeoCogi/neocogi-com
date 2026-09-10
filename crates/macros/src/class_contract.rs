//! Expansion of shared callable class contracts.
//!
//! A class contract owns one constructor and the primary callable surface of a
//! registered class. The constructor lowers to an immutable borrowed frame;
//! ordinary methods reuse the interface ABI machinery because both projections
//! have the same safe two-pointer header and C vtable representation.

use quote::{format_ident, quote};
use syn::{ItemTrait, PathArguments, Result, TraitBoundModifier, TraitItem, TraitItemFn, TypeParamBound};

use crate::abi_method::{parse_abi_trait_item, parse_class_constructor, validate_class_contract_trait};
use crate::args::{ClassContractArgs, InterfaceArgs, InterfaceGuid};
use crate::interface::{expand_com_interface_with_methods, validate_callable_methods};
use crate::model::{AbiArg, AbiArgKind, AbiConstructor};
use crate::util::{abi_hash_prefix_tokens, push_error, runtime_crate_path, stable_guid};

/// Generates one primary class projection, constructor frame, and registry identity.
pub(crate) fn expand_com_class_contract(args: ClassContractArgs, mut input: ItemTrait) -> Result<proc_macro2::TokenStream> {
    let nc = runtime_crate_path(args.runtime_crate.as_ref());
    let class_trait_ident = input.ident.clone();
    let class_vis = input.vis.clone();
    let class_abi_ident = format_ident!("{}Abi", input.ident);
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
    let qualified_name = args.auto_guid;
    let class_id = stable_guid(InterfaceGuid::AutoGuid { value: qualified_name.clone() }, &qualified_name, &nc)?;

    let mut errors = None;
    validate_class_contract_trait(&input, &mut errors);
    let mut constructor = None;
    let mut methods = Vec::new();
    for item in &mut input.items {
        match item {
            TraitItem::Fn(method) if has_constructor_marker(method) => {
                strip_constructor_marker(method, &mut errors);
                let parsed = parse_class_constructor(method);
                // Returning `Self` requires a sized implementor. Add this
                // mechanical bound to the emitted trait so authors do not have
                // to repeat an implementation detail of generated dispatch.
                method.sig.generics.make_where_clause().predicates.push(syn::parse_quote! { Self: Sized });
                match parsed {
                    Ok(parsed) if constructor.is_none() => constructor = Some(parsed),
                    Ok(_) => push_error(&mut errors, syn::Error::new_spanned(method, "a class contract has exactly one constructor")),
                    Err(err) => push_error(&mut errors, err),
                }
            }
            item => match parse_abi_trait_item(item) {
                Ok(method) => methods.push(method),
                Err(err) => push_error(&mut errors, err),
            },
        }
    }
    if constructor.is_none() {
        push_error(
            &mut errors,
            syn::Error::new_spanned(&input.ident, "a callable class contract requires one `#[constructor]` function"),
        );
    }
    validate_callable_methods(&input, &methods, &mut errors);
    if let Some(errors) = errors {
        return Err(errors);
    }
    let constructor = constructor.expect("validated constructor presence");

    // Primary class calls and optional interface calls intentionally share one
    // lowering path. This keeps ownership, method hashes, thunks, and safe
    // wrappers identical without exposing a redundant one-off interface.
    let callable = expand_com_interface_with_methods(
        InterfaceArgs {
            guid: InterfaceGuid::AutoGuid { value: qualified_name.clone() },
            qualified_name: qualified_name.clone(),
            runtime_crate: args.runtime_crate,
        },
        input,
        methods,
    )?;
    let constructor_tokens = constructor_tokens(&nc, &class_vis, &class_trait_ident, &class_abi_ident, &constructor);
    let arguments_type = if constructor.args.is_empty() {
        quote! { () }
    } else {
        let arguments_ident = format_ident!("{}Arguments", class_abi_ident);
        quote! { #arguments_ident<'a> }
    };
    Ok(quote! {
        #callable
        #constructor_tokens

        // SAFETY: The primary projection and constructor frame above are
        // generated from this exact class declaration. A `com_object(Class)`
        // implementation publishes the same identities and frame decoder.
        unsafe impl #nc::ComClassContract for #class_abi_ident {
            type Arguments<'a> = #arguments_type;

            const CLASS_ID: #nc::NcGuid = #class_id;
            const CONSTRUCTOR_ABI_HASH: #nc::NcAbiHash =
                #class_abi_ident::CONSTRUCTOR_ABI_HASH;
            const QUALIFIED_NAME: &'static str = #qualified_name;
            const VERSION: &'static str = env!("CARGO_PKG_VERSION");
            const INTERFACE_CHAINS: &'static [&'static [&'static #nc::NcInterfaceDescriptor]] = &[
                &[&#nc::IUnknown::INTERFACE_DESCRIPTOR],
                &#class_abi_ident::INTERFACE_CHAIN,
                #(&#required_interface_abis::INTERFACE_CHAIN,)*
            ];
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

/// Returns whether a trait method carries the constructor marker.
fn has_constructor_marker(method: &TraitItemFn) -> bool {
    method.attrs.iter().any(|attribute| attribute.path().is_ident("constructor"))
}

/// Removes and validates the macro-only constructor marker before emission.
fn strip_constructor_marker(method: &mut TraitItemFn, errors: &mut Option<syn::Error>) {
    let mut marker_count = 0usize;
    method.attrs.retain(|attribute| {
        if !attribute.path().is_ident("constructor") {
            return true;
        }
        marker_count += 1;
        if !matches!(attribute.meta, syn::Meta::Path(_)) {
            push_error(errors, syn::Error::new_spanned(attribute, "`constructor` does not accept options"));
        }
        false
    });
    if marker_count > 1 {
        push_error(errors, syn::Error::new_spanned(method, "a constructor needs only one `#[constructor]` marker"));
    }
}

/// Generates the safe argument wrapper and plugin-side frame decoder.
fn constructor_tokens(
    nc: &syn::Path,
    class_vis: &syn::Visibility,
    class_trait_ident: &syn::Ident,
    class_abi_ident: &syn::Ident,
    constructor: &AbiConstructor,
) -> proc_macro2::TokenStream {
    let constructor_ident = &constructor.ident;
    let constructor_docs = &constructor.doc_attrs;
    if constructor.args.is_empty() {
        return quote! {
            impl #class_abi_ident {
                /// Zero denotes the canonical empty constructor frame.
                pub const CONSTRUCTOR_ABI_HASH: #nc::NcAbiHash = #nc::NcAbiHash::ZERO;

                /// Builds the empty argument value for this class constructor.
                pub const fn arguments() {}

                /// Invokes the concrete Rust constructor after validating the empty frame.
                ///
                /// # Safety
                ///
                /// `arguments` must be null, as guaranteed by the checked class
                /// constructor wrapper for this zero-hash contract.
                #[doc(hidden)]
                pub unsafe fn __nc_construct<T>(
                    arguments: *const core::ffi::c_void,
                ) -> Result<T, #nc::NcStatus>
                where
                    T: #class_trait_ident,
                {
                    if !arguments.is_null() {
                        return Err(#nc::NC_E_INVALID_ARG);
                    }
                    <T as #class_trait_ident>::#constructor_ident()
                }
            }
        };
    }

    let arguments_ident = format_ident!("{}Arguments", class_abi_ident);
    let frame_ident = format_ident!("__{}ConstructorFrame", class_abi_ident);
    let frame_fields = constructor.args.iter().flat_map(frame_fields);
    let builder_params = constructor.args.iter().map(|argument| {
        let ident = &argument.ident;
        let ty = safe_constructor_arg_type(nc, argument);
        quote! { #ident: #ty }
    });
    let builder_preparations = constructor.args.iter().filter_map(|argument| {
        if matches!(argument.kind, AbiArgKind::CopyFromBufferOffset { .. }) {
            let ident = &argument.ident;
            let len_ident = format_ident!("__nc_{ident}_len");
            Some(quote! {
                // Convert the platform-sized slice length before assembling the
                // fixed-width C frame so failure cannot publish a partial frame.
                let #len_ident = #ident.__nc_len_u64()?;
            })
        } else {
            None
        }
    });
    let frame_initializers = constructor.args.iter().flat_map(frame_initializers);
    let decoded_arguments = constructor.args.iter().map(|argument| decode_argument(nc, argument));
    let argument_names = constructor.args.iter().map(|argument| &argument.ident);
    let hash_parts = constructor
        .args
        .iter()
        .enumerate()
        .map(|(index, argument)| constructor_hash_part(nc, index, argument));
    let schema_parts = constructor
        .args
        .iter()
        .enumerate()
        .map(|(index, _)| neocogi_com_core::NcAbiHashPart::Arg {
            index: index as u32,
            passing: neocogi_com_core::NcAbiPassing::Void,
            ty: neocogi_com_core::NcAbiType::Void,
        })
        .collect::<Vec<_>>();
    let hash_prefix = abi_hash_prefix_tokens(&schema_parts, nc);

    quote! {
        /// Immutable C-shaped storage borrowed only for one constructor call.
        #[repr(C)]
        struct #frame_ident {
            #(#frame_fields)*
        }

        /// Typed borrowed arguments for this class constructor.
        #class_vis struct #arguments_ident<'a> {
            /// Complete immutable frame passed through the erased runtime boundary.
            frame: #frame_ident,
            /// Retains the shortest lifetime of every pointer stored in `frame`.
            borrow: core::marker::PhantomData<&'a ()>,
        }

        // SAFETY: The generated wrapper owns a `repr(C)` frame whose fields,
        // passing modes, order, and hash are emitted from the same lowered
        // constructor parameters. `borrow` prevents referenced data from being
        // invalidated while the runtime uses the returned frame pointer.
        unsafe impl<'a> #nc::ComClassArguments for #arguments_ident<'a> {
            const ABI_HASH: #nc::NcAbiHash = #class_abi_ident::CONSTRUCTOR_ABI_HASH;

            /// Borrows the wrapper's stable frame for one activation call.
            fn as_abi_ptr(&self) -> *const core::ffi::c_void {
                core::ptr::from_ref(&self.frame).cast()
            }
        }

        impl #class_abi_ident {
            /// Hash of the constructor's ordered logical parameter contract.
            pub const CONSTRUCTOR_ABI_HASH: #nc::NcAbiHash =
                #nc::NcAbiHash::__from_abi_prefix(#hash_prefix, &[
                    #(#hash_parts,)*
                ]);

            #(#constructor_docs)*
            /// Builds a lifetime-safe immutable constructor frame.
            pub fn arguments<'a>(
                #(#builder_params,)*
            ) -> Result<#arguments_ident<'a>, #nc::NcStatus> {
                #(#builder_preparations)*
                Ok(#arguments_ident {
                    frame: #frame_ident {
                        #(#frame_initializers)*
                    },
                    borrow: core::marker::PhantomData,
                })
            }

            /// Decodes one checked frame and invokes the concrete Rust constructor.
            ///
            /// # Safety
            ///
            /// `arguments` must point to a readable generated frame whose nested
            /// borrows remain valid for this call. The checked class constructor
            /// wrapper establishes the non-null and ABI-hash requirements.
            #[doc(hidden)]
            pub unsafe fn __nc_construct<T>(
                arguments: *const core::ffi::c_void,
            ) -> Result<T, #nc::NcStatus>
            where
                T: #class_trait_ident,
            {
                if arguments.is_null() {
                    return Err(#nc::NC_E_INVALID_ARG);
                }
                // SAFETY: The caller guarantees the pointer identifies this
                // generated frame layout for the duration of the call.
                let frame = unsafe { &*arguments.cast::<#frame_ident>() };
                #(#decoded_arguments)*
                <T as #class_trait_ident>::#constructor_ident(
                    #(#argument_names,)*
                )
            }
        }
    }
}

/// Emits documented physical frame fields for one logical constructor argument.
fn frame_fields(argument: &AbiArg) -> Vec<proc_macro2::TokenStream> {
    let ident = &argument.ident;
    match &argument.kind {
        AbiArgKind::Plain | AbiArgKind::ComRef { .. } => {
            let ty = &argument.abi_ty;
            vec![quote! {
                #[doc = concat!("Lowered storage for constructor argument `", stringify!(#ident), "`.")]
                #ident: #ty,
            }]
        }
        AbiArgKind::CopyFromBufferOffset { element_ty } => {
            let element_ty = element_ty.as_ref();
            let offset = format_ident!("__nc_{ident}_offset");
            let len = format_ident!("__nc_{ident}_len");
            let ptr = format_ident!("__nc_{ident}_ptr");
            vec![
                quote! {
                    #[doc = concat!("Logical offset for constructor buffer `", stringify!(#ident), "`.")]
                    #offset: u64,
                },
                quote! {
                    #[doc = concat!("Element count for constructor buffer `", stringify!(#ident), "`.")]
                    #len: u64,
                },
                quote! {
                    #[doc = concat!("Borrowed data pointer for constructor buffer `", stringify!(#ident), "`.")]
                    #ptr: *const #element_ty,
                },
            ]
        }
        AbiArgKind::CopyToBufferOffset { .. } => {
            unreachable!("constructor validation rejects writable buffers")
        }
    }
}

/// Selects the safe builder type while naming its borrow as `'a`.
fn safe_constructor_arg_type(nc: &syn::Path, argument: &AbiArg) -> proc_macro2::TokenStream {
    match &argument.kind {
        AbiArgKind::Plain => {
            let ty = &argument.rust_ty;
            quote! { #ty }
        }
        AbiArgKind::CopyFromBufferOffset { element_ty } => {
            let element_ty = element_ty.as_ref();
            quote! { #nc::NcCopyFromBufferOffset<'a, #element_ty> }
        }
        AbiArgKind::ComRef { interface_ty, mode_ty } => {
            let interface_ty = interface_ty.as_ref();
            let mode_ty = mode_ty.as_deref().map(|ty| quote! { #ty }).unwrap_or_else(|| quote! { #nc::ThreadLocal });
            quote! { #nc::ComRef<'a, #interface_ty, #mode_ty> }
        }
        AbiArgKind::CopyToBufferOffset { .. } => {
            unreachable!("constructor validation rejects writable buffers")
        }
    }
}

/// Emits frame initialization values for one safe builder argument.
fn frame_initializers(argument: &AbiArg) -> Vec<proc_macro2::TokenStream> {
    let ident = &argument.ident;
    match &argument.kind {
        AbiArgKind::Plain => vec![quote! { #ident, }],
        AbiArgKind::ComRef { .. } => vec![quote! { #ident: #ident.as_mut_ptr(), }],
        AbiArgKind::CopyFromBufferOffset { .. } => {
            let offset = format_ident!("__nc_{ident}_offset");
            let len = format_ident!("__nc_{ident}_len");
            let ptr = format_ident!("__nc_{ident}_ptr");
            vec![quote! { #offset: #ident.offset(), }, quote! { #len, }, quote! { #ptr: #ident.as_ptr(), }]
        }
        AbiArgKind::CopyToBufferOffset { .. } => {
            unreachable!("constructor validation rejects writable buffers")
        }
    }
}

/// Reconstructs one safe Rust constructor argument from the raw frame.
fn decode_argument(nc: &syn::Path, argument: &AbiArg) -> proc_macro2::TokenStream {
    let ident = &argument.ident;
    match &argument.kind {
        AbiArgKind::Plain => quote! {
            let #ident = frame.#ident;
        },
        AbiArgKind::CopyFromBufferOffset { element_ty } => {
            let element_ty = element_ty.as_ref();
            let offset = format_ident!("__nc_{ident}_offset");
            let len = format_ident!("__nc_{ident}_len");
            let ptr = format_ident!("__nc_{ident}_ptr");
            quote! {
                // SAFETY: Frame validity includes the readable nested buffer
                // borrow. The checked wrapper already matched the frame hash.
                let #ident = unsafe {
                    #nc::NcCopyFromBufferOffset::<#element_ty>::__nc_from_raw_parts(
                        frame.#offset,
                        frame.#ptr,
                        frame.#len,
                    )
                }?;
            }
        }
        AbiArgKind::ComRef { interface_ty, mode_ty } => {
            let interface_ty = interface_ty.as_ref();
            let mode_ty = mode_ty.as_deref().map(|ty| quote! { #ty }).unwrap_or_else(|| quote! { #nc::ThreadLocal });
            quote! {
                // SAFETY: Frame validity includes the live borrowed COM
                // pointer. This conversion also checks its full interface ID.
                let #ident = unsafe {
                    #nc::ComRef::<#interface_ty, #mode_ty>::__nc_from_raw_borrowed_checked(
                        frame.#ident,
                    )
                }?;
            }
        }
        AbiArgKind::CopyToBufferOffset { .. } => {
            unreachable!("constructor validation rejects writable buffers")
        }
    }
}

/// Emits one ordered constructor parameter hash component.
fn constructor_hash_part(nc: &syn::Path, index: usize, argument: &AbiArg) -> proc_macro2::TokenStream {
    let index = index as u32;
    match &argument.kind {
        AbiArgKind::Plain => {
            let ty = &argument.abi_ty;
            quote! {
                #nc::NcAbiHashPart::Arg {
                    index: #index,
                    passing: #nc::NcAbiPassing::Value,
                    ty: #nc::__nc_abi_type::<#ty>(),
                }
            }
        }
        AbiArgKind::CopyFromBufferOffset { element_ty } => {
            let element_ty = element_ty.as_ref();
            quote! {
                #nc::NcAbiHashPart::Arg {
                    index: #index,
                    passing: #nc::NcAbiPassing::CopyFromBufferOffset,
                    ty: #nc::__nc_abi_plain_type::<#element_ty>(),
                }
            }
        }
        AbiArgKind::ComRef { interface_ty, mode_ty } => {
            let interface_ty = interface_ty.as_ref();
            let mode = match mode_ty.as_deref() {
                Some(mode_ty) => quote! {
                    <#mode_ty as #nc::ThreadingModel>::ABI_THREADING_MODE
                },
                None => quote! {
                    <#nc::ThreadLocal as #nc::ThreadingModel>::ABI_THREADING_MODE
                },
            };
            quote! {
                #nc::NcAbiHashPart::Arg {
                    index: #index,
                    passing: #nc::NcAbiPassing::ComRef { mode: #mode },
                    ty: #nc::NcAbiType::Interface {
                        iid: <#interface_ty as #nc::ComInterface>::IID,
                    },
                }
            }
        }
        AbiArgKind::CopyToBufferOffset { .. } => {
            unreachable!("constructor validation rejects writable buffers")
        }
    }
}
