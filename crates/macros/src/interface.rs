//! Generation of interface headers, vtables, thunks, wrappers, and ABI metadata.
//!
//! A validated trait is lowered into one `repr(C)` interface header and vtable,
//! per-implementor thunk table, one borrowed client method view, and immutable
//! method descriptors. The client view is the sole safe Rust-to-ABI lowering
//! surface, so pointer ownership, buffer bounds, and documentation stay aligned.

use quote::{format_ident, quote};
use syn::{ItemTrait, Result, TypeParamBound, TraitBoundModifier, PathArguments};

use crate::abi_method::{parse_abi_trait_item, raw_vtable_method_docs, safe_method_docs, validate_interface_trait};
use crate::args::{InterfaceArgs, InterfaceThreading};
use crate::model::{AbiArgKind, AbiMethod, AbiReceiver, AbiReturnKind};
use crate::util::{abi_signature_static_tokens, checked_u32_len, collect_syn_results, interface_field, interface_guid, push_error, runtime_crate_path};

/// Expands one validated interface trait into its complete Rust and C ABI surface.
pub(crate) fn expand_com_interface(args: InterfaceArgs, input: ItemTrait) -> Result<proc_macro2::TokenStream> {
    let mut errors = None;
    validate_interface_trait(&input, &mut errors);

    // Every trait item is a callable vtable method. IDL objects and factories
    // use the same lowering after the generator attaches description metadata.
    let nc = runtime_crate_path(args.runtime_crate.as_ref());
    let methods = match collect_syn_results(input.items.iter().map(|item| parse_abi_trait_item(item, &nc))) {
        Ok(methods) => methods,
        Err(err) => {
            push_error(&mut errors, err);
            Vec::new()
        }
    };
    validate_callable_methods(&input, &methods, &mut errors);
    if let Some(errors) = errors {
        return Err(errors);
    }

    expand_com_interface_with_methods(args, input, methods)
}

