//! Binds Rust implementations to static COM interfaces and typed storage.
//!
//! Each private allocation owns the common lifetime header, ownership-specific
//! borrow authority, and a fully initialized Rust value. Static tables carry interfaces.
//! Metadata, table bindings, and QueryInterface share the same compile-time set.
//! Typed storage helpers, threading proofs, and generated constructors
//! preserve the one allocation and its concrete implementation type.
//! Weak references can retain storage after value destruction, but no constructor
//! exposes an allocation before initializing that value.

use quote::quote;
use syn::ItemStruct;

use crate::args::{ObjectArgs, ObjectOwnership};
use crate::util::{PrivateNames, runtime_crate_path};

/// Generates compact interface bindings, typed borrow adapters and legal constructors.
pub fn expand_com_object(args: ObjectArgs, input: ItemStruct) -> proc_macro2::TokenStream {
    let nc = runtime_crate_path(args.runtime_crate.as_ref());
    let declared_paths = args.interfaces.iter().map(|interface| &interface.abi_path);
    let mut names = PrivateNames::new(quote! { #input #nc #(#declared_paths)* });
    let object_pointer = names.binding("__nc_object_pointer");
    let initial = names.binding("__nc_initial_value");
    let contracts = names.binding("__nc_contracts");
    let projections = names.binding("__nc_projections");
    let object_ident = &input.ident;
    let object_vis = &input.vis;
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
    let declared_threading_model = if let Some(interface) = &args.primary_interface {
        quote! { <#interface as #nc::ComInterface>::Threading }
    } else if args.thread_safe {
        quote! { #nc::ThreadSafe }
    } else {
        quote! { #nc::ThreadLocal }
    };
    // One proof route for both primary contracts and independent projections.
    generics.make_where_clause().predicates.push(syn::parse_quote! {
        #object_ident #input_type_generics: #nc::__ComSupportsInterfaceThreading<#declared_threading_model>
    });
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let object_type = quote! { #object_ident #type_generics };

    // All generated pointer types project through this one implementation-level
    // threading declaration.
    let pointer_threading_model = quote! {
        <#object_type as #nc::ComObject>::Threading
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
    // Only the selected provider policy exists; shared calls have no dynamic
    // exclusive-admission branch. All unsafe layout work belongs to typed support.
    let (storage, shared_guard, exclusive_guard, constructors) = match args.ownership {
        ObjectOwnership::Shared => (
            quote! { #nc::__ComSharedStorage<Self> },
            quote! { #nc::__ComSharedCall<'a> },
            quote! { #nc::__ComExclusiveCall<'a> },
            quote! {
                /// Owns this exact implementation with checked local access.
                #object_vis fn into_local(#initial: Self) -> #nc::LocalCom<Self> {
                    // SAFETY: the generated constructor proves Self's exact storage.
                    unsafe { #nc::LocalCom::__from_owned(Self::from_value(#initial)) }
                }
                /// Wraps a complete Rust value in one shared COM allocation.
                #object_vis fn from_value(#initial: Self) -> #nc::ComPtr<#nc::IUnknown, #pointer_threading_model> {
                    // SAFETY: this impl's borrow adapters select the same shared layout.
                    unsafe { #nc::__ComSharedStorage::<Self>::into_shared(#initial) }
                }
            },
        ),
        ObjectOwnership::Exclusive => (
            quote! { #nc::__ComBoxStorage<Self> },
            quote! { #nc::lifetime::BoxRead },
            quote! { #nc::lifetime::BoxWrite },
            quote! {
                /// Transfers one exclusive owner of a fully initialized value.
                #object_vis fn into_box(#initial: Self) -> #nc::ComBox<#nc::IUnknown, #pointer_threading_model> {
                    // SAFETY: borrow adapters use this layout's header admission.
                    unsafe { #nc::__ComBoxStorage::<Self>::into_exclusive(#initial) }
                }
                /// Retains concrete construction proof until owner transfer.
                #object_vis fn into_local_box(#initial: Self) -> #nc::LocalBox<Self> {
                    // SAFETY: into_box constructs exactly Self's exclusive storage.
                    unsafe { #nc::LocalBox::__from_owned(Self::into_box(#initial)) }
                }
            },
        ),
    };

    let binding_imports = names.binding_imports();
    quote! {
        #input
        const _: () = {
        #binding_imports

        impl #impl_generics #object_type #where_clause {
            /// Number of queryable contracts, including canonical IUnknown.
            /// The expression depends on contract types, never implementation generics.
            pub const COM_INTERFACE_COUNT: usize = #interface_count;

            #constructors

            }

        // SAFETY: construction, typed storage recovery, tables, and threading
        // are generated together and select one concrete ownership policy.
        unsafe impl #impl_generics #nc::ComObject for #object_type #where_clause {
            type Threading = #declared_threading_model;
            type SharedGuard<'a> = #shared_guard;
            type ExclusiveGuard<'a> = #exclusive_guard;
            /// One trait-owned static table; no inherent forwarding member is emitted.
            const VTABLES: &'static [*const #nc::IUnknownVTable] = &{
                let #contracts = #nc::__nc_interface_chain::<{ #interface_count }>(
                    &#nc::IUnknown::INTERFACE_DESCRIPTOR, &[#(#declared_chains,)*],
                );
                let #projections = #nc::__nc_projection_table(
                    &#contracts,
                    // SAFETY: this allocation's canonical unknown table delegates
                    // lifecycle and queries to the same stored implementation.
                    unsafe { #nc::__ComProjection::<Self>::new(#nc::IUnknown::INTERFACE_ID, &#nc::lifetime::CANONICAL_UNKNOWN_VTABLE) },
                    &[#(#providers,)*],
                );
                #nc::__nc_projection_vtables(&#projections)
            };

            unsafe fn borrow_abi_ref<'a>(
                #object_pointer: *mut ::core::ffi::c_void,
            ) -> ::core::result::Result<#nc::__ComSharedLease<'a, Self, Self::SharedGuard<'a>>, #nc::NcStatus> {
                // SAFETY: caller retains the exact storage allocated by our constructor.
                unsafe { <#storage>::borrow_ref(#object_pointer) }
            }

            unsafe fn borrow_abi_mut<'a>(
                #object_pointer: *mut ::core::ffi::c_void,
            ) -> ::core::result::Result<#nc::__ComExclusiveLease<'a, Self, Self::ExclusiveGuard<'a>>, #nc::NcStatus> {
                // SAFETY: the selected storage enforces its one admission/borrow authority.
                unsafe { <#storage>::borrow_mut(#object_pointer) }
            }
        }
        };
    }
}
