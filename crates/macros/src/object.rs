//! Expansion of Rust structs into reference-counted multi-interface COM objects.
//!
//! The generator builds one private allocation containing an atomic reference
//! lifetime, one shared borrow guard, and the Rust value. Static tables carry interfaces.
//! Metadata, table bindings, and QueryInterface share the same compile-time set.
//! Generated lifecycle thunks, threading assertions, and construction helpers
//! preserve the one allocation and its concrete implementation type.

use quote::{format_ident, quote};
use syn::{ItemStruct, ext::IdentExt};

use crate::args::{ObjectArgs, ObjectOwnership};
use crate::util::runtime_crate_path;

/// Generates object storage, lifecycle thunks, and local construction helpers.
pub(crate) fn expand_com_object(args: ObjectArgs, input: ItemStruct) -> proc_macro2::TokenStream {
    // `FooComObject` is intentionally private generated storage. Users see the
    // original Rust `Foo` plus COM interface pointers; the storage layout is an
    // implementation detail owned by this macro.
    let nc = runtime_crate_path(args.runtime_crate.as_ref());
    let object_ident = &input.ident;
    let object_vis = &input.vis;
    let storage_ident = format_ident!("{object_ident}ComObject", span = object_ident.span());
    // Rust implementation generics stay inside the allocating binary, and COM
    // pointers cannot carry the lifetime of borrowed application storage.
    let generic = !input.generics.params.is_empty();
    if input.generics.lifetimes().next().is_some() {
        return syn::Error::new_spanned(
            &input.generics,
            "COM implementation objects must own their state; lifetime parameters are not supported",
        )
        .to_compile_error();
    }
    // A constructor's interface type shares the enclosing impl's type/const
    // namespace. It must also leave the concrete object name visible in its body.
    // Pick a free binder rather than reserving an authored spelling;
    // mixed-site spans alone cannot make duplicate generic parameters legal.
    let interface_parameter = (0..=input.generics.params.len() + 1)
        .map(|index| format_ident!("__NcInterface{index}", span = proc_macro2::Span::mixed_site()))
        .find(|candidate| *candidate != object_ident.unraw() && input.generics.params.iter().all(|parameter| match parameter {
            syn::GenericParam::Type(parameter) => parameter.ident.unraw() != *candidate,
            syn::GenericParam::Const(parameter) => parameter.ident.unraw() != *candidate,
            syn::GenericParam::Lifetime(_) => true,
        }))
        .expect("two extra candidates cover the object name and all authored parameters");
    let mut generics = input.generics.clone();
    let (_, input_type_generics, _) = input.generics.split_for_impl();
    if generic {
        let (_, types, _) = input.generics.split_for_impl();
        generics.make_where_clause().predicates.push(syn::parse_quote!(#object_ident #types: 'static));
    }
    if let Some(interface) = &args.primary_interface {
        // The IDL interface owns the threading mode. This bound accepts all
        // `'static` local implementations and additionally requires Send +
        // Sync when the interface selects the shared threading model.
        generics.make_where_clause().predicates.push(syn::parse_quote! {
            #object_ident #input_type_generics: #nc::__ComSupportsInterfaceThreading<
                <#interface as #nc::ComInterface>::Threading
            >
        });
    }
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let object_type = quote! { #object_ident #type_generics };
    let storage_type = quote! { #storage_ident #type_generics };

    let declared_threading_model = if let Some(interface) = &args.primary_interface {
        quote! { <#interface as #nc::ComInterface>::Threading }
    } else if args.thread_safe {
        quote! { #nc::ThreadSafe }
    } else {
        quote! { #nc::ThreadLocal }
    };
    // All generated pointer types project through this one implementation-level
    // threading declaration.
    let pointer_threading_model = quote! {
        <#object_type as #nc::ComObject>::Threading
    };
    let com_object_impl = quote! {
        // SAFETY: This associated mode is generated from the object attribute
        // and is reused by every generated pointer and interface association.
        unsafe impl #impl_generics #nc::ComObject for #object_type #where_clause {
            type Threading = #declared_threading_model;
        }
    };
    // Query order and exported metadata use the same derived-first ancestry.
    // Shared bases are advertised once even when several branches expose them.
    let declared_chains = args
        .interfaces
        .iter()
        .map(|iface| {
            let abi = &iface.abi_path;
            quote! { &#abi::INTERFACE_CHAIN }
        })
        .collect::<Vec<_>>();
    let interface_chains = quote! { &[
        &[&#nc::IUnknown::INTERFACE_DESCRIPTOR],
        #(#declared_chains,)*
    ] };
    // This expression contains contract types only, never implementation type
    // parameters. Fixed-size arrays therefore also work for generic Rust owners.
    let interface_count = quote! { #nc::__nc_object_interface_count(#interface_chains) };
    let providers = args
        .interfaces
        .iter()
        .map(|iface| {
            let provider = &iface.vtable_for_path;
            quote! { &#provider::<#object_type>::PROJECTIONS }
        })
        .collect::<Vec<_>>();
    let thread_safe_marker_impl = (args.thread_safe && args.primary_interface.is_none()).then(|| {
        let assertion = (!generic).then(|| {
            quote! {
                const _: fn() = || {
                    // Additional direct diagnostic for ordinary nongeneric objects.
                    fn assert_thread_safe<T: Send + Sync>() {}
                    assert_thread_safe::<#object_type>();
                };
            }
        });
        quote! {
            #assertion
            // SAFETY: ThreadSafeComObject's Send + Sync supertraits prove the
            // bounds for every permitted specialization. Interface behavior must
            // also uphold the documented concurrent-call contract.
            unsafe impl #impl_generics #nc::ThreadSafeComObject for #object_type #where_clause {}
        }
    });

    // Lifetime policy is a property of the provider class, not an interface.
    // Emit only legal constructors and select one class-specific borrowing policy.
    // Neither the associated Rust guard nor its layout crosses the C ABI.
    let (constructors, shared_guard, exclusive_guard, gate_field, acquire_shared, acquire_exclusive) = match args.ownership {
        ObjectOwnership::Shared => (
            quote! {
                /// Owns this exact implementation and permits checked local access
                /// between COM calls without downcasting or borrowing external state.
                #object_vis fn into_local(value: Self) -> #nc::LocalCom<Self> {
                    // SAFETY: from_value allocates this exact generated storage and
                    // transfers its canonical owning IUnknown. The local handle and
                    // all interface projections use this allocation's one call gate.
                    unsafe { #nc::LocalCom::__from_owned(Self::from_value(value)) }
                }

                #[doc = "Wraps a fully initialized Rust value as a COM object and returns its `IUnknown` interface."]
                #object_vis fn from_value(value: Self) -> #nc::ComPtr<#nc::IUnknown, #pointer_threading_model> {
                    // Construction initializes only ownership, borrow authority and state.
                    // Interface tables are static and add no per-object fields.
                    let object = #nc::__alloc::boxed::Box::new(#storage_ident {
                        control: #nc::lifetime::NcWeakControl::new(&<#storage_type>::CLASS_LIFETIME),
                        call_gate: #nc::__ComCallGate::new(),
                        value: core::cell::UnsafeCell::new(core::mem::MaybeUninit::new(value)),
                    });

                    // Publish the allocation paired with its canonical root table.
                    // The pair needs neither embedded headers nor owner recovery.
                    let raw = #nc::__alloc::boxed::Box::into_raw(object);
                    // SAFETY: Box::into_raw transfers the initial strong reference into COM ownership and
                    // remains live until the final generated `Release`.
                    let unknown = #nc::ComRaw { object: raw.cast(), vtable: &#nc::lifetime::CANONICAL_UNKNOWN_VTABLE };
                    unsafe {
                        // SAFETY: `unknown` is non-null and owns the initial
                        // strong reference initialized in the common header.
                        #nc::ComPtr::<#nc::IUnknown, #pointer_threading_model>::from_raw_owned(unknown)
                            .expect("new COM object produced a null pointer")
                    }
                }

                /// Initializes a weak self-origin before publishing the object.
                /// Promotion inside `build` returns None: construction has not yet
                /// published a strong value. The chosen interface is checked against
                /// this class's static projections before allocation or user code.
                #object_vis fn from_value_cyclic<#interface_parameter: #nc::ComInterface>(
                    build: impl FnOnce(#nc::WeakComPtr<#interface_parameter, #pointer_threading_model>) -> Self,
                ) -> Result<#nc::ComPtr<#nc::IUnknown, #pointer_threading_model>, #nc::NcStatus> {
                    // SAFETY: the generated table set and requested identity are immutable.
                    unsafe { #nc::__nc_find_interface(&Self::COM_VTABLES, &#interface_parameter::INTERFACE_ID) }?;
                    let storage = #nc::__alloc::boxed::Box::into_raw(#nc::__alloc::boxed::Box::new(#storage_ident {
                        control: #nc::lifetime::NcWeakControl::unpublished(&<#storage_type>::CLASS_LIFETIME),
                        call_gate: #nc::__ComCallGate::new(),
                        value: core::cell::UnsafeCell::new(core::mem::MaybeUninit::uninit()),
                    }));
                    // The provisional weak owner is the rollback guard. A panic in
                    // build releases it; escaped clones retain only uninitialized
                    // storage, which the deallocator frees without dropping Self.
                    let token = #nc::NcWeakRaw { control: storage.cast() };
                    // SAFETY: the exact requested projection was checked above;
                    // this new allocation transfers its provisional weak owner.
                    let provisional = unsafe { #nc::WeakComPtr::<#interface_parameter, #pointer_threading_model>::from_raw_owned(token) }
                        .expect("new weak control is present");
                    let value = build(provisional.clone());
                    // SAFETY: no strong owner exists yet. Initialize before the
                    // release publication observed by concurrent weak promotion.
                    unsafe {
                        (*(*storage).value.get()).write(value);
                        (*storage).control.publish();
                    }
                    // Publication converts the provisional owner into the implicit
                    // weak reference released only after final value destruction.
                    let _ = provisional.into_raw();
                    let raw = #nc::ComRaw { object: storage.cast(), vtable: &#nc::lifetime::CANONICAL_UNKNOWN_VTABLE };
                    // SAFETY: publication created exactly one initialized strong owner.
                    Ok(unsafe { #nc::ComPtr::from_raw_owned(raw) }.expect("new COM object is present"))
                }

                /// Creates a locally borrowable object with the same unpublished weak initialization.
                #object_vis fn into_local_cyclic<#interface_parameter: #nc::ComInterface>(
                    build: impl FnOnce(#nc::WeakComPtr<#interface_parameter, #pointer_threading_model>) -> Self,
                ) -> Result<#nc::LocalCom<Self>, #nc::NcStatus> {
                    // SAFETY: the generated constructor allocates exactly Self's storage.
                    Ok(unsafe { #nc::LocalCom::__from_owned(Self::from_value_cyclic::<#interface_parameter>(build)?) })
                }

            },
            quote! { #nc::__ComSharedCall<'a> },
            quote! { #nc::__ComExclusiveCall<'a> },
            quote! { call_gate: #nc::__ComCallGate, },
            quote! { #nc::__ComSharedLease::acquire(&storage.call_gate, storage.value.get().cast::<Self>()) },
            quote! { #nc::__ComExclusiveLease::acquire(&storage.call_gate, storage.value.get().cast::<Self>()) },
        ),
        ObjectOwnership::Exclusive => (
            quote! {
                /// Wraps a value in one non-clonable owner. Queries transfer or borrow
                /// this owner; AddRef and shared ownership conversion are rejected.
                #object_vis fn into_box(value: Self) -> #nc::ComBox<#nc::IUnknown, #pointer_threading_model> {
                    let storage = #nc::__alloc::boxed::Box::into_raw(#nc::__alloc::boxed::Box::new(#storage_ident {
                        control: #nc::lifetime::NcWeakControl::boxed(&<#storage_type>::CLASS_LIFETIME),
                        value: core::cell::UnsafeCell::new(core::mem::MaybeUninit::new(value)),
                    }));
                    let raw = #nc::ComRaw { object: storage.cast(), vtable: &#nc::lifetime::CANONICAL_UNKNOWN_VTABLE };
                    // SAFETY: this exact provider initializes and transfers one exclusive owner.
                    unsafe { #nc::ComBox::from_raw_owned(raw) }.expect("new exclusive object is present")
                }

                /// Keeps concrete local construction access until the owner moves into its container.
                #object_vis fn into_local_box(value: Self) -> #nc::LocalBox<Self> {
                    // SAFETY: into_box constructs exactly Self's generated allocation.
                    unsafe { #nc::LocalBox::__from_owned(Self::into_box(value)) }
                }
                /// Initializes exclusive local storage with an unpublished weak origin.
                #object_vis fn into_local_box_cyclic<#interface_parameter: #nc::ComInterface>(
                    build: impl FnOnce(#nc::WeakComBox<#interface_parameter, #pointer_threading_model>) -> Self,
                ) -> Result<#nc::LocalBox<Self>, #nc::NcStatus> {
                    // SAFETY: the generated constructor transfers exactly Self's exclusive storage.
                    Ok(unsafe { #nc::LocalBox::__from_owned(Self::into_box_cyclic::<#interface_parameter>(build)?) })
                }

                /// Initializes a weak origin without publishing or sharing ownership.
                /// Failed initialization releases storage without dropping an absent value.
                #object_vis fn into_box_cyclic<#interface_parameter: #nc::ComInterface>(
                    build: impl FnOnce(#nc::WeakComBox<#interface_parameter, #pointer_threading_model>) -> Self,
                ) -> Result<#nc::ComBox<#nc::IUnknown, #pointer_threading_model>, #nc::NcStatus> {
                    let table = unsafe { #nc::__nc_find_interface(&Self::COM_VTABLES, &#interface_parameter::INTERFACE_ID) }?;
                    let storage = #nc::__alloc::boxed::Box::into_raw(#nc::__alloc::boxed::Box::new(#storage_ident {
                        control: #nc::lifetime::NcWeakControl::boxed_unpublished(&<#storage_type>::CLASS_LIFETIME),
                        value: core::cell::UnsafeCell::new(core::mem::MaybeUninit::uninit()),
                    }));
                    let raw = #nc::ComRaw::<#interface_parameter> { object: storage.cast(), vtable: table.cast() };
                    // SAFETY: the provisional weak owns unpublished storage with a validated table.
                    let provisional = unsafe { #nc::WeakComBox::from_raw_owned(raw) }.expect("present control");
                    let value = build(provisional.clone());
                    unsafe {
                        (*(*storage).value.get()).write(value);
                        (*storage).control.publish_box();
                    }
                    let _ = provisional.into_raw();
                    let raw = #nc::ComRaw { object: storage.cast(), vtable: &#nc::lifetime::CANONICAL_UNKNOWN_VTABLE };
                    // SAFETY: publication transferred the provisional weak to the initialized owner.
                    Ok(unsafe { #nc::ComBox::from_raw_owned(raw) }.expect("present owner"))
                }

            },
            quote! { #nc::lifetime::BoxRead },
            quote! { #nc::lifetime::BoxWrite },
            quote! {},
            quote! { #nc::__ComSharedLease::acquire_box(object.cast(), storage.value.get().cast::<Self>()) },
            quote! { #nc::__ComExclusiveLease::acquire_box(object.cast(), storage.value.get().cast::<Self>()) },
        ),
    };

    quote! {
        #input

        // Storage order is private but still uses `repr(C)` to keep pointer
        // arithmetic and field addresses predictable for generated ABI thunks.
        /// One owned allocation for this exact implementation and all of its concrete interface projections.
        #[repr(C)]
        struct #storage_ident #impl_generics #where_clause {
            /// Common lifetime header at offset zero; shared operations never
            /// need to recover the concrete Rust payload to manage ownership.
            control: #nc::lifetime::NcWeakControl,
            // Shared ownership needs a separate gate because its header counts
            // owners. Exclusive storage uses only the header's combined state.
            #gate_field
            /// Owned Rust state; no reference into a caller's stack escapes through the COM pointer.
            value: core::cell::UnsafeCell<core::mem::MaybeUninit<#object_type>>,
        }

        impl #impl_generics #object_type #where_clause {
            /// Number of queryable contracts, including canonical IUnknown.
            /// The expression depends on contract types, never implementation generics.
            pub const COM_INTERFACE_COUNT: usize = #interface_count;

            /// Runtime queries consume only static vtable pointers for this implementation.
            /// Descriptor traversal and typed provider selection occur inside this constant;
            /// no full descriptor or duplicate query-identity table escapes into runtime use.
            const COM_VTABLES: [*const #nc::IUnknownVTable; #interface_count] = {
                let contracts = #nc::__nc_interface_chain::<{ #interface_count }>(
                    &#nc::IUnknown::INTERFACE_DESCRIPTOR, &[#(#declared_chains,)*],
                );
                let projections = #nc::__nc_projection_table(
                    &contracts,
                    // SAFETY: this allocation's canonical unknown table delegates
                    // lifecycle and queries to the same stored implementation.
                    unsafe { #nc::__ComProjection::<Self>::new(#nc::IUnknown::INTERFACE_ID, &#nc::lifetime::CANONICAL_UNKNOWN_VTABLE) },
                    &[#(#providers,)*],
                );
                #nc::__nc_projection_vtables(&projections)
            };

            #constructors

            }

        // SAFETY: This impl is generated together with the private storage
        // layout above. Every paired vtable accepts this allocation directly.
        unsafe impl #impl_generics #nc::ComAbiObject for #object_type #where_clause {
            /// Concrete provider policy; no dynamic ownership branch on shared calls.
            type SharedGuard<'a> = #shared_guard;
            type ExclusiveGuard<'a> = #exclusive_guard;
            // The generated gate permits concurrent observers but rejects a
            // shared call while a mutable call owns the implementation.
            unsafe fn borrow_abi_ref<'a>(
                object: *mut core::ffi::c_void,
            ) -> Result<#nc::__ComSharedLease<'a, Self, Self::SharedGuard<'a>>, #nc::NcStatus> {
                if object.is_null() {
                    return Err(#nc::NC_E_POINTER);
                }
                let storage = object.cast::<#storage_type>();
                // SAFETY: The caller guarantees that `object` points to this
                // generated storage, whose gate and value live together.
                let storage = unsafe { &*storage };
                // SAFETY: The value and gate are fields of the same live
                // storage allocation and every generated call uses this gate.
                unsafe { #acquire_shared }
            }

            // The generated gate grants exactly one mutable call only while no
            // shared or mutable call is active on any interface projection.
            unsafe fn borrow_abi_mut<'a>(
                object: *mut core::ffi::c_void,
            ) -> Result<#nc::__ComExclusiveLease<'a, Self, Self::ExclusiveGuard<'a>>, #nc::NcStatus> {
                if object.is_null() {
                    return Err(#nc::NC_E_POINTER);
                }
                let storage = object.cast::<#storage_type>();
                // SAFETY: The caller guarantees that `object` points to this
                // generated storage, whose gate and value live together.
                let storage = unsafe { &*storage };
                // SAFETY: The value and gate are fields of the same live
                // storage allocation and every generated call uses this gate.
                unsafe { #acquire_exclusive }
            }

        }

        #com_object_impl
        #thread_safe_marker_impl
        impl #impl_generics #storage_type #where_clause {
            /// One immutable class descriptor, independent of object instances.
            /// Counting/promotion functions are shared; only payload destruction,
            /// allocator release and this class's projections vary.
            const CLASS_LIFETIME: #nc::lifetime::NcClassLifetime = #nc::lifetime::NcClassLifetime {
                destroy_value: Self::destroy_value,
                deallocate: Self::deallocate,
                interfaces: <#object_type>::COM_VTABLES.as_ptr(),
                interface_count: <#object_type>::COM_VTABLES.len(),
            };
            /// Destroys initialized Self once while the implicit weak owner retains storage.
            unsafe extern "C" fn destroy_value(control: *mut #nc::lifetime::NcWeakControl) {
                let storage = control.cast::<Self>();
                // SAFETY: only final strong release calls this, after publication
                // and after all strong-owned calls finish. MaybeUninit suppresses
                // a second payload drop when the allocation is eventually freed.
                unsafe { (*(*storage).value.get()).assume_init_drop(); }
            }
            /// Frees the exact provider allocation, including its concrete alignment.
            /// The value is either already destroyed or was never initialized.
            unsafe extern "C" fn deallocate(control: *mut #nc::lifetime::NcWeakControl) {
                // SAFETY: the final weak owner transfers the allocation once.
                unsafe { drop(#nc::__alloc::boxed::Box::from_raw(control.cast::<Self>())); }
            }
        }
    }
}