/// Generates a callable projection from an already validated method list.
///
/// The emitted source trait remains intact, while `methods` become vtable
/// slots, typed descriptions, thunks, and safe client methods.
pub(crate) fn expand_com_interface_with_methods(args: InterfaceArgs, input: ItemTrait, methods: Vec<AbiMethod>) -> Result<proc_macro2::TokenStream> {
    let InterfaceExpansion { abi, provider } = expand_interface_parts(args, input, methods)?;
    Ok(quote! { #abi #provider })
}

/// Separately emitted contract and provider support for built-in interfaces.
///
/// Core owns the ABI and safe clients. The facade owns generated object thunks,
/// whose borrow gates and concrete object storage must not enter the core ABI.
pub(crate) struct InterfaceExpansion {
    /// Traits, headers, clients and metadata, using only allocation-free core types.
    pub(crate) abi: proc_macro2::TokenStream,
    /// Concrete-provider projections and thunks, using facade object support.
    pub(crate) provider: proc_macro2::TokenStream,
}

/// Lowers both halves once so checked-in built-ins and ordinary IDL stay identical.
pub(crate) fn expand_interface_parts(args: InterfaceArgs, input: ItemTrait, methods: Vec<AbiMethod>) -> Result<InterfaceExpansion> {
    // Generated names are deliberately mechanical and predictable. A trait
    // named `IFoo` produces `IFooAbi`, `IFooVTable`, `IFooVtblFor<T>`, and
    // `IFooClient`.
    let nc = runtime_crate_path(args.runtime_crate.as_ref());
    let trait_ident = &input.ident;
    let vis = &input.vis;
    // Implicit format captures do not pass Ident fragments to quote's span
    // inference. Keep each derived public name anchored to the source trait,
    // including when the trait comes from an OUT_DIR include.
    let abi_ident = format_ident!("{trait_ident}Abi", span = trait_ident.span());
    let vtable_ident = format_ident!("{trait_ident}VTable", span = trait_ident.span());
    let vtable_for_ident = format_ident!("{trait_ident}VtblFor", span = trait_ident.span());
    let client_ident = format_ident!("{trait_ident}Client", span = trait_ident.span());
    let interfaces_ident = format_ident!("{trait_ident}InterfacesFor", span = trait_ident.span());
    let threading = match args.threading {
        InterfaceThreading::Local => quote! { #nc::ThreadLocal },
        InterfaceThreading::Shared => quote! { #nc::ThreadSafe },
    };
    let factory_marker = args.factory.then(|| {
        quote! {
            // SAFETY: IDL assigned the factory role to this exact callable contract.
            unsafe impl #nc::ComFactoryInterface for #abi_ident {}
        }
    });
    let qualified_name = args.qualified_name;
    let guid = interface_guid(args.guid, &qualified_name, &nc)?;
    let assigned_signature = args.assigned_signature.clone().ok_or_else(|| {
        syn::Error::new(
            trait_ident.span(),
            "com_interface requires an IDL-generator assigned_signature; define cross-component contracts in .ncidl",
        )
    })?;
    // Single inheritance has one unambiguous physical prefix. Generated names
    // retain the original module path, so bases may live in another crate.
    let interface_paths = input
        .supertraits
        .iter()
        .map(|bound| match bound {
            TypeParamBound::Trait(bound)
                if bound.modifier == TraitBoundModifier::None
                    && bound.lifetimes.is_none()
                    && bound.path.segments.iter().all(|segment| matches!(segment.arguments, PathArguments::None)) =>
            {
                if bound
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| matches!(segment.ident.to_string().as_str(), "Send" | "Sync" | "Sized" | "Unpin"))
                {
                    return Err(syn::Error::new_spanned(
                        bound,
                        "COM bases must be generated interfaces; use threading = shared for Send + Sync",
                    ));
                }
                Ok(bound.path.clone())
            }
            _ => Err(syn::Error::new_spanned(bound, "expected a nongeneric generated COM interface")),
        })
        .collect::<Result<Vec<_>>>()?;
    let base = interface_paths.first();
    // The first supertrait supplies the physical prefix. Every later one is
    // a required sibling projection; all refer to the same owned Rust value.
    let additional_interfaces = interface_paths
        .iter()
        .skip(1)
        .map(|path| {
            let abi = generated_path(path, "Abi");
            let interfaces_for = generated_path(path, "InterfacesFor");
            let field = interface_field(path);
            (abi, interfaces_for, field)
        })
        .collect::<Vec<_>>();
    let required_abis = interface_paths.iter().map(|path| generated_path(path, "Abi")).collect::<Vec<_>>();
    let mut base_identity = quote! { #nc::NcInterfaceId::NONE };
    let mut prefix_field = quote! {
        /// Common identity, query and reference-count slots.
        pub unknown: #nc::IUnknownVTable,
    };
    let mut prefix_value = quote! {
        unknown: #nc::IUnknownVTable {
            interface_id: identity,
            query_interface: Self::query_interface_thunk,
            add_ref: Self::add_ref_thunk,
            release: Self::release_thunk,
        },
    };
    let mut client_base = quote! {};
    let mut storage_base = quote! {};
    let mut storage_base_init = quote! {};
    let mut storage_base_set = quote! {};
    let mut storage_base_query = quote! {};
    if let Some(base) = base {
        let base_abi = generated_path(base, "Abi");
        let base_vtable = generated_path(base, "VTable");
        let base_vtable_for = generated_path(base, "VtblFor");
        let base_client = generated_path(base, "Client");
        let base_interfaces = generated_path(base, "InterfacesFor");
        base_identity = quote! { <#base_abi as #nc::ComInterface>::INTERFACE_ID };
        prefix_field = quote! {
            /// Complete inherited vtable at offset zero; local slots follow it.
            pub base: #base_vtable,
        };
        prefix_value = quote! { base: #base_vtable_for::<T>::with_identity(identity), };
        client_base = quote! {
            impl<M: #nc::ThreadingModel> core::ops::Deref for #client_ident<M> {
                type Target = #base_client<M>;
                fn deref(&self) -> &Self::Target {
                    // SAFETY: Both generated views are transparent pointer words.
                    // The inherited table is a physical prefix of the derived table.
                    // This lends methods only; ownership and checked base projection
                    // still require QueryInterface on the actual derived header.
                    unsafe { &*core::ptr::from_ref(self).cast::<Self::Target>() }
                }
            }

            impl<M: #nc::ThreadingModel> core::ops::DerefMut for #client_ident<M> {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    // SAFETY: Both views are transparent pointer words and the
                    // mutable borrow remains tied to this derived client view.
                    unsafe { &mut *core::ptr::from_mut(self).cast::<Self::Target>() }
                }
            }
        };
        storage_base = quote! {
            /// Base projections share this object's allocation and reference count.
            base: #base_interfaces<T>,
        };
        storage_base_init = quote! { base: #base_interfaces::<T>::__nc_new(), };
        storage_base_set = quote! { self.base.set_object(object); };
        storage_base_query = quote! {
            let projection = self.base.query(interface)?;
            if !projection.is_null() {
                return Ok(projection);
            }
        };
    }
    let additional_storage = additional_interfaces.iter().map(|(_, interfaces_for, field)| {
        quote! {
            #[doc = "Required sibling interface projections sharing this object allocation."]
            #field: #interfaces_for<T>,
        }
    });
    let additional_storage_init = additional_interfaces.iter().map(|(_, interfaces_for, field)| {
        quote! {
            #field: #interfaces_for::<T>::__nc_new(),
        }
    });
    let additional_storage_set = additional_interfaces.iter().map(|(_, _, field)| {
        quote! {
            self.#field.set_object(object);
        }
    });
    let additional_storage_query = additional_interfaces.iter().map(|(_, _, field)| {
        quote! {
            let projection = self.#field.query(interface)?;
            if !projection.is_null() {
                return Ok(projection);
            }
        }
    });

    let method_count = checked_u32_len(methods.len(), trait_ident.span(), "interface method count")?;
    let method_param_counts = methods
        .iter()
        .map(|method| checked_u32_len(method.args.len(), method.ident.span(), "interface method parameter count"))
        .collect::<Result<Vec<_>>>()?;
    for method in &methods {
        checked_u32_len(method.ident.to_string().len(), method.ident.span(), "interface method name length")?;
        for argument in &method.args {
            checked_u32_len(argument.ident.to_string().len(), argument.ident.span(), "interface parameter name length")?;
        }
    }

    let method_abi_descriptors = methods
        .iter()
        .enumerate()
        .map(|(index, method)| method_abi_descriptor_consts(&nc, index, method))
        .collect::<Vec<_>>();
    let method_descriptors = methods.iter().zip(&method_param_counts).enumerate().map(|(index, (method, param_count))| {
        let index = index as u32;
        let method_ident = &method.ident;
        let receiver = match method.receiver {
            AbiReceiver::Shared => quote! { #nc::NC_ABI_RECEIVER_SHARED },
            AbiReceiver::Mutable => quote! { #nc::NC_ABI_RECEIVER_MUTABLE },
        };
        let params_ident = method_params_const_ident(index as usize);
        let return_ident = method_return_const_ident(index as usize);
        quote! {
            #nc::NcMethodDescriptor {
                name: #nc::NcStaticUtf8::from_static(stringify!(#method_ident)),
                receiver: #receiver,
                param_count: #param_count,
                params: Self::#params_ident.as_ptr(),
                returns: Self::#return_ident,
            }
        }
    });
    let signature_bytes_ident = format_ident!("__nc_{abi_ident}_abi_signature_bytes", span = trait_ident.span());
    let signature_static = abi_signature_static_tokens(&signature_bytes_ident, &assigned_signature, &nc);
    let method_count_usize = methods.len();

    // Each Rust method contributes one vtable slot. Non-void `Result<T,
    // NcStatus>` returns lower to `out: *mut T` plus a `NcStatus` function
    // return. `Result<(), NcStatus>` lowers to a status-only ABI function.
    let method_fields = methods.iter().map(|method| {
        let method_ident = &method.ident;
        let docs = raw_vtable_method_docs(trait_ident, method);
        let abi_args = method.args.iter().flat_map(|arg| raw_vtable_arg_params(arg));
        let out_param = match &method.ok.kind {
            AbiReturnKind::Void => quote! {},
            AbiReturnKind::Plain | AbiReturnKind::ComPtr { .. } => {
                let ret = &method.ok.abi_ty;
                quote! { __nc_out: *mut #ret, }
            }
        };
        quote! {
            #(#docs)*
            pub #method_ident: unsafe extern "C" fn(
                this: *mut #abi_ident,
                #(#abi_args,)*
                #out_param
            ) -> #nc::NcStatus,
        }
    });

    // A thunk adapts one C ABI vtable call back into the Rust trait method.
    // It recovers the owning generated storage through the interface header's
    // opaque `object` pointer, then asks `ComAbiObject` for the concrete Rust
    // value through the generated object-wide borrow gate. Panics are
    // intentionally not caught here; the non-unwind C ABI boundary follows
    // Rust's normal panic and ABI unwind rules.
    let thunks = methods.iter().map(|method| {
        let method_ident = &method.ident;
        let thunk_ident = format_ident!("{method_ident}_thunk", span = method_ident.span());
        // Mixed-site names cannot collide with source parameters that happen
        // to use the same spelling after buffer lowering.
        let object_ident = syn::Ident::new("__nc_dispatch_object", proc_macro2::Span::mixed_site());
        let value_ident = syn::Ident::new("__nc_dispatch_value", proc_macro2::Span::mixed_site());
        let params = method.args.iter().flat_map(|arg| raw_vtable_arg_params(arg));
        let arg_conversions = method.args.iter().filter_map(|arg| match &arg.kind {
            AbiArgKind::Message { message_ty } => {
                let ident = &arg.ident;
                Some(quote! {
                    // SAFETY: the caller supplies this concrete message's C record
                    // and keeps its selected borrowed fields live for the call.
                    let #ident = match unsafe { #nc::NcMessage::<#message_ty>::from_raw(#ident) } {
                        Ok(message) => message,
                        Err(status) => return status,
                    };
                })
            }
            AbiArgKind::Plain => {
                let ident = &arg.ident;
                let ty = &arg.rust_ty;
                Some(quote! {
                    // SAFETY: copied wire storage is initialized; restricted fields are validated here.
                    let #ident = match unsafe { #nc::decode_inline::<#ty>(#ident) } {
                        Ok(value) => value, Err(status) => return status,
                    };
                })
            },
            AbiArgKind::CopyFromBufferOffset { element_ty } | AbiArgKind::CopyToBufferOffset { element_ty } => {
                let ident = &arg.ident;
                let offset = buffer_offset_ident(ident);
                let len = buffer_len_ident(ident);
                let ptr = buffer_ptr_ident(ident);
                let values = format_ident!("__nc_{}_values", ident);
                let wrapper = match &arg.kind {
                    AbiArgKind::CopyFromBufferOffset { .. } => quote! { #nc::NcCopyFromBufferOffset::new(#offset, &#values) },
                    _ => quote! { #nc::NcCopyToBufferOffset::new(#offset, &mut #values) },
                };
                Some(quote! {
                    // SAFETY: initialized raw elements are checked before any application slice exists.
                    let mut #values = match unsafe { #nc::decode_value_buffer::<#element_ty>(#ptr, #len) } {
                        Ok(values) => values, Err(status) => return status,
                    };
                    let #ident = #wrapper;
                })
            }
            AbiArgKind::ComRef { interface_ty, mode_ty, optional } => {
                let ident = &arg.ident;
                let interface_ty = interface_ty.as_ref();
                let mode_ty = mode_ty
                    .as_ref()
                    .map(|ty| {
                        let ty = ty.as_ref();
                        quote! { #ty }
                    })
                    .unwrap_or_else(|| quote! { #nc::ThreadLocal });
                let checked = quote! {
                    match unsafe {
                        // SAFETY: The raw thunk checks the call-scoped pointer
                        // against the callee's expected full interface identity.
                        // Null and mismatched pointers fail before dispatch;
                        // live common-prefix validity remains the raw caller's
                        // responsibility for the duration of the call.
                        #nc::ComRef::<#interface_ty, #mode_ty>::__nc_from_raw_borrowed_checked(#ident)
                    } {
                        Ok(value) => value,
                        Err(status) => return status,
                    }
                };
                Some(match optional {
                    true => quote! {
                        // Absence needs no validation; present capabilities use
                        // exactly the same identity checks as required inputs.
                        let #ident = if #ident.is_null() { None } else { Some(#checked) };
                    },
                    false => quote! { let #ident = #checked; },
                })
            }
        });
        let buffer_writeback = method.args.iter().filter_map(|arg| {
            let AbiArgKind::CopyToBufferOffset { .. } = &arg.kind else { return None; };
            let values = format_ident!("__nc_{}_values", arg.ident);
            let ptr = buffer_ptr_ident(&arg.ident);
            Some(quote! {
                let wires = match #nc::encode_value_buffer(&#values) { Ok(wires) => wires, Err(status) => return status };
                // SAFETY: raw output storage has this exact element count. Encode
                // every field before writing so validation failure leaves it untouched.
                if !wires.is_empty() { unsafe { core::ptr::copy_nonoverlapping(wires.as_ptr(), #ptr, wires.len()); } }
            })
        });
        let arg_names = method.args.iter().map(|arg| &arg.ident).collect::<Vec<_>>();
        let out_param = match &method.ok.kind {
            AbiReturnKind::Void => quote! {},
            AbiReturnKind::Plain | AbiReturnKind::ComPtr { .. } => {
                let ret = &method.ok.abi_ty;
                quote! { __nc_out: *mut #ret, }
            }
        };
        let pointer_check = match &method.ok.kind {
            AbiReturnKind::Void => quote! {
                if this.is_null() {
                    return #nc::NC_E_POINTER;
                }
            },
            AbiReturnKind::Plain | AbiReturnKind::ComPtr { .. } => quote! {
                if this.is_null() || __nc_out.is_null() {
                    return #nc::NC_E_POINTER;
                }
            },
        };
        let success_arm = match &method.ok.kind {
            AbiReturnKind::Void => quote! {
                Ok(()) => #nc::NC_OK,
            },
            AbiReturnKind::Plain => quote! {
                Ok(value) => {
                    // SAFETY: Non-void thunks checked `__nc_out` for null before the
                    // Rust method was called.
                    let raw = match #nc::encode_inline(&value) { Ok(raw) => raw, Err(status) => return status };
                    unsafe { *__nc_out = raw; }
                    #nc::NC_OK
                }
            },
            AbiReturnKind::ComPtr { .. } => quote! {
                Ok(value) => {
                    // SAFETY: Non-void thunks checked `__nc_out` for null.
                    // `into_raw` transfers the returned owned COM reference to
                    // the caller.
                    unsafe { *__nc_out = value.into_raw(); }
                    #nc::NC_OK
                }
            },
        };
        let borrow = match method.receiver {
            AbiReceiver::Shared => quote! {
                <T as #nc::ComAbiObject>::borrow_abi_ref(#object_ident)
            },
            AbiReceiver::Mutable => quote! {
                <T as #nc::ComAbiObject>::borrow_abi_mut(#object_ident)
            },
        };
        let invoke = match method.receiver {
            AbiReceiver::Shared => quote! {
                <T as #trait_ident>::#method_ident(&*#value_ident, #(#arg_names),*)
            },
            AbiReceiver::Mutable => quote! {
                <T as #trait_ident>::#method_ident(&mut *#value_ident, #(#arg_names),*)
            },
        };
        quote! {
            /// Validates raw pointers, borrows the concrete provider through its
            /// object-wide gate, invokes its typed method, and lowers the result.
            /// The gate is released before transferring an owned result; errors
            /// are normalized so an unwritten output can never look successful.
            ///
            /// # Safety
            /// The interface belongs to T's live generated storage. Non-null ABI
            /// arguments must satisfy their method's readable/writable contracts.
            unsafe extern "C" fn #thunk_ident(
                this: *mut #abi_ident,
                #(#params,)*
                #out_param
            ) -> #nc::NcStatus {
                #pointer_check

                // SAFETY: `this` was checked non-null above and is a generated
                // interface header for this vtable call.
                let #object_ident = unsafe { (*this).object };
                if #object_ident.is_null() {
                    return #nc::NC_E_POINTER;
                }
                #(#arg_conversions)*
                // SAFETY: The object pointer came from a generated interface
                // header. `ComAbiObject` validates the requested Rust borrow
                // against every call active on the same allocation.
                let mut #value_ident = match unsafe { #borrow } {
                    Ok(value) => value,
                    Err(status) => return status,
                };
                let result = #invoke;
                // Release the object-wide borrow before lowering an owned
                // result into the caller's output slot.
                drop(#value_ident);
                if result.is_ok() { #(#buffer_writeback)* }
                match result {
                    #success_arm
                    // Rust `Result::Err` must map to a negative ABI status.
                    // Normalizing here prevents an invalid `Err(success)` from
                    // making a safe caller read an unwritten out slot.
                    Err(status) => status.normalize_error(),
                }
            }
        }
    });

    // The vtable is a `const` associated with `IFooVtblFor<T>`, where `T` is the
    // implementing object. Each object/interface pair therefore gets thunks that
    // call the correct concrete Rust type without dynamic Rust dispatch.
    let vtable_entries = methods.iter().map(|method| {
        let method_ident = &method.ident;
        let thunk_ident = format_ident!("{method_ident}_thunk", span = method_ident.span());
        quote! { #method_ident: Self::#thunk_ident }
    });

    // Generate one safe lowering for each source method. Both owning `ComPtr`
    // and borrowed `ComRef` handles lend this client view, and a derived view
    // dereferences through its generated base views for inherited calls.
    let client_methods = methods.iter().map(|method| {
        let user_docs = &method.doc_attrs;
        let generated_docs = safe_method_docs(trait_ident, method);
        let body = safe_client_method_body(&nc, method);
        quote! { #(#user_docs)* #(#generated_docs)* pub #body }
    });

    let abi = quote! {
        #input

        #signature_static

        // The generated interface header is the pointer type stored in
        // `ComPtr<IFooAbi>`. The first field must remain the vtable pointer so
        // `iunknown()` can reinterpret the header as `IUnknown` for common
        // lifecycle operations. The opaque object pointer lets every interface
        // header point back to the same generated allocation.
        #[repr(C)]
        #[doc = concat!("C-compatible ABI header for [`", stringify!(#trait_ident), "`].")]
        #vis struct #abi_ident {
            #[doc = concat!("Pointer to the [`", stringify!(#vtable_ident), "`] used by this interface header.")]
            pub vtable: *const #vtable_ident,
            #[doc = "Opaque pointer to the owning COM object storage."]
            pub object: *mut core::ffi::c_void,
        }

        // Vtables start with the `IUnknown` function table, then append the
        // interface-specific methods in source order.
        #[repr(C)]
        #[doc = concat!("C-compatible vtable for [`", stringify!(#trait_ident), "`].")]
        #vis struct #vtable_ident {
            #prefix_field
            #(#method_fields)*
        }

        // SAFETY: `com_interface` generates `#abi_ident` as a `repr(C)` header
        // with the same vtable/object prefix as `IUnknown`, generates
        // `#vtable_ident` with `IUnknownVTable` first, and combines the derived
        // GUID with the IDL-generator-assigned signature. `iunknown` performs
        // only the shared-prefix pointer cast.
        unsafe impl #nc::ComInterface for #abi_ident {
            const IID: #nc::NcGuid = #guid;
            const INTERFACE_ID: #nc::NcInterfaceId =
                #nc::NcInterfaceId::new(#guid, #abi_ident::ABI_SIGNATURE);
            type VTable = #vtable_ident;
            type Threading = #threading;


            // All generated interface headers share the same prefix as
            // `IUnknown`, so this is the canonical route for AddRef, Release,
            // and QueryInterface from any interface pointer.
            fn iunknown(this: *mut Self) -> *mut #nc::IUnknown {
                this.cast::<#nc::IUnknown>()
            }
        }

        #factory_marker

        // The view only lends method lookup. It has no constructor, ownership
        // conversions or Clone/Copy implementation and cannot outlive its lender.
        #[repr(transparent)]
        #[doc = concat!("Borrowed static method view for [`", stringify!(#trait_ident), "`].")]
        #vis struct #client_ident<M: #nc::ThreadingModel = #nc::ThreadLocal> {
            /// Certified transparent storage supplied only by a live COM pointer borrow.
            __nc_pointer: #nc::NcClientPointer<#abi_ident, M>,
        }

        // SAFETY: The generated view is transparent pointer storage; all safe
        // methods make synchronous ABI calls and never transfer its borrowed pointer.
        unsafe impl #nc::ComClientInterface for #abi_ident {
            type Client<M: #nc::ThreadingModel> = #client_ident<M>;
        }

        impl<M: #nc::ThreadingModel> #client_ident<M> {
            #(#client_methods)*
        }

        #client_base

        impl #abi_ident {
            #[doc = "Canonical validation form of this interface's typed ABI contract."]
            pub const ABI_SIGNATURE: #nc::NcAbiSignature =
                #nc::NcAbiSignature::from_static(&#signature_bytes_ident);

            #(#method_abi_descriptors)*

            #[doc = "Static descriptors for this interface's ABI methods."]
            pub const METHOD_DESCRIPTORS: [#nc::NcMethodDescriptor; #method_count_usize] = [
                #(#method_descriptors,)*
            ];

            #[doc = "Static ABI descriptor for this interface."]
            pub const INTERFACE_DESCRIPTOR: #nc::NcInterfaceDescriptor = #nc::NcInterfaceDescriptor {
                interface_id: <Self as #nc::ComInterface>::IID,
                qualified_name: #nc::NcStaticUtf8::from_static(#qualified_name),
                abi_signature: Self::ABI_SIGNATURE,
                base: #base_identity,
                method_count: #method_count,
                methods: Self::METHOD_DESCRIPTORS.as_ptr(),
            };

            /// This projection followed by every required base and sibling.
            /// Object storage deduplicates identities and rejects ABI conflicts.
            pub const INTERFACE_CHAIN: [&'static #nc::NcInterfaceDescriptor; 1 #(+ #required_abis::INTERFACE_CHAIN.len())*] =
                #nc::__nc_interface_chain(&Self::INTERFACE_DESCRIPTOR, &[#(&#required_abis::INTERFACE_CHAIN,)*]);
        }

    };
    let provider = quote! {
        // Header storage is recursive only over the statically known base
        // chain. It is embedded directly in the one object allocation.
        #[doc(hidden)]
        /// Interface projections embedded in one generated provider allocation.
        #vis struct #interfaces_ident<T> {
            /// Projection whose header advertises this exact interface contract.
            own: #abi_ident,
            #storage_base
            #(#additional_storage)*
            /// Associates the thunk implementor without owning a second value.
            implementor: core::marker::PhantomData<fn() -> T>,
        }

        impl<T: #trait_ident + #nc::ComAbiObject> #interfaces_ident<T> {
            /// Creates unpublished headers; the object installs its back-pointer.
            pub const fn __nc_new() -> Self {
                Self {
                    own: #abi_ident { vtable: &#vtable_for_ident::<T>::VTABLE, object: core::ptr::null_mut() },
                    #storage_base_init
                    #(#additional_storage_init)*
                    implementor: core::marker::PhantomData,
                }
            }
        }

        impl<T> #interfaces_ident<T> {
            /// Patches every projection before the object is published.
            pub fn set_object(&mut self, object: *mut core::ffi::c_void) {
                self.own.object = object;
                #storage_base_set
                #(#additional_storage_set)*
            }

            /// Finds a checked projection without changing the reference count.
            pub fn query(&self, interface: &#nc::NcInterfaceId) -> Result<*mut core::ffi::c_void, #nc::NcStatus> {
                if interface.iid == <#abi_ident as #nc::ComInterface>::IID {
                    if *interface != <#abi_ident as #nc::ComInterface>::INTERFACE_ID {
                        return Err(#nc::NC_E_ABI_VERSION);
                    }
                    return Ok(core::ptr::from_ref(&self.own).cast_mut().cast());
                }
                #storage_base_query
                #(#additional_storage_query)*
                Ok(core::ptr::null_mut())
            }
        }

        #[doc(hidden)]
        /// Statically typed vtable and lifecycle adapters for one provider type.
        #vis struct #vtable_for_ident<T>(
            /// Associates dispatch with T without allocating or owning a value.
            core::marker::PhantomData<T>
        );

        impl<T> #vtable_for_ident<T>
        where
            T: #trait_ident + #nc::ComAbiObject,
        {
            /// Table for a standalone projection of this interface.
            pub const VTABLE: #vtable_ident = Self::with_identity(&<#abi_ident as #nc::ComInterface>::INTERFACE_ID);

            /// Builds an inherited prefix advertising the actual outer projection.
            pub const fn with_identity(identity: &'static #nc::NcInterfaceId) -> #vtable_ident {
                #vtable_ident {
                    #prefix_value
                    #(#vtable_entries,)*
                }
            }

            /// Routes this projection to the allocation-wide interface lookup.
            ///
            /// # Safety
            /// The live header belongs to T; identity and output storage satisfy
            /// the QueryInterface contract. The owning allocation validates them.
            unsafe extern "C" fn query_interface_thunk(
                this: *mut #nc::IUnknown,
                interface_id: *const #nc::NcInterfaceId,
                out: *mut *mut core::ffi::c_void,
            ) -> #nc::NcStatus {
                if this.is_null() {
                    return #nc::NC_E_POINTER;
                }
                // SAFETY: Non-null `this` is a generated interface header and
                // carries the storage back-pointer.
                let object = unsafe { (*this).object };
                // SAFETY: Delegates to the generated object object, which
                // validates `object`, `interface_id`, and `out` before
                // dereferencing.
                unsafe {
                    <T as #nc::ComAbiObject>::query_interface_from_abi_object(
                        object,
                        interface_id,
                        out,
                    )
                }
            }

            /// Retains the common allocation through this interface projection.
            ///
            /// # Safety
            /// Non-null this belongs to T's live generated object storage.
            unsafe extern "C" fn add_ref_thunk(this: *mut #nc::IUnknown) -> u32 {
                if this.is_null() {
                    return 0;
                }
                // SAFETY: Non-null `this` is a generated interface header and
                // carries the storage back-pointer.
                let object = unsafe { (*this).object };
                // SAFETY: Delegates to generated storage refcount handling.
                unsafe { <T as #nc::ComAbiObject>::add_ref_from_abi_object(object) }
            }

            /// Releases the common allocation through this interface projection.
            ///
            /// # Safety
            /// Non-null this transfers one live reference from T's object storage.
            unsafe extern "C" fn release_thunk(this: *mut #nc::IUnknown) -> u32 {
                if this.is_null() {
                    return 0;
                }
                // SAFETY: Non-null `this` is a generated interface header and
                // carries the storage back-pointer.
                let object = unsafe { (*this).object };
                // SAFETY: Delegates to generated storage refcount handling.
                unsafe { <T as #nc::ComAbiObject>::release_from_abi_object(object) }
            }

            #(#thunks)*
        }
    };
    Ok(InterfaceExpansion { abi, provider })
}

