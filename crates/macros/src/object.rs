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
            /// Common lifetime header at offset zero; shared operations never
            /// need to recover the concrete Rust payload to manage ownership.
            control: #nc::lifetime::NcWeakControl,
            /// Generated borrow authority shared by every interface projection.
            call_gate: #nc::__ComCallGate,
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
                let unknown = #nc::ComRaw { object: raw.cast(), vtable: &<#storage_type>::UNKNOWN_VTABLE };
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
            #object_vis fn from_value_cyclic<I: #nc::ComInterface>(
                build: impl FnOnce(#nc::WeakComPtr<I, #pointer_threading_model>) -> Self,
            ) -> Result<#nc::ComPtr<#nc::IUnknown, #pointer_threading_model>, #nc::NcStatus> {
                // SAFETY: the generated table set and I's identity are immutable.
                unsafe { #nc::__nc_find_interface(&Self::COM_VTABLES, &I::INTERFACE_ID) }?;
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
                let provisional = unsafe { #nc::WeakComPtr::<I, #pointer_threading_model>::from_raw_owned(token) }
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
                let raw = #nc::ComRaw { object: storage.cast(), vtable: &<#storage_type>::UNKNOWN_VTABLE };
                // SAFETY: publication created exactly one initialized strong owner.
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
                unsafe { #nc::__ComSharedLease::acquire(&storage.call_gate, storage.value.get().cast::<Self>()) }
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
                unsafe { #nc::__ComExclusiveLease::acquire(&storage.call_gate, storage.value.get().cast::<Self>()) }
            }

        }

        #com_object_impl
        #thread_safe_marker_impl
        #factory_implementation
        #unit_factory_bootstrap

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

            /// Canonical strong root table, separate from the weak lifetime operations.
            const UNKNOWN_VTABLE: #nc::IUnknownVTable = #nc::IUnknownVTable {
                interface_id: &#nc::IUnknown::INTERFACE_ID,
                query_interface: #nc::lifetime::nc_query_interface,
                add_ref: #nc::lifetime::nc_strong_retain,
                release: #nc::lifetime::nc_strong_release,
                downgrade: Some(#nc::lifetime::nc_downgrade),
            };

        }
    }
}
