//! Generation of interface headers, vtables, thunks, wrappers, and ABI metadata.
//!
//! A validated trait is lowered into one `repr(C)` interface header and vtable,
//! per-implementor thunk table, smart-pointer forwarding methods, and immutable
//! method descriptors. All generated call paths share the normalized lowering
//! model so pointer ownership, buffer bounds, docs, and ABI hashes stay aligned.

use quote::{format_ident, quote};
use syn::{ItemTrait, Result, TypeParamBound, TraitBoundModifier, PathArguments};

use crate::abi_method::{parse_abi_trait_item, raw_vtable_method_docs, safe_method_docs, validate_interface_trait};
use crate::args::InterfaceArgs;
use crate::model::{AbiArgKind, AbiMethod, AbiReceiver, AbiReturnKind};
use crate::util::{abi_hash_prefix_tokens, checked_u32_len, collect_syn_results, interface_field, interface_guid, push_error, runtime_crate_path};

/// Expands one validated interface trait into its complete Rust and C ABI surface.
pub(crate) fn expand_com_interface(args: InterfaceArgs, input: ItemTrait) -> Result<proc_macro2::TokenStream> {
    let mut errors = None;
    validate_interface_trait(&input, &mut errors);

    // Every item on an ordinary interface is a callable vtable method. Class
    // contracts use the lowered entrypoint below after removing their single
    // constructor declaration from this list.
    let methods = match collect_syn_results(input.items.iter().map(parse_abi_trait_item)) {
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
/// Class contracts call this after parsing their constructor separately. The
/// emitted source trait remains intact, while only `methods` become vtable
/// slots, descriptors, hashes, thunks, and safe client methods.
pub(crate) fn expand_com_interface_with_methods(args: InterfaceArgs, input: ItemTrait, methods: Vec<AbiMethod>) -> Result<proc_macro2::TokenStream> {
    // Generated names are deliberately mechanical and predictable. A trait
    // named `IFoo` produces `IFooAbi`, `IFooVTable`, `IFooVtblFor<T>`, and
    // `IFooComPtrExt`.
    let nc = runtime_crate_path(args.runtime_crate.as_ref());
    let trait_ident = &input.ident;
    let vis = &input.vis;
    let abi_ident = format_ident!("{trait_ident}Abi");
    let vtable_ident = format_ident!("{trait_ident}VTable");
    let vtable_for_ident = format_ident!("{trait_ident}VtblFor");
    let ext_ident = format_ident!("{trait_ident}ComPtrExt");
    let client_ident = format_ident!("{trait_ident}Client");
    let interfaces_ident = format_ident!("{trait_ident}InterfacesFor");
    let qualified_name = args.qualified_name;
    let schema_iid = neocogi_com_core::guid_from_qualified_name(&qualified_name.value());
    let guid = interface_guid(args.guid, &qualified_name, &nc)?;
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
                Ok(bound.path.clone())
            }
            _ => Err(syn::Error::new_spanned(bound, "expected a nongeneric generated COM interface")),
        })
        .collect::<Result<Vec<_>>>()?;
    let base = interface_paths.first();
    // Callable class contracts may require more than the one physical vtable
    // base. Each later supertrait is stored and queried as a sibling COM
    // projection. Ordinary interfaces reject such declarations before this
    // shared generator is entered.
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
    let mut base_identity = quote! { #nc::NcInterfaceId::NONE };
    let mut base_chain = quote! { &[] };
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
        base_chain = quote! { &#base_abi::INTERFACE_CHAIN };
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
            /// Base projections share this class's allocation and reference count.
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
            #[doc = "Required sibling interface projections sharing this class allocation."]
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

    let method_hash_parts = methods.iter().map(|method| method_dynamic_hash_parts(&nc, method)).collect::<Vec<_>>();
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
    let method_names = methods.iter().map(|method| method.ident.to_string()).collect::<Vec<_>>();
    let mut schema_parts = vec![neocogi_com_core::NcAbiHashPart::Interface { iid: schema_iid }];
    if base.is_some() {
        schema_parts.push(neocogi_com_core::NcAbiHashPart::BaseInterface {
            interface: neocogi_com_core::NcInterfaceId::NONE,
        });
    }
    for ((index, method), method_name) in methods.iter().enumerate().zip(&method_names) {
        schema_parts.push(neocogi_com_core::NcAbiHashPart::Method { index: index as u32, name: method_name });
        let receiver = match method.receiver {
            AbiReceiver::Shared => neocogi_com_core::NcAbiReceiver::Shared,
            AbiReceiver::Mutable => neocogi_com_core::NcAbiReceiver::Mutable,
        };
        schema_parts.push(neocogi_com_core::NcAbiHashPart::Receiver { receiver });
        schema_parts.extend(method.args.iter().enumerate().map(|(arg_index, _)| neocogi_com_core::NcAbiHashPart::Arg {
            index: arg_index as u32,
            passing: neocogi_com_core::NcAbiPassing::Void,
            ty: neocogi_com_core::NcAbiType::Void,
        }));
        schema_parts.push(neocogi_com_core::NcAbiHashPart::Return {
            passing: neocogi_com_core::NcAbiPassing::Void,
            ty: neocogi_com_core::NcAbiType::Void,
        });
        schema_parts.push(neocogi_com_core::NcAbiHashPart::Status);
    }
    let hash_prefix = abi_hash_prefix_tokens(&schema_parts, &nc);

    let mut interface_hash_parts = Vec::new();
    if base.is_some() {
        interface_hash_parts.push(quote! { #nc::NcAbiHashPart::BaseInterface { interface: #base_identity } });
    }
    for parts in &method_hash_parts {
        interface_hash_parts.extend(parts.iter().cloned());
    }
    let method_count_usize = methods.len();

    // Each Rust method contributes one vtable slot. Non-void `Result<T,
    // NcStatus>` returns lower to `out: *mut T` plus a `NcStatus` function
    // return. `Result<(), NcStatus>` lowers to a status-only ABI function.
    let method_fields = methods.iter().map(|method| {
        let method_ident = &method.ident;
        let docs = raw_vtable_method_docs(trait_ident, method);
        let abi_args = method.args.iter().flat_map(raw_vtable_arg_params);
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
        let thunk_ident = format_ident!("{method_ident}_thunk");
        // Mixed-site names cannot collide with source parameters that happen
        // to use the same spelling after buffer lowering.
        let object_ident = syn::Ident::new("__nc_dispatch_object", proc_macro2::Span::mixed_site());
        let value_ident = syn::Ident::new("__nc_dispatch_value", proc_macro2::Span::mixed_site());
        let params = method.args.iter().flat_map(raw_vtable_arg_params);
        let arg_conversions = method.args.iter().filter_map(|arg| match &arg.kind {
            AbiArgKind::Plain => None,
            AbiArgKind::CopyFromBufferOffset { element_ty } => {
                let ident = &arg.ident;
                let element_ty = element_ty.as_ref();
                let offset_ident = buffer_offset_ident(ident);
                let len_ident = buffer_len_ident(ident);
                let ptr_ident = buffer_ptr_ident(ident);
                Some(quote! {
                    let #ident = match unsafe {
                        // SAFETY: The raw ABI caller owns the buffer contract.
                        // This conversion validates null and representable length
                        // before the Rust trait method receives the safe wrapper.
                        #nc::NcCopyFromBufferOffset::<#element_ty>::__nc_from_raw_parts(
                            #offset_ident,
                            #ptr_ident,
                            #len_ident,
                        )
                    } {
                        Ok(value) => value,
                        Err(status) => return status.normalize_error(),
                    };
                })
            }
            AbiArgKind::CopyToBufferOffset { element_ty } => {
                let ident = &arg.ident;
                let element_ty = element_ty.as_ref();
                let offset_ident = buffer_offset_ident(ident);
                let len_ident = buffer_len_ident(ident);
                let ptr_ident = buffer_ptr_ident(ident);
                Some(quote! {
                    let #ident = match unsafe {
                        // SAFETY: The raw ABI caller owns the buffer contract.
                        // This conversion validates null and representable length
                        // before the Rust trait method receives the safe wrapper.
                        #nc::NcCopyToBufferOffset::<#element_ty>::__nc_from_raw_parts(
                            #offset_ident,
                            #ptr_ident,
                            #len_ident,
                        )
                    } {
                        Ok(value) => value,
                        Err(status) => return status.normalize_error(),
                    };
                })
            }
            AbiArgKind::ComRef { interface_ty, mode_ty } => {
                let ident = &arg.ident;
                let interface_ty = interface_ty.as_ref();
                let mode_ty = mode_ty
                    .as_ref()
                    .map(|ty| {
                        let ty = ty.as_ref();
                        quote! { #ty }
                    })
                    .unwrap_or_else(|| quote! { #nc::ThreadLocal });
                Some(quote! {
                    let #ident = match unsafe {
                        // SAFETY: The raw thunk checks the call-scoped pointer
                        // against the callee's expected full interface identity.
                        // Null and mismatched pointers fail before dispatch;
                        // live common-prefix validity remains the raw caller's
                        // responsibility for the duration of the call.
                        #nc::ComRef::<#interface_ty, #mode_ty>::__nc_from_raw_borrowed_checked(#ident)
                    } {
                        Ok(value) => value,
                        Err(status) => return status,
                    };
                })
            }
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
                    unsafe { *__nc_out = value; }
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
        let dispatch = match method.receiver {
            AbiReceiver::Shared => quote! {
                <T as #nc::ComAbiObject>::with_abi_ref(#object_ident, |#value_ident| {
                    <T as #trait_ident>::#method_ident(#value_ident, #(#arg_names),*)
                })
            },
            AbiReceiver::Mutable => quote! {
                <T as #nc::ComAbiObject>::with_abi_mut(#object_ident, |#value_ident| {
                    <T as #trait_ident>::#method_ident(#value_ident, #(#arg_names),*)
                })
            },
        };
        quote! {
            // Raw vtable entry that validates ABI pointers, reconstructs safe
            // parameters, invokes the Rust trait method, and lowers its result.
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
                let result = match unsafe { #dispatch } {
                    Ok(result) => result,
                    Err(status) => return status,
                };
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
    // implementing class. Each class/interface pair therefore gets thunks that
    // call the correct concrete Rust type without dynamic Rust dispatch.
    let vtable_entries = methods.iter().map(|method| {
        let method_ident = &method.ident;
        let thunk_ident = format_ident!("{method_ident}_thunk");
        quote! { #method_ident: Self::#thunk_ident }
    });

    // Safe extension methods hide the raw vtable call from Rust users. They
    // allocate an uninitialized out slot, call the ABI function, and only read
    // the out value when the returned status is successful.
    let ext_methods = safe_method_implementations(&nc, &methods, quote! { self.as_mut_ptr() });
    let client_bodies = safe_method_implementations(&nc, &methods, quote! { self.__nc_pointer.as_mut_ptr() });
    let client_methods = methods.iter().zip(client_bodies).map(|(method, body)| {
        let user_docs = &method.doc_attrs;
        let generated_docs = safe_method_docs(trait_ident, method);
        quote! { #(#user_docs)* #(#generated_docs)* pub #body }
    });

    let ext_trait_methods = methods.iter().map(|method| {
        let method_ident = &method.ident;
        let user_docs = &method.doc_attrs;
        let generated_docs = safe_method_docs(trait_ident, method);
        let args = method.args.iter().map(|arg| {
            let ident = &arg.ident;
            let ty = &arg.rust_ty;
            quote! { #ident: #ty }
        });
        let ret = &method.ok.rust_ty;
        let receiver = safe_receiver(method.receiver);
        quote! {
            #(#user_docs)*
            #(#generated_docs)*
            // Declares the safe smart-pointer forwarding surface generated for
            // this source trait method.
            fn #method_ident(#receiver, #(#args),*) -> Result<#ret, #nc::NcStatus>;
        }
    });

    Ok(quote! {
        #input

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
        // `#vtable_ident` with `IUnknownVTable` first, and derives the identity
        // and ABI hash from the exact generated method contract. `iunknown`
        // performs only the shared-prefix pointer cast.
        unsafe impl #nc::ComInterface for #abi_ident {
            const IID: #nc::NcGuid = #guid;
            const INTERFACE_ID: #nc::NcInterfaceId =
                #nc::NcInterfaceId::new(#guid, #abi_ident::ABI_HASH);
            type VTable = #vtable_ident;

            // All generated interface headers share the same prefix as
            // `IUnknown`, so this is the canonical route for AddRef, Release,
            // and QueryInterface from any interface pointer.
            fn iunknown(this: *mut Self) -> *mut #nc::IUnknown {
                this.cast::<#nc::IUnknown>()
            }
        }

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
            #[doc = "SHA-256 hash of this interface's typed ABI contract."]
            pub const ABI_HASH: #nc::NcAbiHash = #nc::NcAbiHash::__from_abi_prefix(#hash_prefix, &[
                #(#interface_hash_parts,)*
            ]);

            #(#method_abi_descriptors)*

            #[doc = "Static descriptors for this interface's ABI methods."]
            pub const METHOD_DESCRIPTORS: [#nc::NcMethodDescriptor; #method_count_usize] = [
                #(#method_descriptors,)*
            ];

            #[doc = "Static ABI descriptor for this interface."]
            pub const INTERFACE_DESCRIPTOR: #nc::NcInterfaceDescriptor = #nc::NcInterfaceDescriptor {
                interface_id: <Self as #nc::ComInterface>::IID,
                qualified_name: #nc::NcStaticUtf8::from_static(#qualified_name),
                abi_hash: Self::ABI_HASH,
                base: #base_identity,
                method_count: #method_count,
                methods: Self::METHOD_DESCRIPTORS.as_ptr(),
            };

            /// Static base ancestry used to size and construct the complete chain.
            const BASE_CHAIN: &'static [&'static #nc::NcInterfaceDescriptor] = #base_chain;

            /// This contract followed by every generated base, in query order.
            pub const INTERFACE_CHAIN: [&'static #nc::NcInterfaceDescriptor; 1 + Self::BASE_CHAIN.len()] =
                #nc::__nc_interface_chain(&Self::INTERFACE_DESCRIPTOR, #base_chain);
        }

        // Header storage is recursive only over the statically known base
        // chain. It is embedded directly in the one class allocation.
        #[doc(hidden)]
        #vis struct #interfaces_ident<T> {
            /// Projection whose header advertises this exact interface contract.
            own: #abi_ident,
            #storage_base
            #(#additional_storage)*
            /// Associates the thunk implementor without owning a second value.
            implementor: core::marker::PhantomData<fn() -> T>,
        }

        impl<T: #trait_ident + #nc::ComAbiObject> #interfaces_ident<T> {
            /// Creates unpublished headers; the class installs its back-pointer.
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
        #vis struct #vtable_for_ident<T>(core::marker::PhantomData<T>);

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

            // Routes this interface header's IUnknown prefix to the generated
            // object's allocation-wide interface negotiation implementation.
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
                // SAFETY: Delegates to the generated class object, which
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

            // Routes this interface header's AddRef to shared object storage.
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

            // Routes this interface header's Release to shared object storage.
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

        #[doc = concat!("Safe owning and borrowed call helpers for `ComPtr<", stringify!(#abi_ident), ", M>`.")]
        #vis trait #ext_ident {
            #(#ext_trait_methods)*
        }

        impl<M: #nc::ThreadingModel> #ext_ident for #nc::ComPtr<#abi_ident, M> {
            #(#ext_methods)*
        }

        // Borrowing already keeps the interface alive for this synchronous call.
        // Reuse the identical lowering without a temporary owned reference.
        impl<M: #nc::ThreadingModel> #ext_ident for #nc::ComRef<'_, #abi_ident, M> {
            #(#ext_methods)*
        }
    })
}