/// Checks names shared by ordinary interfaces and callable object projections.
pub(crate) fn validate_callable_methods(input: &ItemTrait, methods: &[AbiMethod], errors: &mut Option<syn::Error>) {
    validate_generated_method_identifiers(methods, errors);
    // Vtable prefix fields are physical ABI members, so a local method cannot
    // use their name. Report the source method instead of duplicate generated
    // fields.
    let prefix_name = if input.supertraits.is_empty() { "unknown" } else { "base" };
    for method in methods {
        if method.ident == prefix_name {
            push_error(
                errors,
                syn::Error::new(
                    method.ident.span(),
                    format!("COM method conflicts with the generated `{prefix_name}` vtable prefix"),
                ),
            );
        }
    }
}

/// Emits the sole safe Rust-to-ABI body for one generated interface method.
///
/// `ComPtr` and `ComRef` both dereference to the client view containing this
/// body. Derived client views also dereference to their base client, so one
/// emitted implementation serves owned, borrowed, and inherited calls.
fn safe_client_method_body(nc: &syn::Path, method: &AbiMethod) -> proc_macro2::TokenStream {
    let method_ident = &method.ident;
    let args = method.args.iter().map(|arg| {
        let ident = &arg.ident;
        let ty = &arg.rust_ty;
        quote! { #ident: #ty }
    });
    let arg_preparations = method.args.iter().filter_map(|arg| safe_arg_preparation(nc, arg));
    let arg_names = method.args.iter().flat_map(safe_vtable_call_args).collect::<Vec<_>>();
    let buffer_acquisition = method.args.iter().filter_map(|arg| {
        let AbiArgKind::CopyToBufferOffset { element_ty } = &arg.kind else { return None; };
        let ident = &arg.ident;
        let wires = format_ident!("__nc_{}_wire_buffer", ident);
        Some(quote! {
            // SAFETY: successful foreign calls leave all raw output elements initialized.
            let acquired = unsafe { #nc::decode_value_buffer::<#element_ty>(#wires.as_ptr(), #wires.len() as u64) }?;
            #ident.as_mut_slice().copy_from_slice(&acquired);
        })
    }).collect::<Vec<_>>();
    let rust_ret = &method.ok.rust_ty;
    let receiver = safe_receiver(method.receiver);
    match &method.ok.kind {
        AbiReturnKind::Void => quote! {
            // Forwards the canonical safe client call to the raw status-only
            // vtable slot.
            fn #method_ident(#receiver, #(#args),*) -> Result<#rust_ret, #nc::NcStatus> {
                #(#arg_preparations)*
                let this = self.__nc_pointer.as_mut_ptr();
                // SAFETY: The client view borrows a live interface reference.
                // The raw vtable method receives pointers valid for this call
                // and returns only an `NcStatus`.
                let status = unsafe {
                    ((*(*this).vtable).#method_ident)(
                        this,
                        #(#arg_names,)*
                    )
                };
                if status.is_ok() {
                    #(#buffer_acquisition)*
                    Ok(())
                } else {
                    Err(status)
                }
            }
        },
        AbiReturnKind::Plain | AbiReturnKind::ComPtr { .. } => {
            let abi_ret = &method.ok.abi_ty;
            let success_return = match &method.ok.kind {
                AbiReturnKind::Plain => quote! {
                    // SAFETY: The raw method returned success, which is the
                    // ABI contract that it initialized the out slot.
                    unsafe { #nc::decode_inline::<#rust_ret>(__nc_out.assume_init()) }
                },
                AbiReturnKind::ComPtr { .. } => quote! {
                    // SAFETY: The raw method returned success, which is the
                    // ABI contract that it initialized the out slot.
                    let __nc_out = unsafe { __nc_out.assume_init() };
                    // SAFETY: Successful COM pointer returns transfer one
                    // owned reference to the caller.
                    unsafe { <#rust_ret>::__nc_from_raw_owned_checked(__nc_out) }
                },
                AbiReturnKind::Void => unreachable!(),
            };
            quote! {
                // Uses a local output slot that is read only after ABI success.
                fn #method_ident(#receiver, #(#args),*) -> Result<#rust_ret, #nc::NcStatus> {
                    #(#arg_preparations)*
                    let mut __nc_out = core::mem::MaybeUninit::<#abi_ret>::uninit();
                    let this = self.__nc_pointer.as_mut_ptr();
                    // SAFETY: The client view borrows a live interface reference.
                    // The out pointer targets local storage that remains valid
                    // until the call returns.
                    let status = unsafe {
                        ((*(*this).vtable).#method_ident)(
                            this,
                            #(#arg_names,)*
                            __nc_out.as_mut_ptr(),
                        )
                    };
                    if status.is_ok() {
                        #(#buffer_acquisition)*
                        #success_return
                    } else {
                        Err(status)
                    }
                }
            }
        }
    }
}

/// Emits the safe Rust receiver corresponding to one validated COM method.
fn safe_receiver(receiver: AbiReceiver) -> proc_macro2::TokenStream {
    match receiver {
        AbiReceiver::Shared => quote! { &self },
        AbiReceiver::Mutable => quote! { &mut self },
    }
}

/// Emits private parameter and return descriptor constants for one method.
fn method_abi_descriptor_consts(nc: &syn::Path, index: usize, method: &AbiMethod) -> proc_macro2::TokenStream {
    let params_ident = method_params_const_ident(index);
    let return_ident = method_return_const_ident(index);
    let params = method.args.iter().map(|arg| method_param_descriptor(nc, arg));
    let param_count = method.args.len();
    let returns = method_return_descriptor(nc, method);
    quote! {
        #[doc(hidden)]
        pub const #params_ident: [#nc::NcMethodParamDescriptor; #param_count] = [
            #(#params,)*
        ];

        #[doc(hidden)]
        pub const #return_ident: #nc::NcMethodReturnDescriptor = #returns;
    }
}

/// Emits the physical descriptor for one lowered parameter.
fn method_param_descriptor(nc: &syn::Path, arg: &crate::model::AbiArg) -> proc_macro2::TokenStream {
    let ident = &arg.ident;
    let passing = arg_passing_descriptor(nc, arg);
    let ty = arg_type_descriptor(nc, arg);
    quote! {
        #nc::NcMethodParamDescriptor {
            name: #nc::NcStaticUtf8::from_static(stringify!(#ident)),
            passing: #passing,
            ty: #ty,
        }
    }
}

