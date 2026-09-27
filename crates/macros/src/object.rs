//! Expansion of Rust structs into reference-counted multi-interface COM objects.
//!
//! The generator builds one private allocation containing an atomic reference
//! count, one canonical array of distinct interface headers, and the Rust value.
//! Metadata, table bindings, and QueryInterface share the same compile-time set.
//! Generated lifecycle thunks, threading assertions, and construction helpers
//! preserve the one allocation and its concrete implementation type.

use quote::{format_ident, quote};
use syn::{Fields, ItemStruct};

use crate::args::ObjectArgs;
use crate::util::{create_visibility, runtime_crate_path};

/// Generates object storage, lifecycle thunks, and local construction helpers.
pub(crate) fn expand_com_object(args: ObjectArgs, input: ItemStruct, require_factory: bool) -> proc_macro2::TokenStream {
    // `FooComObject` is intentionally private generated storage. Users see the
    // original Rust `Foo` plus COM interface pointers; the storage layout is an
    // implementation detail owned by this macro.
    let nc = runtime_crate_path(args.runtime_crate.as_ref());
    let object_ident = &input.ident;
    let object_vis = create_visibility(&input.vis);
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
    let is_unit_struct = matches!(input.fields, Fields::Unit);
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
    let factory_assertion = require_factory.then(|| {
        let interface = args.primary_interface.as_ref().expect("com_factory validates a primary interface");
        quote! {
            const _: fn() = || {
                // This compile-time-only obligation rejects an IDL object passed
                // accidentally to `com_factory` without adding runtime tags.
                fn assert_factory<T: #nc::ComFactoryInterface>() {}
                assert_factory::<#interface>();
            };
        }
    });
    let factory_implementation = require_factory.then(|| {
        let interface = args.primary_interface.as_ref().expect("com_factory validates a primary interface");
        quote! {
            // SAFETY: `com_factory` generates the storage and complete
            // interface projection set directly from this IDL interface.
            unsafe impl #impl_generics #nc::ComFactoryImplementation for #object_type #where_clause {
                /// Exact IDL factory interface implemented by this Rust type.
                type Interface = #interface;
            }
        }
    });
    let unit_factory_bootstrap = (require_factory && is_unit_struct).then(|| {
        let interface = args.primary_interface.as_ref().expect("com_factory validates a primary interface");
        quote! {
            impl #impl_generics #nc::ComFactoryBootstrap for #object_type #where_clause {
                /// Creates the stateless root and returns its exact typed factory projection.
                fn create_factory() -> Result<
                    #nc::ComPtr<
                        #interface,
                        <#interface as #nc::ComInterface>::Threading,
                    >,
                    #nc::NcStatus,
                > {
                    // QueryInterface converts the newly allocated IUnknown
                    // owner into the exact factory interface.
                    Self::create().query::<#interface>()
                }
            }
        }
    });
    // Every object wraps an explicitly initialized Rust value. Only factories
    // receive zero-argument construction: typed factory methods own all other
    // public construction policy.
    let unit_create = (require_factory && is_unit_struct).then(|| {
        quote! {
            #[doc = "Creates a COM object from this unit struct and returns its `IUnknown` interface."]
            #object_vis fn create() -> #nc::ComPtr<#nc::IUnknown, #pointer_threading_model> {
                Self::from_value(#object_ident)
            }
        }
    });

    quote! {
        #input

        // Storage order is private but still uses `repr(C)` to keep pointer
        // arithmetic and field addresses predictable for generated ABI thunks.
        /// One owned allocation for this exact implementation and all of its concrete interface projections.
        #[repr(C)]
        struct #storage_ident #impl_generics #where_clause {
            /// Allocation-wide ownership count; final Release destroys the stored specialization once.
            ref_count: ::core::sync::atomic::AtomicU32,
            /// Generated borrow authority shared by every interface projection.
            call_gate: #nc::__ComCallGate,
            /// One two-pointer C header per distinct contract; canonical IUnknown is first.
            interfaces: [#nc::IUnknown; #interface_count],
            /// Owned Rust state; no reference into a caller's stack escapes through the COM pointer.
            value: core::cell::UnsafeCell<#object_type>,
        }

        impl #impl_generics #object_type #where_clause {
            /// Number of physically stored headers, including canonical IUnknown.
            /// The expression depends on contract types, never implementation generics.
            pub const COM_INTERFACE_COUNT: usize = #interface_count;

            /// Runtime construction consumes only vtable pointers for this implementation.
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
                    unsafe { #nc::__ComProjection::<Self>::new(#nc::IUnknown::INTERFACE_ID, &<#storage_type>::UNKNOWN_VTABLE) },
                    &[#(#providers,)*],
                );
                #nc::__nc_projection_vtables(&projections)
            };

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
                // The array length and associated table are evaluated during
                // compilation. Runtime construction only initializes those slots.
                let object = #nc::__alloc::boxed::Box::new(#storage_ident {
                    ref_count: ::core::sync::atomic::AtomicU32::new(1),
                    call_gate: #nc::__ComCallGate::new(),
                    interfaces: ::core::array::from_fn(|index| #nc::IUnknown {
                        vtable: Self::COM_VTABLES[index],
                        object: core::ptr::null_mut(),
                    }),
                    value: core::cell::UnsafeCell::new(value),
                });

                // Establish raw ownership before creating self-referential
                // back-pointers; no subsequent Box borrow can retag the owner.
                let raw = #nc::__alloc::boxed::Box::into_raw(object);
                // SAFETY: this fresh allocation is still unpublished, so the
                // initializer has exclusive mutable access to every header.
                unsafe { (*raw).initialize_interfaces(raw.cast::<core::ffi::c_void>()); }
                // SAFETY: The allocation has been leaked into COM ownership and
                // remains live until the final generated `Release`.
                let unknown = unsafe { core::ptr::addr_of_mut!((*raw).interfaces[0]) };
                unsafe {
                    // SAFETY: `unknown` is non-null and owns the initial
                    // reference created with `ref_count = 1`.
                    #nc::ComPtr::<#nc::IUnknown, #pointer_threading_model>::from_raw_owned(unknown)
                        .expect("new COM object produced a null pointer")
                }
            }

            #unit_create
        }

        #factory_assertion

        // SAFETY: This impl is generated together with the private storage
        // layout above. Every object pointer accepted here is the back-pointer
        // installed in one of that storage allocation's interface headers.
        unsafe impl #impl_generics #nc::ComAbiObject for #object_type #where_clause {
            // The generated gate permits concurrent observers but rejects a
            // shared call while a mutable call owns the implementation.
            unsafe fn borrow_abi_ref<'a>(
                object: *mut core::ffi::c_void,
            ) -> Result<#nc::__ComSharedLease<'a, Self>, #nc::NcStatus> {
                if object.is_null() {
                    return Err(#nc::NC_E_POINTER);
                }
                let storage = object.cast::<#storage_type>();
                // SAFETY: The caller guarantees that `object` points to this
                // generated storage, whose gate and value live together.
                let storage = unsafe { &*storage };
                // SAFETY: The value and gate are fields of the same live
                // storage allocation and every generated call uses this gate.
                unsafe { #nc::__ComSharedLease::acquire(&storage.call_gate, storage.value.get()) }
            }

            // The generated gate grants exactly one mutable call only while no
            // shared or mutable call is active on any interface projection.
            unsafe fn borrow_abi_mut<'a>(
                object: *mut core::ffi::c_void,
            ) -> Result<#nc::__ComExclusiveLease<'a, Self>, #nc::NcStatus> {
                if object.is_null() {
                    return Err(#nc::NC_E_POINTER);
                }
                let storage = object.cast::<#storage_type>();
                // SAFETY: The caller guarantees that `object` points to this
                // generated storage, whose gate and value live together.
                let storage = unsafe { &*storage };
                // SAFETY: The value and gate are fields of the same live
                // storage allocation and every generated call uses this gate.
                unsafe { #nc::__ComExclusiveLease::acquire(&storage.call_gate, storage.value.get()) }
            }

            // Central QueryInterface implementation for the object. All
            // interface-specific IUnknown thunks delegate here.
            unsafe fn query_interface_from_abi_object(
                object: *mut core::ffi::c_void,
                interface_id: *const #nc::NcInterfaceId,
                out: *mut *mut core::ffi::c_void,
            ) -> #nc::NcStatus {
                if object.is_null() || interface_id.is_null() || out.is_null() {
                    return #nc::NC_E_POINTER;
                }
                // SAFETY: `out` was checked non-null and is valid for one
                // pointer write under the raw ABI contract.
                unsafe { *out = core::ptr::null_mut(); }
                let storage = object.cast::<#storage_type>();
                // SAFETY: `interface_id` was checked non-null and is readable
                // for this call.
                let interface_id = unsafe { &*interface_id };
                // SAFETY: all headers are initialized before publication and
                // remain immutable until final Release. Their vtable prefixes
                // carry exact generated identities, independent of inspection data.
                match unsafe { #nc::__nc_find_interface(&(*storage).interfaces, interface_id) } {
                    Ok(projection) => {
                        // SAFETY: the live allocation transfers exactly one
                        // reference after a successful identity/signature match.
                        unsafe {
                            Self::add_ref_from_abi_object(object);
                            *out = projection.cast::<core::ffi::c_void>();
                        }
                        #nc::NC_OK
                    }
                    Err(status) => status,
                }
            }

            // Relaxed increment is enough for AddRef itself; Release uses
            // release/acquire ordering around destruction.
            unsafe fn add_ref_from_abi_object(object: *mut core::ffi::c_void) -> u32 {
                if object.is_null() {
                    return 0;
                }
                let storage = object.cast::<#storage_type>();
                // SAFETY: `object` is a live generated storage pointer whose
                // refcount field remains valid until final Release.
                unsafe { #nc::__com_add_ref_count(&(*storage).ref_count) }
            }

            // The final Release reconstructs the Box and drops the complete
            // generated storage, including the original Rust value and any COM
            // children it owns.
            unsafe fn release_from_abi_object(object: *mut core::ffi::c_void) -> u32 {
                if object.is_null() {
                    return 0;
                }
                let storage = object.cast::<#storage_type>();
                // SAFETY: `object` is a live generated storage pointer. A zero
                // remaining count grants this call exclusive final-drop rights.
                let Some(remaining) = (unsafe { #nc::__com_release_ref_count(&(*storage).ref_count) }) else {
                    return 0;
                };
                if remaining == 0 {
                    // SAFETY: The allocation was originally leaked by
                    // `Box::into_raw` in `from_value`; final Release rebuilds
                    // and drops that Box exactly once.
                    unsafe { drop(#nc::__alloc::boxed::Box::from_raw(storage)); }
                }
                remaining
            }
        }

        #com_object_impl
        #thread_safe_marker_impl
        #factory_implementation
        #unit_factory_bootstrap

        impl #impl_generics #storage_type #where_clause {
            /// Installs back-pointers through exclusive access before publication.
            /// The caller supplies this allocation's stable raw ownership pointer.
            fn initialize_interfaces(&mut self, object: *mut core::ffi::c_void) {
                for header in &mut self.interfaces {
                    header.object = object;
                }
            }

            // The standalone IUnknown header uses this vtable. Interface
            // headers use their interface-specific vtables, which embed the
            // same three lifecycle operations as the first field.
            const UNKNOWN_VTABLE: #nc::IUnknownVTable = #nc::IUnknownVTable {
                interface_id: &#nc::IUnknown::INTERFACE_ID,
                query_interface: Self::query_interface_thunk,
                add_ref: Self::add_ref_thunk,
                release: Self::release_thunk,
            };

            // Adapts the standalone IUnknown header to generated object-level
            // interface negotiation through its opaque storage back-pointer.
            unsafe extern "C" fn query_interface_thunk(
                this: *mut #nc::IUnknown,
                interface_id: *const #nc::NcInterfaceId,
                out: *mut *mut core::ffi::c_void,
            ) -> #nc::NcStatus {
                if this.is_null() {
                    return #nc::NC_E_POINTER;
                }
                // SAFETY: Non-null `this` is an interface header generated by
                // this object and carries the storage back-pointer.
                let object = unsafe { (*this).object };
                // SAFETY: Delegates to the object implementation, which validates
                // null `object`, `interface_id`, and `out` before
                // dereferencing.
                unsafe {
                    <#object_type as #nc::ComAbiObject>::query_interface_from_abi_object(
                        object,
                        interface_id,
                        out,
                    )
                }
            }

            // Routes IUnknown AddRef to the allocation-wide generated count.
            unsafe extern "C" fn add_ref_thunk(this: *mut #nc::IUnknown) -> u32 {
                if this.is_null() {
                    return 0;
                }
                // SAFETY: Non-null `this` is an interface header generated by
                // this object and carries the storage back-pointer.
                let object = unsafe { (*this).object };
                // SAFETY: Delegates to generated storage refcount handling.
                unsafe { <#object_type as #nc::ComAbiObject>::add_ref_from_abi_object(object) }
            }

            // Routes IUnknown Release to final-drop-aware generated storage.
            unsafe extern "C" fn release_thunk(this: *mut #nc::IUnknown) -> u32 {
                if this.is_null() {
                    return 0;
                }
                // SAFETY: Non-null `this` is an interface header generated by
                // this object and carries the storage back-pointer.
                let object = unsafe { (*this).object };
                // SAFETY: Delegates to generated storage refcount handling.
                unsafe { <#object_type as #nc::ComAbiObject>::release_from_abi_object(object) }
            }
        }
    }
}