/// Checks names shared by ordinary interfaces and callable class projections.
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

/// Generates one synchronous call lowering reused by owning, borrowed and inherited clients.
fn safe_method_implementations(nc: &syn::Path, methods: &[AbiMethod], pointer: proc_macro2::TokenStream) -> Vec<proc_macro2::TokenStream> {
    methods
        .iter()
        .map(|method| {
            let method_ident = &method.ident;
            let args = method.args.iter().map(|arg| {
                let ident = &arg.ident;
                let ty = &arg.rust_ty;
                quote! { #ident: #ty }
            });
            let arg_preparations = method.args.iter().filter_map(safe_arg_preparation);
            let arg_names = method.args.iter().flat_map(safe_vtable_call_args).collect::<Vec<_>>();
            let rust_ret = &method.ok.rust_ty;
            let receiver = safe_receiver(method.receiver);
            match &method.ok.kind {
                AbiReturnKind::Void => quote! {
                    // Implements the safe extension trait by forwarding to the raw
                    // status-only vtable slot.
                    fn #method_ident(#receiver, #(#args),*) -> Result<#rust_ret, #nc::NcStatus> {
                        #(#arg_preparations)*
                        let this = #pointer;
                        // SAFETY: `self` holds a live interface reference. The raw
                        // vtable method receives borrowed pointers valid for this
                        // call and returns only an `NcStatus`.
                        let status = unsafe {
                            ((*(*this).vtable).#method_ident)(
                                this,
                                #(#arg_names,)*
                            )
                        };
                        if status.is_ok() {
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
                            Ok(unsafe { __nc_out.assume_init() })
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
                        // Implements the safe extension trait with a local output
                        // slot that is read only after ABI success.
                        fn #method_ident(#receiver, #(#args),*) -> Result<#rust_ret, #nc::NcStatus> {
                            #(#arg_preparations)*
                            let mut __nc_out = core::mem::MaybeUninit::<#abi_ret>::uninit();
                            let this = #pointer;
                            // SAFETY: `self` holds a live interface reference. The
                            // out pointer targets local storage that remains valid
                            // until the call returns.
                            let status = unsafe {
                                ((*(*this).vtable).#method_ident)(
                                    this,
                                    #(#arg_names,)*
                                    __nc_out.as_mut_ptr(),
                                )
                            };
                            if status.is_ok() {
                                #success_return
                            } else {
                                Err(status)
                            }
                        }
                    }
                }
            }
        })
        .collect::<Vec<_>>()
}