/// Emits the physical descriptor for one successful return contract.
fn method_return_descriptor(nc: &syn::Path, method: &AbiMethod) -> proc_macro2::TokenStream {
    let passing = return_passing_descriptor(nc, method);
    let ty = return_type_descriptor(nc, method);
    quote! {
        #nc::NcMethodReturnDescriptor {
            passing: #passing,
            ty: #ty,
        }
    }
}

/// Maps an argument kind to its C-shaped passing descriptor.
fn arg_passing_descriptor(nc: &syn::Path, arg: &crate::model::AbiArg) -> proc_macro2::TokenStream {
    match &arg.kind {
        AbiArgKind::Message { .. } => quote! {
            #nc::NcAbiPassingDescriptor::from_abi_passing(#nc::NcAbiPassing::Message)
        },
        AbiArgKind::Plain => quote! {
            #nc::NcAbiPassingDescriptor::from_abi_passing(#nc::NcAbiPassing::Value)
        },
        AbiArgKind::CopyFromBufferOffset { .. } => quote! {
            #nc::NcAbiPassingDescriptor::from_abi_passing(#nc::NcAbiPassing::CopyFromBufferOffset)
        },
        AbiArgKind::CopyToBufferOffset { .. } => quote! {
            #nc::NcAbiPassingDescriptor::from_abi_passing(#nc::NcAbiPassing::CopyToBufferOffset)
        },
        AbiArgKind::ComRef { mode_ty, optional, .. } => {
            let mode = threading_mode_expr(nc, mode_ty.as_deref());
            let passing = if *optional { quote! { OptionalComRef } } else { quote! { ComRef } };
            quote! {
                #nc::NcAbiPassingDescriptor::from_abi_passing(#nc::NcAbiPassing::#passing {
                    mode: #mode,
                })
            }
        }
    }
}

