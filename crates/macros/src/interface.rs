//! Generation of interface markers, vtables, thunks, wrappers, and ABI metadata.
//!
//! A validated trait is lowered into an interface marker and `repr(C)` method table,
//! per-implementor thunk table, one borrowed client method view, and immutable
//! method descriptors. The client view is the sole safe Rust-to-ABI lowering
//! surface, so pointer ownership, buffer bounds, and documentation stay aligned.

use quote::{format_ident, quote};
use syn::{ItemTrait, Result, TypeParamBound, TraitBoundModifier, PathArguments};

use crate::abi_method::{parse_abi_trait_item, raw_vtable_method_docs, safe_method_docs, validate_interface_trait};
use crate::args::{InterfaceArgs, InterfaceThreading};
use crate::model::{AbiArgKind, AbiMethod, AbiReceiver, AbiReturnKind};
use crate::util::{abi_signature_static_tokens, checked_u32_len, collect_syn_results, interface_guid, private_ident, push_error, runtime_crate_path};

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
    /// Traits, markers, clients and metadata, using only allocation-free core types.
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
    let required_providers = interface_paths.iter().map(|path| generated_path(path, "VtblFor")).collect::<Vec<_>>();
    let required_abis = interface_paths.iter().map(|path| generated_path(path, "Abi")).collect::<Vec<_>>();
    let mut base_identity = quote! { #nc::NcInterfaceId::NONE };
    let mut prefix_field = quote! {
        /// Common identity, query and reference-count slots.
        pub unknown: #nc::IUnknownVTable,
    };
    let mut prefix_value = quote! {
        unknown: #nc::IUnknownVTable {
            interface_id: identity,
            query_interface: #nc::lifetime::nc_query_interface,
            add_ref: #nc::lifetime::nc_strong_retain,
            release: #nc::lifetime::nc_strong_release,
            downgrade: ::core::option::Option::Some(#nc::lifetime::nc_downgrade),
        },
    };
    let mut client_base = quote! {};
    if let Some(base) = base {
        let base_abi = generated_path(base, "Abi");
        let base_vtable = generated_path(base, "VTable");
        let base_vtable_for = generated_path(base, "VtblFor");
        let base_client = generated_path(base, "Client");
        base_identity = quote! { <#base_abi as #nc::ComInterface>::INTERFACE_ID };
        prefix_field = quote! {
            /// Complete inherited vtable at offset zero; local slots follow it.
            pub base: #base_vtable,
        };
        prefix_value = quote! { base: #base_vtable_for::<T>::with_identity(identity), };
        client_base = quote! {
            impl<M: #nc::ThreadingModel> ::core::ops::Deref for #client_ident<M> {
                type Target = #base_client<M>;
                fn deref(&self) -> &Self::Target {
                    // SAFETY: Both generated views are transparent pointer words.
                    // The inherited table is a physical prefix of the derived table.
                    // This lends methods only; ownership and checked base projection
                    // still require QueryInterface on the same object with the actual derived table.
                    unsafe { &*::core::ptr::from_ref(self).cast::<Self::Target>() }
                }
            }

            impl<M: #nc::ThreadingModel> ::core::ops::DerefMut for #client_ident<M> {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    // SAFETY: Both views are transparent pointer words and the
                    // mutable borrow remains tied to this derived client view.
                    unsafe { &mut *::core::ptr::from_mut(self).cast::<Self::Target>() }
                }
            }
        };
    }

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
                this: *mut ::core::ffi::c_void,
                #(#abi_args,)*
                #out_param
            ) -> #nc::NcStatus,
        }
    });

    // A thunk adapts one C ABI vtable call back into the Rust trait method.
    // It accesses the owning generated storage directly through the handle's
    // opaque `object` pointer, then asks `ComAbiObject` for the concrete Rust
    // value through the generated object-wide borrow gate. Panics are
    // intentionally not caught here; the non-unwind C ABI boundary follows
    // Rust's normal panic and ABI unwind rules.
    let thunks = methods.iter().map(|method| {
        let method_ident = &method.ident;
        let thunk_ident = format_ident!("{method_ident}_thunk", span = method_ident.span());
        // Argument bindings are positional and distinct from dispatch helpers.
        // Mixed-site spans additionally isolate ordinary macro expansion.
        let object_ident = private_ident("__nc_dispatch_object");
        let value_ident = private_ident("__nc_dispatch_value");
        let params = method.args.iter().flat_map(|arg| raw_vtable_arg_params(arg));
        let arg_conversions = method.args.iter().filter_map(|arg| match &arg.kind {
            AbiArgKind::ComBox { interface_ty, mode_ty } => {
                let ident = &arg.binding;
                let mode = mode_ty.as_ref().map(|ty| quote! { #ty }).unwrap_or_else(|| quote! { #nc::ThreadLocal });
                Some(quote! {
                    // Validate without adopting: any later input or receiver
                    // rejection must leave the original owner in the caller slot.
                    if let ::core::result::Result::Err(status) = unsafe { #nc::ComBox::<#interface_ty, #mode>::__nc_validate_raw_input(#ident) } {
                        return status;
                    }
                })
            },
            AbiArgKind::BorrowedValue { value_ty } => {
                let ident = &arg.binding;
                Some(quote! {
                    // SAFETY: the caller supplies this concrete value's raw C record
                    // and keeps its selected borrowed fields live for the call.
                    let #ident = match unsafe { #nc::NcValueRef::<#value_ty>::from_raw(#ident) } {
                        ::core::result::Result::Ok(value) => value,
                        ::core::result::Result::Err(status) => return status,
                    };
                })
            }
            AbiArgKind::Plain => {
                let ident = &arg.binding;
                let ty = &arg.rust_ty;
                Some(quote! {
                    // SAFETY: copied wire storage is initialized; restricted fields are validated here.
                    let #ident = match unsafe { #nc::decode_inline::<#ty>(#ident) } {
                        ::core::result::Result::Ok(value) => value, ::core::result::Result::Err(status) => return status,
                    };
                })
            }
            AbiArgKind::CopyFromBufferOffset { element_ty } | AbiArgKind::CopyToBufferOffset { element_ty } => {
                let ident = &arg.binding;
                let offset = buffer_offset_ident(ident);
                let len = buffer_len_ident(ident);
                let ptr = buffer_ptr_ident(ident);
                let values = buffer_values_ident(ident);
                let (decoder, wrapper) = match &arg.kind {
                    AbiArgKind::CopyFromBufferOffset { .. } => (
                        quote! { #nc::DecodedInput },
                        quote! { #nc::NcCopyFromBufferOffset::new(#offset, #values.as_slice()) },
                    ),
                    _ => (
                        quote! { #nc::DecodedOutput },
                        quote! { #nc::NcCopyToBufferOffset::new(#offset, #values.as_mut_slice()) },
                    ),
                };
                Some(quote! {
                    // SAFETY: initialized raw elements are checked before any application slice exists;
                    // direct borrowing additionally requires the codec's identity-conversion proof.
                    let mut #values = match unsafe { #decoder::<#element_ty>::from_raw(#ptr, #len) } {
                        ::core::result::Result::Ok(values) => values, ::core::result::Result::Err(status) => return status,
                    };
                    let #ident = #wrapper;
                })
            }
            AbiArgKind::ComRef { interface_ty, mode_ty, optional } => {
                let ident = &arg.binding;
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
                        ::core::result::Result::Ok(value) => value,
                        ::core::result::Result::Err(status) => return status,
                    }
                };
                Some(match optional {
                    true => quote! {
                        // Absence needs no validation; present capabilities use
                        // exactly the same identity checks as required inputs.
                        let #ident = if #ident.is_null() { ::core::option::Option::None } else { ::core::option::Option::Some(#checked) };
                    },
                    false => quote! { let #ident = #checked; },
                })
            }
        });
        let owners = method.args.iter().filter(|arg| matches!(arg.kind, AbiArgKind::ComBox { .. })).collect::<Vec<_>>();
        let has_owners = !owners.is_empty();
        let output_checks = consuming_output_checks(&nc, method, |arg| {
            let len = buffer_len_ident(&arg.binding);
            quote! { #len }
        });
        let output_preflight = output_checks.iter().map(|check| quote! {
            if let ::core::result::Result::Err(status) = #check { return status; }
        });
        // Save slots before replacing raw argument bindings with ordinary owners.
        // Adoption happens only after all validation and receiver admission.
        let owner_adoptions = owners.iter().map(|arg| {
            let ident = &arg.binding;
            let ty = &arg.rust_ty;
            let slot = owner_slot_ident(ident);
            quote! {
                let #slot = #ident;
                let #ident = unsafe {
                    <#ty>::from_raw_owned(::core::mem::replace(&mut *#slot, #nc::ComRaw::null()))
                }.expect("validated unique COM owner");
            }
        });
        let error_arm = match has_owners {
            false => quote! { ::core::result::Result::Err(status) => status.normalize_error(), },
            true => {
                let names = owners.iter().map(|arg| &arg.binding);
                let restores = owners.iter().map(|arg| {
                    let ident = &arg.binding;
                    let slot = owner_slot_ident(ident);
                    quote! { unsafe { *#slot = #ident.into_raw(); } }
                });
                quote! {
                    ::core::result::Result::Err((status, #(#names),*)) => {
                        // No receiver borrow survives while returned owners are
                        // handed back. Every forwarding provider must propagate
                        // this error tuple without destroying or replacing inputs.
                        #(#restores)*
                        status.normalize_error()
                    },
                }
            },
        };
        // A successful owning method has committed. It cannot subsequently
        // report rejection without returning those owners. Malformed successful
        // output is a provider contract violation, never a recoverable rejection.
        let output_error = match has_owners {
            true => quote! { panic!("invalid output after COM ownership commit: {:?}", status) },
            false => quote! { return status },
        };
        let buffer_conversion = method.args.iter().filter_map(|arg| {
            let AbiArgKind::CopyToBufferOffset { .. } = &arg.kind else {
                return None;
            };
            let values = buffer_values_ident(&arg.binding);
            Some(quote! { if let ::core::result::Result::Err(status) = #values.convert_output() { #output_error; } })
        }).collect::<Vec<_>>();
        let buffer_publication = method.args.iter().filter_map(|arg| {
            let AbiArgKind::CopyToBufferOffset { .. } = &arg.kind else { return None; };
            let values = buffer_values_ident(&arg.binding);
            Some(quote! { #values.publish(); })
        }).collect::<Vec<_>>();
        let arg_names = method.args.iter().map(|arg| &arg.binding).collect::<Vec<_>>();
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
                ::core::result::Result::Ok(()) => {
                    #(#buffer_conversion)*
                    #(#buffer_publication)*
                    #nc::NC_OK
                },
            },
            AbiReturnKind::Plain => quote! {
                ::core::result::Result::Ok(value) => {
                    // SAFETY: Non-void thunks checked `__nc_out` for null before the
                    // Rust method was called.
                    let raw = match #nc::encode_inline(&value) { ::core::result::Result::Ok(raw) => raw, ::core::result::Result::Err(status) => { #output_error } };
                    #(#buffer_conversion)*
                    #(#buffer_publication)*
                    unsafe { *__nc_out = raw; }
                    #nc::NC_OK
                }
            },
            AbiReturnKind::ComPtr { .. } => quote! {
                ::core::result::Result::Ok(value) => {
                    // SAFETY: Non-void thunks checked `__nc_out` for null.
                    // `into_raw` transfers the returned owned COM reference to
                    // the caller.
                    #(#buffer_conversion)*
                    #(#buffer_publication)*
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
                this: *mut ::core::ffi::c_void,
                #(#params,)*
                #out_param
            ) -> #nc::NcStatus {
                #pointer_check

                // SAFETY: `this` was checked non-null above and is a generated
                // object address paired with this vtable.
                let #object_ident = this;
                if #object_ident.is_null() {
                    return #nc::NC_E_POINTER;
                }
                // All possible outputs must fit before any owning input is adopted.
                #(#output_preflight)*
                #(#arg_conversions)*
                // SAFETY: The object pointer is paired with this generated table.
                // `ComAbiObject` validates the requested Rust borrow
                // against every call active on the same allocation.
                let mut #value_ident = match unsafe { #borrow } {
                    ::core::result::Result::Ok(value) => value,
                    ::core::result::Result::Err(status) => return status,
                };
                #(#owner_adoptions)*
                let result = #invoke;
                // Release the object-wide borrow before lowering an owned
                // result into the caller's output slot.
                ::core::mem::drop(#value_ident);
                match result {
                    #success_arm
                    // Rust `Result::Err` must map to a negative ABI status.
                    // Normalizing here prevents an invalid `Err(success)` from
                    // making a safe caller read an unwritten out slot.
                    #error_arm
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

    let output_contracts = methods.iter().flat_map(|method| {
        if !method.args.iter().any(|arg| matches!(arg.kind, AbiArgKind::ComBox { .. })) {
            return Vec::new();
        }
        let mut proofs = Vec::new();
        if matches!(method.ok.kind, AbiReturnKind::Plain) {
            let ty = &method.ok.rust_ty;
            proofs.push(quote! {
                assert!(<#ty as #nc::ComValue>::MAX_COST.is_some(),
                    "consuming COM copied output requires an allocation-free MAX_COST");
            });
        }
        for arg in &method.args {
            if let AbiArgKind::CopyToBufferOffset { element_ty } = &arg.kind {
                proofs.push(quote! {
                    assert!(<#element_ty as #nc::ComValue>::BORROW_WIRE || <#element_ty as #nc::ComValue>::MAX_COST.is_some(),
                        "consuming COM converted buffer output requires an allocation-free MAX_COST");
                });
            }
        }
        proofs
    }).collect::<Vec<_>>();
    let output_contracts = match output_contracts.is_empty() {
        true => quote! {},
        false => quote! { const _: () = { #(#output_contracts)* }; },
    };
    let abi = quote! {
        #input

        // Even unused declarations must prove allocation-free converted completion.
        // IDL and handwritten interfaces share this codec-level requirement.
        #output_contracts

        #signature_static

        // The marker gives ComRaw/ComPtr their static interface contract.
        // No IFooAbi instance is stored in the object allocation.
        #[doc = concat!("Interface marker for [`", stringify!(#trait_ident), "`]; handles carry its object/table pair.")]
        #vis enum #abi_ident {}

        // Vtables start with the `IUnknown` function table, then append the
        // interface-specific methods in source order.
        #[repr(C)]
        #[doc = concat!("C-compatible vtable for [`", stringify!(#trait_ident), "`].")]
        #vis struct #vtable_ident {
            #prefix_field
            #(#method_fields)*
        }

        // SAFETY: The generated table has the common IUnknownVTable prefix,
        // exact identity, and thunks accepting the matching object allocation.
        // The marker carries the IDL-assigned contract and threading mode.
        unsafe impl #nc::ComInterface for #abi_ident {
            const IID: #nc::NcGuid = #guid;
            const INTERFACE_ID: #nc::NcInterfaceId =
                #nc::NcInterfaceId::new(#guid, #abi_ident::ABI_SIGNATURE);
            type VTable = #vtable_ident;
            type Threading = #threading;


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

            /// Distinct projections in derived-first, declared-parent order.
            /// Shared ancestors occur once; repeated identities must have the same ABI.
            pub const INTERFACE_CHAIN: [&'static #nc::NcInterfaceDescriptor; #nc::__nc_object_interface_count(&[
                &[&Self::INTERFACE_DESCRIPTOR], #(&#required_abis::INTERFACE_CHAIN,)*
            ])] =
                #nc::__nc_interface_chain(&Self::INTERFACE_DESCRIPTOR, &[#(&#required_abis::INTERFACE_CHAIN,)*]);
        }

    };
    let provider = quote! {
        #[doc(hidden)]
        /// Statically typed vtable and lifecycle adapters for one provider type.
        #vis struct #vtable_for_ident<T>(
            /// Associates dispatch with T without allocating or owning a value.
            ::core::marker::PhantomData<T>
        );

        impl<T> #vtable_for_ident<T>
        where
            T: #trait_ident + #nc::ComAbiObject,
        {
            /// Distinct provider bindings in the interface's canonical metadata order.
            /// T fixes the thunk implementation; no object headers are stored here.
            pub const PROJECTIONS: [#nc::__ComProjection<T>; #abi_ident::INTERFACE_CHAIN.len()] =
                #nc::__nc_projection_table(
                    &#abi_ident::INTERFACE_CHAIN,
                    // SAFETY: this macro emits the exact contract identity and
                    // complete prefix-compatible vtable for T's allocation.
                    unsafe { #nc::__ComProjection::new(<#abi_ident as #nc::ComInterface>::INTERFACE_ID,
                        &Self::VTABLE as *const #vtable_ident as *const #nc::IUnknownVTable) },
                    &[#(&#required_providers::<T>::PROJECTIONS,)*],
                );

            /// Table for a standalone projection of this interface.
            pub const VTABLE: #vtable_ident = Self::with_identity(&<#abi_ident as #nc::ComInterface>::INTERFACE_ID);

            /// Builds an inherited prefix advertising the actual outer projection.
            pub const fn with_identity(identity: &'static #nc::NcInterfaceId) -> #vtable_ident {
                #vtable_ident {
                    #prefix_value
                    #(#vtable_entries,)*
                }
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
        let ident = &arg.binding;
        let ty = &arg.rust_ty;
        quote! { #ident: #ty }
    });
    let owners = method
        .args
        .iter()
        .filter(|arg| matches!(arg.kind, AbiArgKind::ComBox { .. }))
        .collect::<Vec<_>>();
    let has_owners = !owners.is_empty();
    let error_type = match has_owners {
        false => quote! { #nc::NcStatus },
        true => {
            let types = owners.iter().map(|arg| &arg.rust_ty);
            quote! { (#nc::NcStatus, #(#types),*) }
        }
    };
    let owner_setup = owners
        .iter()
        .map(|arg| {
            let ident = &arg.binding;
            let slot = owner_slot_ident(ident);
            quote! {
                // The wire slot carries the token during the call. This dormant
                // wrapper is recovered only on rejection, never dropped on success.
                let #ident = ::core::mem::ManuallyDrop::new(#ident);
                let mut #slot = #ident.as_raw();
            }
        })
        .collect::<Vec<_>>();
    let owner_checks = owners
        .iter()
        .map(|arg| {
            let ident = &arg.binding;
            let slot = owner_slot_ident(ident);
            quote! {
                match status.is_ok() {
                    true => assert!(#slot.is_null(), "successful COM call did not consume its owner"),
                    false => assert!(#slot.object == #ident.as_raw().object && #slot.vtable == #ident.as_raw().vtable,
                        "rejected COM call did not return its original owner"),
                }
            }
        })
        .collect::<Vec<_>>();
    let arg_preparations = method.args.iter().filter_map(|arg| safe_arg_preparation(nc, arg)).collect::<Vec<_>>();
    let output_checks = consuming_output_checks(nc, method, |arg| {
        let ident = &arg.binding;
        quote! { #ident.__nc_len_u64()? }
    });
    let preparation = match has_owners {
        false => quote! { #(#arg_preparations)* },
        true => {
            let prepared = method
                .args
                .iter()
                .filter(|arg| {
                    matches!(
                        arg.kind,
                        AbiArgKind::Plain | AbiArgKind::CopyFromBufferOffset { .. } | AbiArgKind::CopyToBufferOffset { .. }
                    )
                })
                .map(|arg| &arg.binding)
                .collect::<Vec<_>>();
            let owner_names = owners.iter().map(|arg| &arg.binding);
            quote! {
                // Keep ordinary Rust owners during fallible encoding, including
                // unwinding. Only the actual non-unwinding ABI call lends tokens.
                let (#(mut #prepared,)*) = match (|| {
                    #(#output_checks?;)*
                    #(#arg_preparations)*
                    ::core::result::Result::Ok::<_, #nc::NcStatus>((#(#prepared,)*))
                })() {
                    ::core::result::Result::Ok(prepared) => prepared,
                    ::core::result::Result::Err(status) => return ::core::result::Result::Err((status, #(#owner_names),*)),
                };
                #(#owner_setup)*
            }
        }
    };
    let rejection = match has_owners {
        false => quote! { status },
        true => {
            let recover = owners.iter().map(|arg| {
                let ident = &arg.binding;
                quote! { ::core::mem::ManuallyDrop::into_inner(#ident) }
            });
            quote! { (status, #(#recover),*) }
        }
    };
    // Malformed output after a successful ownership commit is a provider
    // violation: the caller no longer has input owners to return as an error.
    let acquire = match has_owners {
        false => quote! { ? },
        true => quote! { .expect("invalid output after COM ownership commit") },
    };
    let arg_names = method.args.iter().flat_map(safe_vtable_call_args).collect::<Vec<_>>();
    let buffer_acquisition = method
        .args
        .iter()
        .filter_map(|arg| {
            let AbiArgKind::CopyToBufferOffset { .. } = &arg.kind else {
                return None;
            };
            let ident = &arg.binding;
            Some(quote! { #ident.convert_output() #acquire; })
        })
        .collect::<Vec<_>>();
    let buffer_publication = method.args.iter().filter_map(|arg| {
        let AbiArgKind::CopyToBufferOffset { .. } = &arg.kind else { return None; };
        let ident = &arg.binding;
        Some(quote! { #ident.publish(); })
    });
    let rust_ret = &method.ok.rust_ty;
    let receiver = safe_receiver(method.receiver);
    // Output shape is the only return-specific part of a client call. Keep one
    // admission, ownership handshake and error path for every method shape.
    let (output_storage, output_argument, success) = match &method.ok.kind {
        AbiReturnKind::Void => (quote! {}, quote! {}, quote! { () }),
        kind => {
            let abi_ret = &method.ok.abi_ty;
            let decode = match kind {
                AbiReturnKind::Plain => quote! {
                    (unsafe { #nc::decode_inline::<#rust_ret>(__nc_out.assume_init()) }) #acquire
                },
                AbiReturnKind::ComPtr { .. } => quote! {
                    (unsafe { <#rust_ret>::__nc_from_raw_owned_checked(__nc_out.assume_init()) }) #acquire
                },
                AbiReturnKind::Void => unreachable!(),
            };
            (
                quote! { let mut __nc_out = ::core::mem::MaybeUninit::<#abi_ret>::uninit(); },
                quote! { __nc_out.as_mut_ptr(), },
                decode,
            )
        }
    };
    let body = quote! {
        #preparation
        #output_storage
        let this = self.__nc_pointer.as_raw();
        // SAFETY: this view borrows retained interface storage. Prepared input
        // buffers, owner slots and optional output storage remain live for the
        // synchronous call. The provider validates its own object admission.
        let status = unsafe {
            ((*this.vtable).#method_ident)(this.object, #(#arg_names,)* #output_argument)
        };
        #(#owner_checks)*
        if status.is_err() { return ::core::result::Result::Err(#rejection); }
        // ABI success promises initialized output. Acquire owned results before
        // validating mutable buffers so a buffer error cannot leak that result.
        let result = #success;
        #(#buffer_acquisition)*
        // Publish only after all converted outputs, including the result, validated.
        #(#buffer_publication)*
        ::core::result::Result::Ok(result)
    };
    quote! {
        fn #method_ident (#receiver, #(#args),*) -> ::core::result::Result<#rust_ret, #error_type> {
            #body
        }
    }
}

/// Builds pre-acceptance resource checks from the shared output codecs.
/// The caller supplies wrapper lengths; the provider supplies raw ABI lengths.
/// Non-consuming methods keep actual-output accounting and require no maximum.
fn consuming_output_checks(
    nc: &syn::Path,
    method: &AbiMethod,
    buffer_len: impl Fn(&crate::model::AbiArg) -> proc_macro2::TokenStream,
) -> Vec<proc_macro2::TokenStream> {
    if !method.args.iter().any(|arg| matches!(arg.kind, AbiArgKind::ComBox { .. })) {
        return Vec::new();
    }
    let mut checks = Vec::new();
    if matches!(method.ok.kind, AbiReturnKind::Plain) {
        let ty = &method.ok.rust_ty;
        checks.push(quote! { #nc::check_inline_output::<#ty>() });
    }
    for arg in &method.args {
        if let AbiArgKind::CopyToBufferOffset { element_ty } = &arg.kind {
            let len = buffer_len(arg);
            checks.push(quote! { #nc::check_buffer_output::<#element_ty>(#len) });
        }
    }
    checks
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
        AbiArgKind::ComBox { mode_ty, .. } => {
            let mode = threading_mode_expr(nc, mode_ty.as_deref());
            quote! { #nc::NcAbiPassingDescriptor::from_abi_passing(#nc::NcAbiPassing::ComBoxIn { mode: #mode }) }
        }
        AbiArgKind::BorrowedValue { .. } => quote! {
            #nc::NcAbiPassingDescriptor::from_abi_passing(#nc::NcAbiPassing::BorrowedValue)
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
            let passing = if *optional {
                quote! { OptionalComRef }
            } else {
                quote! { ComRef }
            };
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
        AbiArgKind::BorrowedValue { value_ty } => quote! {
            #nc::NcAbiTypeDescriptor::from_abi_type(#nc::NcAbiType::Aggregate(
                <#value_ty as #nc::ComValueType>::VALUE_SIGNATURE
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
        AbiArgKind::ComRef { interface_ty, .. } | AbiArgKind::ComBox { interface_ty, .. } => {
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
        AbiReturnKind::ComPtr { mode_ty, ownership, .. } => {
            let mode = threading_mode_expr(nc, mode_ty.as_deref());
            let passing = match ownership {
                crate::model::InterfaceOwnership::Shared => quote! { ComPtrOut },
                crate::model::InterfaceOwnership::Exclusive => quote! { ComBoxOut },
                crate::model::InterfaceOwnership::Weak => quote! { WeakComBoxOut },
            };
            quote! {
                #nc::NcAbiPassingDescriptor::from_abi_passing(#nc::NcAbiPassing::#passing {
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
    let ident = &arg.binding;
    match &arg.kind {
        AbiArgKind::Plain | AbiArgKind::BorrowedValue { .. } | AbiArgKind::ComRef { .. } | AbiArgKind::ComBox { .. } => {
            let ty = &arg.abi_ty;
            vec![quote! { #ident: #ty }]
        }
        AbiArgKind::CopyFromBufferOffset { .. } => {
            let offset_ident = buffer_offset_ident(ident);
            let len_ident = buffer_len_ident(ident);
            let ptr_ident = buffer_ptr_ident(ident);
            vec![quote! { #offset_ident: u64 }, quote! { #len_ident: u64 }, {
                let ty = &arg.abi_ty;
                quote! { #ptr_ident: #ty }
            }]
        }
        AbiArgKind::CopyToBufferOffset { .. } => {
            let offset_ident = buffer_offset_ident(ident);
            let len_ident = buffer_len_ident(ident);
            let ptr_ident = buffer_ptr_ident(ident);
            vec![quote! { #offset_ident: u64 }, quote! { #len_ident: u64 }, {
                let ty = &arg.abi_ty;
                quote! { #ptr_ident: #ty }
            }]
        }
    }
}

/// Emits pre-call pointer/bounds extraction required by a safe wrapper argument.
fn safe_arg_preparation(nc: &syn::Path, arg: &crate::model::AbiArg) -> Option<proc_macro2::TokenStream> {
    match &arg.kind {
        AbiArgKind::ComBox { .. } => None,
        AbiArgKind::Plain => {
            let ident = &arg.binding;
            Some(quote! { let #ident = #nc::encode_inline(&#ident)?; })
        }
        AbiArgKind::CopyToBufferOffset { .. } => {
            let ident = &arg.binding;
            Some(quote! { let mut #ident = #nc::EncodedOutput::new(#ident)?; })
        }
        AbiArgKind::CopyFromBufferOffset { .. } => {
            let ident = &arg.binding;
            Some(quote! { let #ident = #nc::EncodedInput::new(#ident)?; })
        }
        AbiArgKind::BorrowedValue { .. } | AbiArgKind::ComRef { .. } => None,
    }
}

/// Emits the raw values passed for one source-level safe argument.
fn safe_vtable_call_args(arg: &crate::model::AbiArg) -> Vec<proc_macro2::TokenStream> {
    let ident = &arg.binding;
    match &arg.kind {
        AbiArgKind::ComBox { .. } => {
            let slot = owner_slot_ident(ident);
            vec![quote! { &mut #slot }]
        }
        AbiArgKind::BorrowedValue { .. } => vec![quote! { #ident.as_ptr() }],
        AbiArgKind::Plain => vec![quote! { #ident }],
        AbiArgKind::ComRef { optional: true, .. } => vec![quote! { #ident.map_or(::core::default::Default::default(), |value| value.as_raw()) }],
        AbiArgKind::ComRef { optional: false, .. } => vec![quote! { #ident.as_raw() }],
        AbiArgKind::CopyFromBufferOffset { .. } => vec![quote! { #ident.offset() }, quote! { #ident.len() }, quote! { #ident.as_ptr() }],
        AbiArgKind::CopyToBufferOffset { .. } => vec![quote! { #ident.offset() }, quote! { #ident.len() }, quote! { #ident.as_mut_ptr() }],
    }
}

/// Names private owner wire slots hygienically so user arguments cannot alias them.
fn owner_slot_ident(ident: &syn::Ident) -> syn::Ident {
    private_ident(&format!("{ident}_owner_slot"))
}

/// Builds the generated offset parameter name for a bounded buffer.
fn buffer_offset_ident(ident: &syn::Ident) -> syn::Ident {
    private_ident(&format!("{ident}_offset"))
}

/// Builds the generated length parameter name for a bounded buffer.
fn buffer_len_ident(ident: &syn::Ident) -> syn::Ident {
    private_ident(&format!("{ident}_len"))
}

/// Builds the generated data-pointer parameter name for a bounded buffer.
fn buffer_ptr_ident(ident: &syn::Ident) -> syn::Ident {
    private_ident(&format!("{ident}_ptr"))
}

/// Names the decoded backing retained until provider buffer writeback completes.
fn buffer_values_ident(ident: &syn::Ident) -> syn::Ident {
    private_ident(&format!("{ident}_values"))
}

/// Rejects public method conflicts with actual generated lifecycle operations.
/// Private locals impose no source-parameter restrictions: positional bindings
/// are disjoint from helper roles, even after printing and reparsing the output.
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