/// Emits the safe Rust receiver corresponding to one validated COM method.
fn safe_receiver(receiver: AbiReceiver) -> proc_macro2::TokenStream {
    match receiver {
        AbiReceiver::Shared => quote! { &self },
        AbiReceiver::Mutable => quote! { &mut self },
    }
}

/// Emits compiler-resolved hash values for one lowered method.
fn method_dynamic_hash_parts(nc: &syn::Path, method: &AbiMethod) -> Vec<proc_macro2::TokenStream> {
    let mut parts = Vec::new();
    for (arg_index, arg) in method.args.iter().enumerate() {
        let arg_index = arg_index as u32;
        match &arg.kind {
            AbiArgKind::Plain => {
                let ty = &arg.abi_ty;
                parts.push(quote! {
                    #nc::NcAbiHashPart::Arg {
                        index: #arg_index,
                        passing: #nc::NcAbiPassing::Value,
                        ty: #nc::__nc_abi_type::<#ty>(),
                    }
                });
            }
            AbiArgKind::CopyFromBufferOffset { element_ty } => {
                let element_ty = element_ty.as_ref();
                parts.push(quote! {
                    #nc::NcAbiHashPart::Arg {
                        index: #arg_index,
                        passing: #nc::NcAbiPassing::CopyFromBufferOffset,
                        ty: #nc::__nc_abi_plain_type::<#element_ty>(),
                    }
                });
            }
            AbiArgKind::CopyToBufferOffset { element_ty } => {
                let element_ty = element_ty.as_ref();
                parts.push(quote! {
                    #nc::NcAbiHashPart::Arg {
                        index: #arg_index,
                        passing: #nc::NcAbiPassing::CopyToBufferOffset,
                        ty: #nc::__nc_abi_plain_type::<#element_ty>(),
                    }
                });
            }
            AbiArgKind::ComRef { interface_ty, mode_ty } => {
                let interface_ty = interface_ty.as_ref();
                let mode = threading_mode_expr(nc, mode_ty.as_deref());
                parts.push(quote! {
                    #nc::NcAbiHashPart::Arg {
                        index: #arg_index,
                        passing: #nc::NcAbiPassing::ComRef {
                            mode: #mode,
                        },
                        ty: #nc::NcAbiType::Interface {
                            iid: <#interface_ty as #nc::ComInterface>::IID,
                        },
                    }
                });
            }
        }
    }
    match &method.ok.kind {
        AbiReturnKind::Void => {
            parts.push(quote! {
                #nc::NcAbiHashPart::Return {
                    passing: #nc::NcAbiPassing::Void,
                    ty: #nc::NcAbiType::Void,
                }
            });
        }
        AbiReturnKind::Plain => {
            let ty = &method.ok.abi_ty;
            parts.push(quote! {
                #nc::NcAbiHashPart::Return {
                    passing: #nc::NcAbiPassing::Out,
                    ty: #nc::__nc_abi_type::<#ty>(),
                }
            });
        }
        AbiReturnKind::ComPtr { interface_ty, mode_ty } => {
            let interface_ty = interface_ty.as_ref();
            let mode = threading_mode_expr(nc, mode_ty.as_deref());
            parts.push(quote! {
                #nc::NcAbiHashPart::Return {
                    passing: #nc::NcAbiPassing::ComPtrOut {
                        mode: #mode,
                    },
                    ty: #nc::NcAbiType::Interface {
                        iid: <#interface_ty as #nc::ComInterface>::IID,
                    },
                }
            });
        }
    }
    parts
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
        AbiArgKind::Plain => quote! {
            #nc::NcAbiPassingDescriptor::from_abi_passing(#nc::NcAbiPassing::Value)
        },
        AbiArgKind::CopyFromBufferOffset { .. } => quote! {
            #nc::NcAbiPassingDescriptor::from_abi_passing(#nc::NcAbiPassing::CopyFromBufferOffset)
        },
        AbiArgKind::CopyToBufferOffset { .. } => quote! {
            #nc::NcAbiPassingDescriptor::from_abi_passing(#nc::NcAbiPassing::CopyToBufferOffset)
        },
        AbiArgKind::ComRef { mode_ty, .. } => {
            let mode = threading_mode_expr(nc, mode_ty.as_deref());
            quote! {
                #nc::NcAbiPassingDescriptor::from_abi_passing(#nc::NcAbiPassing::ComRef {
                    mode: #mode,
                })
            }
        }
    }
}