/// Maps an argument's source type to its canonical ABI type descriptor.
fn arg_type_descriptor(nc: &syn::Path, arg: &crate::model::AbiArg) -> proc_macro2::TokenStream {
    match &arg.kind {
        AbiArgKind::Message { message_ty } => quote! {
            #nc::NcAbiTypeDescriptor::from_abi_type(#nc::NcAbiType::Aggregate(
                <#message_ty as #nc::ComMessage>::SIGNATURE
            ))
        },
        AbiArgKind::Plain => {
            let ty = &arg.rust_ty;
            quote! {
                #nc::NcAbiTypeDescriptor::from_abi_type(#nc::__nc_abi_type::<#ty>())
            }
        }
        AbiArgKind::CopyFromBufferOffset { element_ty } | AbiArgKind::CopyToBufferOffset { element_ty } => {
            let element_ty = element_ty.as_ref();
            quote! {
                #nc::NcAbiTypeDescriptor::from_abi_type(#nc::__nc_abi_plain_type::<#element_ty>())
            }
        }
        AbiArgKind::ComRef { interface_ty, .. } => {
            let interface_ty = interface_ty.as_ref();
            quote! {
                #nc::NcAbiTypeDescriptor::from_interface(
                    <#interface_ty as #nc::ComInterface>::INTERFACE_ID,
                )
            }
        }
    }
}

