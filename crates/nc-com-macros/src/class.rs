//! Expansion of Rust structs into reference-counted multi-interface COM classes.
//!
//! The generator builds one private allocation containing an atomic reference
//! count, an `IUnknown` header, headers for each declared interface and its ancestors, and the Rust
//! value. It emits query/add-ref/release thunks, optional class descriptors and
//! constructors, threading assertions, and safe object-creation helpers.

use quote::{format_ident, quote};
use syn::{Fields, ItemStruct};

use crate::args::{ClassArgs, ConstructorArguments};
use crate::util::{
    create_visibility, runtime_crate_path, stable_guid, validate_version_literal,
};

/// Generates object storage, lifecycle thunks, interfaces, and optional metadata.
pub(crate) fn expand_com_class(args: ClassArgs, input: ItemStruct) -> proc_macro2::TokenStream {
    // `FooComObject` is intentionally private generated storage. Users see the
    // original Rust `Foo` plus COM interface pointers; the storage layout is an
    // implementation detail owned by this macro.
    let nc = runtime_crate_path(args.runtime_crate.as_ref());
    let class_ident = &input.ident;
    let class_vis = create_visibility(&input.vis);
    let storage_ident = format_ident!("{class_ident}ComObject");
    let declared_threading_model = if args.thread_safe {
        quote! { #nc::ThreadSafe }
    } else {
        quote! { #nc::ThreadLocal }
    };
    // All generated pointer types, descriptor metadata, and constructor checks
    // project through this one class-level threading declaration.
    let pointer_threading_model = quote! {
        <#class_ident as #nc::ComClass>::Threading
    };
    let class_threading = quote! {
        <<#class_ident as #nc::ComClass>::Threading as #nc::ThreadingModel>::ABI_THREADING_MODE
            .as_raw()
    };
    let com_class_impl = quote! {
        // SAFETY: This associated mode is generated from the class attribute and
        // is reused by every generated pointer, descriptor, and constructor.
        unsafe impl #nc::ComClass for #class_ident {
            type Threading = #declared_threading_model;
        }
    };
    let is_unit_struct = matches!(input.fields, Fields::Unit);
    // Query order and exported metadata use the same derived-first ancestry.
    // Shared bases are advertised once even when several branches expose them.
    let interface_chains = args.interfaces.iter().map(|iface| {
        let abi = &iface.abi_path;
        quote! { &#abi::INTERFACE_CHAIN }
    });
    let thread_safe_marker_impl = args.thread_safe.then(|| {
        quote! {
            const _: fn() = || {
                // Forces a compile-time proof for the generated thread-safe marker.
                fn assert_thread_safe<T: Send + Sync>() {}
                assert_thread_safe::<#class_ident>();
            };

            // SAFETY: The compile-time assertion above requires the Rust class
            // to be `Send + Sync`; the user is also responsible for the
            // `ThreadSafeComObject` contract on exposed method behavior.
            unsafe impl #nc::ThreadSafeComObject for #class_ident {}
        }
    });
    // Every class can be constructed from an initialized Rust value. Unit
    // structs also get the ergonomic `create()` shortcut because no fields need
    // caller-provided state.
    let unit_create = is_unit_struct.then(|| {
        quote! {
            #[doc = "Creates a COM object from this unit struct and returns its `IUnknown` interface."]
            #class_vis fn create() -> #nc::ComPtr<#nc::IUnknown, #pointer_threading_model> {
                Self::from_value(#class_ident)
            }
        }
    });

    // Each exposed interface gets its own header inside the same allocation.
    // These headers may live at different addresses, so user-visible casting
    // must always go through QueryInterface rather than pointer casts.
    let interface_inits = args.interfaces.iter().map(|iface| {
        let interfaces_for = &iface.interfaces_for_path;
        let field = &iface.field_ident;
        quote! {
            #field: #interfaces_for::<#class_ident>::__nc_new()
        }
    });

    let interface_fields = args.interfaces.iter().map(|iface| {
        let interfaces_for = &iface.interfaces_for_path;
        let field = &iface.field_ident;
        quote! { #field: #interfaces_for<#class_ident> }
    });

    // The object pointer is not known until the Box allocation exists. Headers
    // are initialized with null and patched immediately before leaking the Box
    // into reference-counted COM ownership. The Box path is reached through
    // nc-com's hidden alloc re-export so no_std users do not need their own
    // `extern crate alloc` just to use generated classes.
    let set_interface_objects = args.interfaces.iter().map(|iface| {
        let field = &iface.field_ident;
        quote! { (*raw).#field.set_object(raw.cast::<core::ffi::c_void>()); }
    });

    // QueryInterface matches stable IIDs and returns the address of the
    // matching interface header. Returning a pointer also transfers one owned
    // reference, so every successful arm calls AddRef first.
    let query_projections = args.interfaces.iter().map(|iface| {
        let field = &iface.field_ident;
        quote! {
            // SAFETY: The shared borrow only obtains a header address in live
            // class storage; query never changes object state.
            let projection = match unsafe { (*storage).#field.query(interface_id) } {
                Ok(pointer) => pointer,
                Err(status) => return status,
            };
            if !projection.is_null() {
                // SAFETY: The enclosing object is live and `out` is writable.
                // Exactly one reference is transferred after finding the header.
                unsafe {
                    Self::add_ref_from_abi_object(object);
                    *out = projection;
                }
                return #nc::NC_OK;
            }
        }
    });
    let (class_descriptor_items, class_descriptor_module_items) = match args.descriptor.as_ref() {
        None => (quote! {}, quote! {}),
        Some(descriptor) => {
            let class_id = match stable_guid(descriptor.class_id.clone(), &descriptor.name, &nc) {
                Ok(class_id) => class_id,
                Err(err) => return err.to_compile_error(),
            };
            let name = &descriptor.name;
            let version = &descriptor.version;
            if let Err(err) = validate_version_literal(version) {
                return err.to_compile_error();
            }
            let constructor_args = match &descriptor.constructor.arguments {
                ConstructorArguments::None => quote! { #nc::NcInterfaceId::NONE },
                ConstructorArguments::Interface { abi_path } => {
                    quote! { <#abi_path as #nc::ComInterface>::INTERFACE_ID }
                }
            };
            let generated_constructor_ident = format_ident!(
                "__nc_com_construct_{}",
                class_ident.to_string().to_lowercase()
            );
            let descriptor_static_ident =
                format_ident!("__NC_COM_CLASS_DESCRIPTOR_{}", class_ident);
            let constructor_fn = descriptor
                .constructor
                .function
                .as_ref()
                .map(|function| quote! { #function })
                .unwrap_or_else(|| quote! { Self::#generated_constructor_ident });
            if !is_unit_struct
                && descriptor.constructor.function.is_none()
                && !descriptor.constructor.use_default
            {
                return syn::Error::new_spanned(
                    class_ident,
                    "generated no-argument constructors for non-unit COM classes require `constructor(args = none, default)` or `constructor(..., function = path)`",
                )
                .to_compile_error();
            }
            let default_value = if is_unit_struct {
                quote! { #class_ident }
            } else {
                quote! { <#class_ident as ::core::default::Default>::default() }
            };
            let generated_constructor = if descriptor.constructor.function.is_some() {
                quote! {}
            } else {
                quote! {
                    // Default descriptor constructors are intentionally limited
                    // to `args = none`: the macro can allocate the COM object,
                    // but it cannot invent domain-specific argument parsing.
                    unsafe extern "C" fn #generated_constructor_ident(
                        class: *const #nc::NcClassDescriptor,
                        arguments: *mut #nc::IUnknown,
                        out: *mut *mut #nc::IUnknown,
                    ) -> #nc::NcStatus {
                        // SAFETY: This is the raw constructor entrypoint stored
                        // in the generated class descriptor. The checked helper
                        // validates descriptor identity, argument shape, and
                        // output storage before publishing the object pointer.
                        unsafe {
                            #nc::construct_class_checked::<#pointer_threading_model, _>(
                                class,
                                arguments,
                                out,
                                #class_ident::CLASS_ID,
                                #nc::NcInterfaceId::NONE,
                                |_arguments| {
                                    let value = #default_value;
                                    let unknown = #class_ident::from_value(value);
                                    Ok(unknown)
                                },
                            )
                        }
                    }
                }
            };

            let impl_items = quote! {
                #[doc = "Stable class identifier used by generated class metadata."]
                pub const CLASS_ID: #nc::NcGuid = #class_id;

                #[doc = "Class descriptor value consumed by `nc_com::plugin!`."]
                pub const CLASS_DESCRIPTOR: #nc::NcClassDescriptor = #nc::NcClassDescriptor {
                    class_id: Self::CLASS_ID,
                    qualified_name: #nc::NcStaticUtf8::from_static(#name),
                    class_version: #nc::NcStaticUtf8::from_static(#version),
                    constructor_args: #constructor_args,
                    threading: #class_threading,
                    construct: Some(#constructor_fn as #nc::NcConstructorFn),
                    interface_count: Self::COM_INTERFACE_COUNT as u32,
                    interfaces: Self::COM_INTERFACES.as_ptr(),
                };

                #[doc = "Returns this class's generated static class descriptor."]
                #class_vis fn class_descriptor() -> &'static #nc::NcClassDescriptor {
                    &#descriptor_static_ident
                }

                #generated_constructor
            };
            let module_items = quote! {
                // Host registration APIs borrow descriptor pointers beyond the
                // immediate call. Keep a real static descriptor address for
                // direct in-process registration, while `plugin!` can still
                // copy the const descriptor into its plugin-owned class table.
                #[allow(non_upper_case_globals)]
                static #descriptor_static_ident: #nc::NcClassDescriptor =
                    #class_ident::CLASS_DESCRIPTOR;
            };
            (impl_items, module_items)
        }
    };

    quote! {
        #input

        // Storage order is private but still uses `repr(C)` to keep pointer
        // arithmetic and field addresses predictable for generated ABI thunks.
        #[repr(C)]
        struct #storage_ident {
            ref_count: ::core::sync::atomic::AtomicU32,
            unknown: #nc::IUnknown,
            #(#interface_fields,)*
            value: #class_ident,
        }

        impl #class_ident {
            /// Static ancestry inputs; canonical IUnknown always comes first.
            const COM_INTERFACE_CHAINS: &'static [&'static [&'static #nc::NcInterfaceDescriptor]] = &[
                &[&#nc::IUnknown::INTERFACE_DESCRIPTOR],
                #(#interface_chains,)*
            ];

            /// Number of unique contracts, including automatically exposed bases.
            pub const COM_INTERFACE_COUNT: usize = #nc::__nc_class_interface_count(Self::COM_INTERFACE_CHAINS);

            /// All supported contracts in deterministic QueryInterface order.
            pub const COM_INTERFACES: [#nc::NcClassInterfaceDescriptor; Self::COM_INTERFACE_COUNT] =
                #nc::__nc_class_interfaces(Self::COM_INTERFACE_CHAINS);

            #class_descriptor_items

            #[doc = "Wraps a fully initialized Rust value as a COM object and returns its `IUnknown` interface."]
            #class_vis fn from_value(value: Self) -> #nc::ComPtr<#nc::IUnknown, #pointer_threading_model> {
                let object = #nc::__alloc::boxed::Box::new(#storage_ident {
                    ref_count: ::core::sync::atomic::AtomicU32::new(1),
                    unknown: #nc::IUnknown {
                        vtable: &#storage_ident::UNKNOWN_VTABLE,
                        object: core::ptr::null_mut(),
                    },
                    #(#interface_inits,)*
                    value,
                });

                let raw = #nc::__alloc::boxed::Box::into_raw(object);
                unsafe {
                    // SAFETY: `raw` was just produced by `Box::into_raw` and is
                    // uniquely initialized here before any interface pointer is
                    // published.
                    (*raw).unknown.object = raw.cast::<core::ffi::c_void>();
                    #(#set_interface_objects)*
                }
                // SAFETY: The allocation has been leaked into COM ownership and
                // remains live until the final generated `Release`.
                let unknown = unsafe { core::ptr::addr_of_mut!((*raw).unknown) };
                unsafe {
                    // SAFETY: `unknown` is non-null and owns the initial
                    // reference created with `ref_count = 1`.
                    #nc::ComPtr::<#nc::IUnknown, #pointer_threading_model>::from_raw_owned(unknown)
                        .expect("new COM object produced a null pointer")
                }
            }

            #unit_create
        }

        #class_descriptor_module_items

        // SAFETY: This impl is generated together with the private storage
        // layout above. Every object pointer accepted here is the back-pointer
        // installed in one of that storage allocation's interface headers.
        unsafe impl #nc::ComAbiObject for #class_ident {
            // The interface thunk has only the opaque object pointer. This
            // method maps it back to the Rust value so the original trait impl
            // can be called.
            unsafe fn value_from_abi_object(object: *mut core::ffi::c_void) -> *mut Self {
                let storage = object.cast::<#storage_ident>();
                // SAFETY: `object` is guaranteed by the generated interface
                // header to point at this class's storage allocation.
                unsafe { core::ptr::addr_of_mut!((*storage).value) }
            }

            // Central QueryInterface implementation for the class. All
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
                let storage = object.cast::<#storage_ident>();
                // SAFETY: `interface_id` was checked non-null and is readable
                // for this call.
                let interface_id = unsafe { &*interface_id };
                match interface_id {
                    interface_id if interface_id.iid == <#nc::IUnknown as #nc::ComInterface>::IID => {
                        if *interface_id != <#nc::IUnknown as #nc::ComInterface>::INTERFACE_ID {
                            return #nc::NC_E_ABI_VERSION;
                        }
                        // SAFETY: `object` is this class's live storage pointer.
                        unsafe { Self::add_ref_from_abi_object(object); }
                        // SAFETY: `storage` points to the enclosing generated
                        // storage and `out` was checked non-null.
                        unsafe { *out = core::ptr::addr_of_mut!((*storage).unknown).cast::<core::ffi::c_void>(); }
                        #nc::NC_OK
                    }
                    _ => {
                        #(#query_projections)*
                        #nc::NC_E_NOINTERFACE
                    },
                }
            }

            // Relaxed increment is enough for AddRef itself; Release uses
            // release/acquire ordering around destruction.
            unsafe fn add_ref_from_abi_object(object: *mut core::ffi::c_void) -> u32 {
                if object.is_null() {
                    return 0;
                }
                let storage = object.cast::<#storage_ident>();
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
                let storage = object.cast::<#storage_ident>();
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

        #com_class_impl
        #thread_safe_marker_impl

        impl #storage_ident {
            // The standalone IUnknown header uses this vtable. Interface
            // headers use their interface-specific vtables, which embed the
            // same three lifecycle operations as the first field.
            const UNKNOWN_VTABLE: #nc::IUnknownVTable = #nc::IUnknownVTable {
                interface_id: &#nc::IUnknown::INTERFACE_ID,
                query_interface: Self::query_interface_thunk,
                add_ref: Self::add_ref_thunk,
                release: Self::release_thunk,
            };

            // Adapts the standalone IUnknown header to generated class-level
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
                // this class and carries the storage back-pointer.
                let object = unsafe { (*this).object };
                // SAFETY: Delegates to the class implementation, which validates
                // null `object`, `interface_id`, and `out` before
                // dereferencing.
                unsafe {
                    <#class_ident as #nc::ComAbiObject>::query_interface_from_abi_object(
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
                // this class and carries the storage back-pointer.
                let object = unsafe { (*this).object };
                // SAFETY: Delegates to generated storage refcount handling.
                unsafe { <#class_ident as #nc::ComAbiObject>::add_ref_from_abi_object(object) }
            }

            // Routes IUnknown Release to final-drop-aware generated storage.
            unsafe extern "C" fn release_thunk(this: *mut #nc::IUnknown) -> u32 {
                if this.is_null() {
                    return 0;
                }
                // SAFETY: Non-null `this` is an interface header generated by
                // this class and carries the storage back-pointer.
                let object = unsafe { (*this).object };
                // SAFETY: Delegates to generated storage refcount handling.
                unsafe { <#class_ident as #nc::ComAbiObject>::release_from_abi_object(object) }
            }
        }
    }
}