/// Maps an argument's source type to its canonical ABI type descriptor.
fn arg_type_descriptor(nc: &syn::Path, arg: &crate::model::AbiArg) -> proc_macro2::TokenStream {
    match &arg.kind {
        AbiArgKind::Plain => {
            let ty = &arg.abi_ty;
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
            let ty = &method.ok.abi_ty;
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
        AbiArgKind::Plain | AbiArgKind::ComRef { .. } => {
            let ty = &arg.abi_ty;
            vec![quote! { #ident: #ty }]
        }
        AbiArgKind::CopyFromBufferOffset { element_ty } => {
            let element_ty = element_ty.as_ref();
            let offset_ident = buffer_offset_ident(ident);
            let len_ident = buffer_len_ident(ident);
            let ptr_ident = buffer_ptr_ident(ident);
            vec![
                quote! { #offset_ident: u64 },
                quote! { #len_ident: u64 },
                quote! { #ptr_ident: *const #element_ty },
            ]
        }
        AbiArgKind::CopyToBufferOffset { element_ty } => {
            let element_ty = element_ty.as_ref();
            let offset_ident = buffer_offset_ident(ident);
            let len_ident = buffer_len_ident(ident);
            let ptr_ident = buffer_ptr_ident(ident);
            vec![
                quote! { #offset_ident: u64 },
                quote! { #len_ident: u64 },
                quote! { #ptr_ident: *mut #element_ty },
            ]
        }
    }
}

/// Emits pre-call pointer/bounds extraction required by a safe wrapper argument.
fn safe_arg_preparation(arg: &crate::model::AbiArg) -> Option<proc_macro2::TokenStream> {
    match &arg.kind {
        AbiArgKind::CopyToBufferOffset { .. } => {
            let ident = &arg.ident;
            Some(quote! { let mut #ident = #ident; })
        }
        AbiArgKind::Plain | AbiArgKind::CopyFromBufferOffset { .. } | AbiArgKind::ComRef { .. } => None,
    }
}

/// Emits the raw values passed for one source-level safe argument.
fn safe_vtable_call_args(arg: &crate::model::AbiArg) -> Vec<proc_macro2::TokenStream> {
    let ident = &arg.ident;
    match &arg.kind {
        AbiArgKind::Plain => vec![quote! { #ident }],
        AbiArgKind::ComRef { .. } => vec![quote! { #ident.as_mut_ptr() }],
        AbiArgKind::CopyFromBufferOffset { .. } => vec![quote! { #ident.offset() }, quote! { #ident.__nc_len_u64()? }, quote! { #ident.as_ptr() }],
        AbiArgKind::CopyToBufferOffset { .. } => vec![quote! { #ident.offset() }, quote! { #ident.__nc_len_u64()? }, quote! { #ident.__nc_mut_ptr() }],
    }
}

/// Builds the generated offset parameter name for a bounded buffer.
fn buffer_offset_ident(ident: &syn::Ident) -> syn::Ident {
    format_ident!("__nc_{ident}_offset")
}

/// Builds the generated length parameter name for a bounded buffer.
fn buffer_len_ident(ident: &syn::Ident) -> syn::Ident {
    format_ident!("__nc_{ident}_len")
}

/// Builds the generated data-pointer parameter name for a bounded buffer.
fn buffer_ptr_ident(ident: &syn::Ident) -> syn::Ident {
    format_ident!("__nc_{ident}_ptr")
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
                "this" => Some("the generated safe-wrapper receiver pointer"),
                "object" if matches!(&arg.kind, AbiArgKind::Plain | AbiArgKind::ComRef { .. }) => Some("the generated ABI object back-pointer local"),
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
        AbiArgKind::Plain | AbiArgKind::ComRef { .. } => vec![(ident.to_string(), false)],
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