/// Maps a return kind to its C-shaped output passing descriptor.
fn return_passing_descriptor(nc: &syn::Path, method: &AbiMethod) -> proc_macro2::TokenStream {
    match &method.ok.kind {
        AbiReturnKind::Void => quote! {
            #nc::NcAbiPassingDescriptor::from_abi_passing(#nc::NcAbiPassing::Void)
        },
        AbiReturnKind::Plain => quote! {
            #nc::NcAbiPassingDescriptor::from_abi_passing(#nc::NcAbiPassing::Out)
        },
        AbiReturnKind::ComPtr { mode_ty, .. } => {
            let mode = threading_mode_expr(nc, mode_ty.as_deref());
            quote! {
                #nc::NcAbiPassingDescriptor::from_abi_passing(#nc::NcAbiPassing::ComPtrOut {
                    mode: #mode,
                })
            }
        }
    }
}

/// Maps a successful return type to its canonical ABI type descriptor.
fn return_type_descriptor(nc: &syn::Path, method: &AbiMethod) -> proc_macro2::TokenStream {
    match &method.ok.kind {
        AbiReturnKind::Void => quote! {
            #nc::NcAbiTypeDescriptor::from_abi_type(#nc::NcAbiType::Void)
        },
        AbiReturnKind::Plain => {
            let ty = &method.ok.rust_ty;
            quote! {
                #nc::NcAbiTypeDescriptor::from_abi_type(#nc::__nc_abi_type::<#ty>())
            }
        }
        AbiReturnKind::ComPtr { interface_ty, .. } => {
            let interface_ty = interface_ty.as_ref();
            quote! {
                #nc::NcAbiTypeDescriptor::from_interface(
                    <#interface_ty as #nc::ComInterface>::INTERFACE_ID,
                )
            }
        }
    }
}

