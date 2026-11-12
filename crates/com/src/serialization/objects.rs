// Generated from com/src/serialization/objects.rs.in.
// Regenerate: NC_UPDATE_PERSISTENCE_BINDINGS=1 cargo test -p neocogi-idl serialization_bindings_match_idl

/// Owns one NCB document session; finishing freezes its bytes for output queries.
pub struct NcbSerializer {
    /// Concrete codec state, accessed through the ordinary object-wide call gate.
    encoder: binary::Encoder<'static>,
}
const _: () = {
    #[allow(unused_imports)]
    use ::core::convert::{
        identity as __nc_object_pointer, identity as __nc_initial_value,
        identity as __nc_contracts, identity as __nc_projections,
    };
    impl NcbSerializer
    where
        NcbSerializer: ::neocogi_com::__ComSupportsInterfaceThreading<
            ::neocogi_com::ThreadLocal,
        >,
    {
        /// Number of queryable contracts, including canonical IUnknown.
        /// The expression depends on contract types, never implementation generics.
        /// The count type remains primitive even when a generic is named usize.
        pub const COM_INTERFACE_COUNT: ::core::primitive::usize = ::neocogi_com::__nc_object_interface_count(
            &[
                &[&::neocogi_com::IUnknown::INTERFACE_DESCRIPTOR],
                &ISerializerAbi::INTERFACE_CHAIN,
                &ISerializationOutputAbi::INTERFACE_CHAIN,
            ],
        );
        /// Transfers one exclusive owner of a fully initialized value.
        pub fn into_box(
            __nc_initial_value: Self,
        ) -> ::neocogi_com::ComBox<
            ::neocogi_com::IUnknown,
            <NcbSerializer as ::neocogi_com::ComObject>::Threading,
        > {
            unsafe {
                ::neocogi_com::__ComBoxStorage::<
                    Self,
                >::into_exclusive(__nc_initial_value)
            }
        }
        /// Retains concrete construction proof until owner transfer.
        pub fn into_local_box(
            __nc_initial_value: Self,
        ) -> ::neocogi_com::LocalBox<Self> {
            unsafe {
                ::neocogi_com::LocalBox::__from_owned(Self::into_box(__nc_initial_value))
            }
        }
    }
    unsafe impl ::neocogi_com::ComObject for NcbSerializer
    where
        NcbSerializer: ::neocogi_com::__ComSupportsInterfaceThreading<
            ::neocogi_com::ThreadLocal,
        >,
    {
        type Threading = ::neocogi_com::ThreadLocal;
        type SharedGuard<'a> = ::neocogi_com::lifetime::BoxRead;
        type ExclusiveGuard<'a> = ::neocogi_com::lifetime::BoxWrite;
        /// One trait-owned static table; no inherent forwarding member is emitted.
        const VTABLES: &'static [*const ::neocogi_com::IUnknownVTable] = &{
            let __nc_contracts = ::neocogi_com::__nc_interface_chain::<
                {
                    ::neocogi_com::__nc_object_interface_count(
                        &[
                            &[&::neocogi_com::IUnknown::INTERFACE_DESCRIPTOR],
                            &ISerializerAbi::INTERFACE_CHAIN,
                            &ISerializationOutputAbi::INTERFACE_CHAIN,
                        ],
                    )
                },
            >(
                &::neocogi_com::IUnknown::INTERFACE_DESCRIPTOR,
                &[
                    &ISerializerAbi::INTERFACE_CHAIN,
                    &ISerializationOutputAbi::INTERFACE_CHAIN,
                ],
            );
            let __nc_projections = ::neocogi_com::__nc_projection_table(
                &__nc_contracts,
                unsafe {
                    ::neocogi_com::__ComProjection::<
                        Self,
                    >::new(
                        ::neocogi_com::IUnknown::INTERFACE_ID,
                        &::neocogi_com::lifetime::CANONICAL_UNKNOWN_VTABLE,
                    )
                },
                &[
                    &ISerializerVtblFor::<NcbSerializer>::PROJECTIONS,
                    &ISerializationOutputVtblFor::<NcbSerializer>::PROJECTIONS,
                ],
            );
            ::neocogi_com::__nc_projection_vtables(&__nc_projections)
        };
        unsafe fn borrow_abi_ref<'a>(
            __nc_object_pointer: *mut ::core::ffi::c_void,
        ) -> ::core::result::Result<
            ::neocogi_com::__ComSharedLease<'a, Self, Self::SharedGuard<'a>>,
            ::neocogi_com::NcStatus,
        > {
            unsafe {
                <::neocogi_com::__ComBoxStorage<Self>>::borrow_ref(__nc_object_pointer)
            }
        }
        unsafe fn borrow_abi_mut<'a>(
            __nc_object_pointer: *mut ::core::ffi::c_void,
        ) -> ::core::result::Result<
            ::neocogi_com::__ComExclusiveLease<'a, Self, Self::ExclusiveGuard<'a>>,
            ::neocogi_com::NcStatus,
        > {
            unsafe {
                <::neocogi_com::__ComBoxStorage<Self>>::borrow_mut(__nc_object_pointer)
            }
        }
    }
};
/// Owns immutable input and a mutable typed decoding cursor.
pub struct NcbDeserializer {
    /// Owned input remains valid through every call and is released with the object.
    decoder: binary::Decoder<'static>,
}
const _: () = {
    #[allow(unused_imports)]
    use ::core::convert::{
        identity as __nc_object_pointer, identity as __nc_initial_value,
        identity as __nc_contracts, identity as __nc_projections,
    };
    impl NcbDeserializer
    where
        NcbDeserializer: ::neocogi_com::__ComSupportsInterfaceThreading<
            <IDeserializerAbi as ::neocogi_com::ComInterface>::Threading,
        >,
    {
        /// Number of queryable contracts, including canonical IUnknown.
        /// The expression depends on contract types, never implementation generics.
        /// The count type remains primitive even when a generic is named usize.
        pub const COM_INTERFACE_COUNT: ::core::primitive::usize = ::neocogi_com::__nc_object_interface_count(
            &[
                &[&::neocogi_com::IUnknown::INTERFACE_DESCRIPTOR],
                &IDeserializerAbi::INTERFACE_CHAIN,
            ],
        );
        /// Transfers one exclusive owner of a fully initialized value.
        pub fn into_box(
            __nc_initial_value: Self,
        ) -> ::neocogi_com::ComBox<
            ::neocogi_com::IUnknown,
            <NcbDeserializer as ::neocogi_com::ComObject>::Threading,
        > {
            unsafe {
                ::neocogi_com::__ComBoxStorage::<
                    Self,
                >::into_exclusive(__nc_initial_value)
            }
        }
        /// Retains concrete construction proof until owner transfer.
        pub fn into_local_box(
            __nc_initial_value: Self,
        ) -> ::neocogi_com::LocalBox<Self> {
            unsafe {
                ::neocogi_com::LocalBox::__from_owned(Self::into_box(__nc_initial_value))
            }
        }
    }
    unsafe impl ::neocogi_com::ComObject for NcbDeserializer
    where
        NcbDeserializer: ::neocogi_com::__ComSupportsInterfaceThreading<
            <IDeserializerAbi as ::neocogi_com::ComInterface>::Threading,
        >,
    {
        type Threading = <IDeserializerAbi as ::neocogi_com::ComInterface>::Threading;
        type SharedGuard<'a> = ::neocogi_com::lifetime::BoxRead;
        type ExclusiveGuard<'a> = ::neocogi_com::lifetime::BoxWrite;
        /// One trait-owned static table; no inherent forwarding member is emitted.
        const VTABLES: &'static [*const ::neocogi_com::IUnknownVTable] = &{
            let __nc_contracts = ::neocogi_com::__nc_interface_chain::<
                {
                    ::neocogi_com::__nc_object_interface_count(
                        &[
                            &[&::neocogi_com::IUnknown::INTERFACE_DESCRIPTOR],
                            &IDeserializerAbi::INTERFACE_CHAIN,
                        ],
                    )
                },
            >(
                &::neocogi_com::IUnknown::INTERFACE_DESCRIPTOR,
                &[&IDeserializerAbi::INTERFACE_CHAIN],
            );
            let __nc_projections = ::neocogi_com::__nc_projection_table(
                &__nc_contracts,
                unsafe {
                    ::neocogi_com::__ComProjection::<
                        Self,
                    >::new(
                        ::neocogi_com::IUnknown::INTERFACE_ID,
                        &::neocogi_com::lifetime::CANONICAL_UNKNOWN_VTABLE,
                    )
                },
                &[&IDeserializerVtblFor::<NcbDeserializer>::PROJECTIONS],
            );
            ::neocogi_com::__nc_projection_vtables(&__nc_projections)
        };
        unsafe fn borrow_abi_ref<'a>(
            __nc_object_pointer: *mut ::core::ffi::c_void,
        ) -> ::core::result::Result<
            ::neocogi_com::__ComSharedLease<'a, Self, Self::SharedGuard<'a>>,
            ::neocogi_com::NcStatus,
        > {
            unsafe {
                <::neocogi_com::__ComBoxStorage<Self>>::borrow_ref(__nc_object_pointer)
            }
        }
        unsafe fn borrow_abi_mut<'a>(
            __nc_object_pointer: *mut ::core::ffi::c_void,
        ) -> ::core::result::Result<
            ::neocogi_com::__ComExclusiveLease<'a, Self, Self::ExclusiveGuard<'a>>,
            ::neocogi_com::NcStatus,
        > {
            unsafe {
                <::neocogi_com::__ComBoxStorage<Self>>::borrow_mut(__nc_object_pointer)
            }
        }
    }
};
/// Stateless shared factory whose document sessions each have exclusive ownership.
pub struct NcbFormat;
const _: () = {
    #[allow(unused_imports)]
    use ::core::convert::{
        identity as __nc_object_pointer, identity as __nc_initial_value,
        identity as __nc_contracts, identity as __nc_projections,
    };
    impl NcbFormat
    where
        NcbFormat: ::neocogi_com::__ComSupportsInterfaceThreading<
            <ISerializationFormatAbi as ::neocogi_com::ComInterface>::Threading,
        >,
    {
        /// Number of queryable contracts, including canonical IUnknown.
        /// The expression depends on contract types, never implementation generics.
        /// The count type remains primitive even when a generic is named usize.
        pub const COM_INTERFACE_COUNT: ::core::primitive::usize = ::neocogi_com::__nc_object_interface_count(
            &[
                &[&::neocogi_com::IUnknown::INTERFACE_DESCRIPTOR],
                &ISerializationFormatAbi::INTERFACE_CHAIN,
            ],
        );
        /// Owns this exact implementation with checked local access.
        pub fn into_local(__nc_initial_value: Self) -> ::neocogi_com::LocalCom<Self> {
            unsafe {
                ::neocogi_com::LocalCom::__from_owned(
                    Self::from_value(__nc_initial_value),
                )
            }
        }
        /// Wraps a complete Rust value in one shared COM allocation.
        pub fn from_value(
            __nc_initial_value: Self,
        ) -> ::neocogi_com::ComPtr<
            ::neocogi_com::IUnknown,
            <NcbFormat as ::neocogi_com::ComObject>::Threading,
        > {
            unsafe {
                ::neocogi_com::__ComSharedStorage::<
                    Self,
                >::into_shared(__nc_initial_value)
            }
        }
    }
    unsafe impl ::neocogi_com::ComObject for NcbFormat
    where
        NcbFormat: ::neocogi_com::__ComSupportsInterfaceThreading<
            <ISerializationFormatAbi as ::neocogi_com::ComInterface>::Threading,
        >,
    {
        type Threading = <ISerializationFormatAbi as ::neocogi_com::ComInterface>::Threading;
        type SharedGuard<'a> = ::neocogi_com::__ComSharedCall<'a>;
        type ExclusiveGuard<'a> = ::neocogi_com::__ComExclusiveCall<'a>;
        /// One trait-owned static table; no inherent forwarding member is emitted.
        const VTABLES: &'static [*const ::neocogi_com::IUnknownVTable] = &{
            let __nc_contracts = ::neocogi_com::__nc_interface_chain::<
                {
                    ::neocogi_com::__nc_object_interface_count(
                        &[
                            &[&::neocogi_com::IUnknown::INTERFACE_DESCRIPTOR],
                            &ISerializationFormatAbi::INTERFACE_CHAIN,
                        ],
                    )
                },
            >(
                &::neocogi_com::IUnknown::INTERFACE_DESCRIPTOR,
                &[&ISerializationFormatAbi::INTERFACE_CHAIN],
            );
            let __nc_projections = ::neocogi_com::__nc_projection_table(
                &__nc_contracts,
                unsafe {
                    ::neocogi_com::__ComProjection::<
                        Self,
                    >::new(
                        ::neocogi_com::IUnknown::INTERFACE_ID,
                        &::neocogi_com::lifetime::CANONICAL_UNKNOWN_VTABLE,
                    )
                },
                &[&ISerializationFormatVtblFor::<NcbFormat>::PROJECTIONS],
            );
            ::neocogi_com::__nc_projection_vtables(&__nc_projections)
        };
        unsafe fn borrow_abi_ref<'a>(
            __nc_object_pointer: *mut ::core::ffi::c_void,
        ) -> ::core::result::Result<
            ::neocogi_com::__ComSharedLease<'a, Self, Self::SharedGuard<'a>>,
            ::neocogi_com::NcStatus,
        > {
            unsafe {
                <::neocogi_com::__ComSharedStorage<
                    Self,
                >>::borrow_ref(__nc_object_pointer)
            }
        }
        unsafe fn borrow_abi_mut<'a>(
            __nc_object_pointer: *mut ::core::ffi::c_void,
        ) -> ::core::result::Result<
            ::neocogi_com::__ComExclusiveLease<'a, Self, Self::ExclusiveGuard<'a>>,
            ::neocogi_com::NcStatus,
        > {
            unsafe {
                <::neocogi_com::__ComSharedStorage<
                    Self,
                >>::borrow_mut(__nc_object_pointer)
            }
        }
    }
};
