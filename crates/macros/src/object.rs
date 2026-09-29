//! Expansion of Rust structs into reference-counted multi-interface COM objects.
//!
//! The generator builds one private allocation containing an atomic reference
//! lifetime, one shared borrow guard, and the Rust value. Static tables carry interfaces.
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
            /// Generated borrow authority shared by every interface projection.
            call_gate: #nc::__ComCallGate,
            /// Owned Rust state; no reference into a caller's stack escapes through the COM pointer.
            value: core::cell::UnsafeCell<#object_type>,
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
                // Construction initializes only ownership, borrow authority and state.
                // Interface tables are static and add no per-object fields.
                let object = #nc::__alloc::sync::Arc::new(#storage_ident {
                    call_gate: #nc::__ComCallGate::new(),
                    value: core::cell::UnsafeCell::new(value),
                });

                // Publish the allocation paired with its canonical root table.
                // The pair needs neither embedded headers nor owner recovery.
                let raw = #nc::__alloc::sync::Arc::into_raw(object).cast_mut();
                // SAFETY: Arc::into_raw transfers a strong reference into COM ownership and
                // remains live until the final generated `Release`.
                let unknown = #nc::ComRaw { object: raw.cast(), vtable: &<#storage_type>::UNKNOWN_VTABLE };
                unsafe {
                    // SAFETY: `unknown` is non-null and owns the initial
                    // strong reference created by Arc::new.
                    #nc::ComPtr::<#nc::IUnknown, #pointer_threading_model>::from_raw_owned(unknown)
                        .expect("new COM object produced a null pointer")
                }
            }

            /// Initializes a weak self-origin before publishing the object.
            /// Promotion inside `build` returns None: construction has not yet
            /// published a strong value. The chosen interface is checked against
            /// this class's static projections before allocation or user code.
            #object_vis fn from_value_cyclic<I: #nc::ComInterface>(
                build: impl FnOnce(#nc::WeakComPtr<I, #pointer_threading_model>) -> Self,
            ) -> Result<#nc::ComPtr<#nc::IUnknown, #pointer_threading_model>, #nc::NcStatus> {
                // SAFETY: the generated table set and I's identity are immutable.
                unsafe { #nc::__nc_find_interface(&Self::COM_VTABLES, &I::INTERFACE_ID) }?;
                let object = #nc::__alloc::sync::Arc::new_cyclic(|weak: &#nc::__alloc::sync::Weak<#storage_type>| {
                    let raw = #nc::NcWeakRaw {
                        control: #nc::__alloc::sync::Weak::into_raw(weak.clone()).cast_mut().cast(),
                        vtable: &<#storage_type>::WEAK_VTABLE,
                    };
                    // SAFETY: the retained weak token uses this allocation's own
                    // callbacks; interface and threading are checked above.
                    let origin = unsafe { #nc::WeakComPtr::from_raw_owned(raw) }.expect("new weak token is present");
                    #storage_ident { call_gate: #nc::__ComCallGate::new(), value: core::cell::UnsafeCell::new(build(origin)) }
                });
                let raw = #nc::ComRaw {
                    object: #nc::__alloc::sync::Arc::into_raw(object).cast_mut().cast(),
                    vtable: &<#storage_type>::UNKNOWN_VTABLE,
                };
                // SAFETY: Arc::into_raw transfers the published initial strong owner.
                Ok(unsafe { #nc::ComPtr::from_raw_owned(raw) }.expect("new COM object is present"))
            }

            /// Creates a locally borrowable object with the same unpublished weak initialization.
            #object_vis fn into_local_cyclic<I: #nc::ComInterface>(
                build: impl FnOnce(#nc::WeakComPtr<I, #pointer_threading_model>) -> Self,
            ) -> Result<#nc::LocalCom<Self>, #nc::NcStatus> {
                // SAFETY: the generated constructor allocates exactly Self's storage.
                Ok(unsafe { #nc::LocalCom::__from_owned(Self::from_value_cyclic::<I>(build)?) })
            }

            #unit_create
        }

        #factory_assertion

        // SAFETY: This impl is generated together with the private storage
        // layout above. Every paired vtable accepts this allocation directly.
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
            // interface vtables point directly here, without adapter thunks.
            unsafe extern "C" fn query_interface_from_abi_object(
                object: *mut core::ffi::c_void,
                interface_id: *const #nc::NcInterfaceId,
                out: *mut #nc::ComRaw<#nc::IUnknown>,
            ) -> #nc::NcStatus {
                if object.is_null() { return #nc::NC_E_POINTER; }
                // SAFETY: this adapter knows the concrete storage layout. The
                // shared query body receives its retain callback explicitly and never
                // recovers fields from the opaque allocation address.
                unsafe {
                    #nc::__com_query_interfaces(
                        object,
                        Self::add_ref_from_abi_object,
                        &Self::COM_VTABLES,
                        interface_id,
                        out,
                    )
                }
            }

            /// Retains one strong owner; the returned count is a diagnostic snapshot.
            unsafe extern "C" fn add_ref_from_abi_object(object: *mut core::ffi::c_void) -> u32 {
                if object.is_null() { return 0; }
                let storage = object.cast::<#storage_type>();
                // SAFETY: caller retains a live Arc-backed object through this call.
                unsafe { #nc::__alloc::sync::Arc::increment_strong_count(storage); }
                // SAFETY: inspect the counter without consuming the caller's owner.
                let owner = core::mem::ManuallyDrop::new(unsafe { #nc::__alloc::sync::Arc::from_raw(storage) });
                #nc::__alloc::sync::Arc::strong_count(&owner).min(u32::MAX as usize) as u32
            }

            /// Consumes one strong owner, dropping the value at the final release.
            /// Weak tokens may retain the allocation shell, never the value/resources.
            unsafe extern "C" fn release_from_abi_object(object: *mut core::ffi::c_void) -> u32 {
                if object.is_null() { return 0; }
                // SAFETY: the caller transfers exactly one Arc-backed strong owner.
                let owner = unsafe { #nc::__alloc::sync::Arc::from_raw(object.cast::<#storage_type>()) };
                let remaining = #nc::__alloc::sync::Arc::strong_count(&owner).saturating_sub(1).min(u32::MAX as usize) as u32;
                drop(owner);
                remaining
            }

            /// Returns the same weak control identity from every interface projection.
            unsafe extern "C" fn downgrade_from_abi_object(object: *mut core::ffi::c_void, out: *mut #nc::NcWeakRaw) -> #nc::NcStatus {
                // SAFETY: this callback is published only for this exact Arc storage;
                // the shared helper receives its allocator-local weak table as data.
                unsafe { #nc::__com_arc_downgrade::<#storage_type>(object, out, &<#storage_type>::WEAK_VTABLE) }
            }

        }

        #com_object_impl
        #thread_safe_marker_impl
        #factory_implementation
        #unit_factory_bootstrap

        impl #impl_generics #storage_type #where_clause {
            /// Canonical root table. Every other table embeds the same lifecycle
            /// functions but advertises its own exact interface identity.
            const WEAK_VTABLE: #nc::NcWeakVTable = #nc::NcWeakVTable {
                retain: Self::retain_weak,
                release: Self::release_weak,
                upgrade: Self::upgrade_weak,
            };

            /// Clones only the weak control owner, including after value destruction.
            unsafe extern "C" fn retain_weak(control: *mut core::ffi::c_void) {
                // SAFETY: the weak protocol keeps the Arc control allocation alive.
                let weak = core::mem::ManuallyDrop::new(unsafe { #nc::__alloc::sync::Weak::<Self>::from_raw(control.cast()) });
                let _ = #nc::__alloc::sync::Weak::into_raw(#nc::__alloc::sync::Weak::clone(&weak));
            }
            /// Releases one weak owner through the same provider that allocated it.
            unsafe extern "C" fn release_weak(control: *mut core::ffi::c_void) {
                // SAFETY: consumes exactly the transferred weak reference.
                unsafe { drop(#nc::__alloc::sync::Weak::<Self>::from_raw(control.cast())); }
            }
            /// Promotes before querying, so no value/table access races final destruction.
            unsafe extern "C" fn upgrade_weak(
                control: *mut core::ffi::c_void,
                requested: *const #nc::NcInterfaceId,
                out: *mut #nc::ComRaw<#nc::IUnknown>,
            ) -> #nc::NcStatus {
                // SAFETY: this provider owns the typed Weak token and all matching
                // projection tables. Promotion transfers its acquired owner directly,
                // without a second QueryInterface retain and temporary Release.
                unsafe { #nc::__com_arc_upgrade::<Self>(control, requested, out, &<#object_type>::COM_VTABLES) }
            }

            /// Canonical strong root table, separate from the weak lifetime operations.
            const UNKNOWN_VTABLE: #nc::IUnknownVTable = #nc::IUnknownVTable {
                interface_id: &#nc::IUnknown::INTERFACE_ID,
                query_interface: <#object_type as #nc::ComAbiObject>::query_interface_from_abi_object,
                add_ref: <#object_type as #nc::ComAbiObject>::add_ref_from_abi_object,
                release: <#object_type as #nc::ComAbiObject>::release_from_abi_object,
                downgrade: Some(<#object_type as #nc::ComAbiObject>::downgrade_from_abi_object),
            };

        }
    }
}