/// Builds the private parameter-array constant name for a method index.
fn method_params_const_ident(index: usize) -> syn::Ident {
    format_ident!("__NC_METHOD_{index}_PARAMS")
}

/// Builds the private return-descriptor constant name for a method index.
fn method_return_const_ident(index: usize) -> syn::Ident {
    format_ident!("__NC_METHOD_{index}_RETURN")
}

/// Expands one logical argument into its raw vtable parameter list.
fn raw_vtable_arg_params(arg: &crate::model::AbiArg) -> Vec<proc_macro2::TokenStream> {
    let ident = &arg.ident;
    match &arg.kind {
        AbiArgKind::Plain | AbiArgKind::Message { .. } | AbiArgKind::ComRef { .. } => {
            let ty = &arg.abi_ty;
            vec![quote! { #ident: #ty }]
        }
        AbiArgKind::CopyFromBufferOffset { .. } => {
            let offset_ident = buffer_offset_ident(ident);
            let len_ident = buffer_len_ident(ident);
            let ptr_ident = buffer_ptr_ident(ident);
            vec![
                quote! { #offset_ident: u64 },
                quote! { #len_ident: u64 },
                { let ty = &arg.abi_ty; quote! { #ptr_ident: #ty } },
            ]
        }
        AbiArgKind::CopyToBufferOffset { .. } => {
            let offset_ident = buffer_offset_ident(ident);
            let len_ident = buffer_len_ident(ident);
            let ptr_ident = buffer_ptr_ident(ident);
            vec![
                quote! { #offset_ident: u64 },
                quote! { #len_ident: u64 },
                { let ty = &arg.abi_ty; quote! { #ptr_ident: #ty } },
            ]
        }
    }
}

/// Emits pre-call pointer/bounds extraction required by a safe wrapper argument.
fn safe_arg_preparation(nc: &syn::Path, arg: &crate::model::AbiArg) -> Option<proc_macro2::TokenStream> {
    match &arg.kind {
        AbiArgKind::Plain => {
            let ident = &arg.ident;
            Some(quote! { let #ident = #nc::encode_inline(&#ident)?; })
        }
        AbiArgKind::CopyToBufferOffset { .. } | AbiArgKind::CopyFromBufferOffset { .. } => {
            let ident = &arg.ident;
            let wires = format_ident!("__nc_{}_wire_buffer", ident);
            let slice = match &arg.kind { AbiArgKind::CopyToBufferOffset { .. } => quote! { #ident.as_mut_slice() }, _ => quote! { #ident.as_slice() } };
            Some(quote! { let mut #ident = #ident; let mut #wires = #nc::encode_value_buffer(#slice)?; })
        }
        AbiArgKind::Message { .. } | AbiArgKind::ComRef { .. } => None,
    }
}

/// Emits the raw values passed for one source-level safe argument.
fn safe_vtable_call_args(arg: &crate::model::AbiArg) -> Vec<proc_macro2::TokenStream> {
    let ident = &arg.ident;
    let wires = format_ident!("__nc_{}_wire_buffer", ident);
    match &arg.kind {
        AbiArgKind::Message { .. } => vec![quote! { #ident.as_ptr() }],
        AbiArgKind::Plain => vec![quote! { #ident }],
        AbiArgKind::ComRef { optional: true, .. } => vec![quote! { #ident.map_or(::core::ptr::null_mut(), |value| value.as_mut_ptr()) }],
        AbiArgKind::ComRef { optional: false, .. } => vec![quote! { #ident.as_mut_ptr() }],
        AbiArgKind::CopyFromBufferOffset { .. } => vec![quote! { #ident.offset() }, quote! { #ident.__nc_len_u64()? }, quote! { #wires.as_ptr() }],
        AbiArgKind::CopyToBufferOffset { .. } => vec![quote! { #ident.offset() }, quote! { #ident.__nc_len_u64()? }, quote! { #wires.as_mut_ptr() }],
    }
}

/// Builds the generated offset parameter name for a bounded buffer.
fn buffer_offset_ident(ident: &syn::Ident) -> syn::Ident {
    format_ident!("__nc_{ident}_offset", span = ident.span())
}

/// Builds the generated length parameter name for a bounded buffer.
fn buffer_len_ident(ident: &syn::Ident) -> syn::Ident {
    format_ident!("__nc_{ident}_len", span = ident.span())
}

/// Builds the generated data-pointer parameter name for a bounded buffer.
fn buffer_ptr_ident(ident: &syn::Ident) -> syn::Ident {
    format_ident!("__nc_{ident}_ptr", span = ident.span())
}

/// Rejects source names that collide after helper and raw-parameter generation.
fn validate_generated_method_identifiers(methods: &[AbiMethod], errors: &mut Option<syn::Error>) {
    for method in methods {
        let method_name = method.ident.to_string();
        if matches!(method_name.as_str(), "query_interface" | "add_ref" | "release") {
            push_error(
                errors,
                syn::Error::new(
                    method.ident.span(),
                    format!("COM method `{method_name}` conflicts with the generated IUnknown lifecycle thunk"),
                ),
            );
        }

        let has_out = !matches!(&method.ok.kind, AbiReturnKind::Void);
        let mut raw_names: Vec<(String, &syn::Ident, bool)> = Vec::new();

        for arg in &method.args {
            let arg_name = arg.ident.to_string();
            let fixed_collision = match arg_name.as_str() {
                "this" => Some("the generated client receiver pointer"),
                "object" if matches!(&arg.kind, AbiArgKind::Plain | AbiArgKind::Message { .. } | AbiArgKind::ComRef { .. }) => {
                    Some("the generated ABI object back-pointer local")
                }
                "__nc_value" => Some("the generated Rust implementation pointer local"),
                "__nc_out" if has_out => Some("the generated non-void output slot"),
                _ => None,
            };
            if let Some(generated) = fixed_collision {
                push_error(
                    errors,
                    syn::Error::new(arg.ident.span(), format!("COM method parameter `{arg_name}` conflicts with {generated}")),
                );
            }

            for (raw_name, derived) in raw_parameter_names(arg) {
                if let Some((_, previous_ident, previous_derived)) = raw_names.iter().find(|(previous_name, _, _)| previous_name == &raw_name) {
                    let current_origin = if derived {
                        format!("buffer parameter `{}` generates", arg.ident)
                    } else {
                        format!("source parameter `{}` uses", arg.ident)
                    };
                    let previous_origin = if *previous_derived {
                        format!("buffer parameter `{previous_ident}`")
                    } else {
                        format!("source parameter `{previous_ident}`")
                    };
                    push_error(
                        errors,
                        syn::Error::new(
                            arg.ident.span(),
                            format!("{current_origin} raw ABI parameter `{raw_name}`, which conflicts with {previous_origin}"),
                        ),
                    );
                } else {
                    raw_names.push((raw_name, &arg.ident, derived));
                }
            }
        }
    }
}

/// Returns generated raw parameter names and whether they are reserved helpers.
fn raw_parameter_names(arg: &crate::model::AbiArg) -> Vec<(String, bool)> {
    let ident = &arg.ident;
    match &arg.kind {
        AbiArgKind::Plain | AbiArgKind::Message { .. } | AbiArgKind::ComRef { .. } => vec![(ident.to_string(), false)],
        AbiArgKind::CopyFromBufferOffset { .. } | AbiArgKind::CopyToBufferOffset { .. } => vec![
            (buffer_offset_ident(ident).to_string(), true),
            (buffer_len_ident(ident).to_string(), true),
            (buffer_ptr_ident(ident).to_string(), true),
        ],
    }
}

/// Emits the logical threading mode for an optional pointer marker type.
fn threading_mode_expr(nc: &syn::Path, mode_ty: Option<&syn::Type>) -> proc_macro2::TokenStream {
    match mode_ty {
        Some(mode_ty) => quote! { <#mode_ty as #nc::ThreadingModel>::ABI_THREADING_MODE },
        None => quote! { <#nc::ThreadLocal as #nc::ThreadingModel>::ABI_THREADING_MODE },
    }
}

/// Preserves a base's module path while selecting one generated companion type.
fn generated_path(path: &syn::Path, suffix: &str) -> syn::Path {
    let mut generated = path.clone();
    let last = generated.segments.last_mut().expect("trait paths have a final segment");
    last.ident = format_ident!("{}{suffix}", last.ident);
    generated
}
