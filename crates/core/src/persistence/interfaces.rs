// Generated from core/idl/neocogi/com/persistence/objects.ncidl.
// Regenerate: NC_UPDATE_PERSISTENCE_BINDINGS=1 cargo test -p neocogi-idl persistence_bindings_match_idl

/// Stable logical identity of this generated contract module.
pub const MODULE_ID: ::neocogi_com::NcGuid = ::neocogi_com::NcGuid::from_fields(
    3545662286u32,
    20204u16,
    22946u16,
    [140u8, 82u8, 62u8, 53u8, 229u8, 28u8, 242u8, 16u8],
);
/// Transitive canonical ABI fingerprint of this generated module.
pub const ABI_FINGERPRINT: [u8; 32] = [
    33u8, 81u8, 246u8, 243u8, 125u8, 56u8, 133u8, 202u8, 30u8, 147u8, 202u8, 41u8, 37u8,
    62u8, 129u8, 194u8, 191u8, 148u8, 242u8, 247u8, 144u8, 113u8, 167u8, 228u8, 197u8,
    22u8, 157u8, 140u8, 99u8, 107u8, 123u8, 101u8,
];
/// Exact dotted module name derived from the normalized IDL path.
pub const MODULE_NAME: &str = "neocogi.com.persistence";
/// An ordered, owning list used only while constructing a persistent parent.
/// IDL source: `neocogi/com/persistence/objects.ncidl:3:11` (module `neocogi.com.persistence`).
pub trait IObjectChildren {
    /// Returns the number of fully initialized child objects.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:5:8` (module `neocogi.com.persistence`).
    fn count(&self) -> ::core::result::Result<u32, ::neocogi_com::NcStatus>;
    /// Returns an owned reference; indices must be less than count.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:7:8` (module `neocogi.com.persistence`).
    fn get(
        &self,
        index: u32,
    ) -> ::core::result::Result<
        ::neocogi_com::ComPtr<::neocogi_com::IUnknown>,
        ::neocogi_com::NcStatus,
    >;
}
///Interface marker for [`IObjectChildren`]; handles carry its object/table pair.
pub enum IObjectChildrenAbi {}
#[repr(C)]
///C-compatible vtable for [`IObjectChildren`].
pub struct IObjectChildrenVTable {
    /// Common identity, query and reference-count slots.
    pub unknown: ::neocogi_com::IUnknownVTable,
    /// Returns the number of fully initialized child objects.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:5:8` (module `neocogi.com.persistence`).
    ///Raw COM ABI vtable slot for [`IObjectChildren::count`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< u32 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub count: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Returns an owned reference; indices must be less than count.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:7:8` (module `neocogi.com.persistence`).
    ///Raw COM ABI vtable slot for [`IObjectChildren::get`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `index` uses ABI storage `< u32 as :: neocogi_com :: ComValue > :: Wire` for `u32`. Passed by value.
    ///
    ///On success, `out` receives `:: neocogi_com :: ComRaw < :: neocogi_com :: IUnknown >` for `:: neocogi_com :: ComPtr < :: neocogi_com :: IUnknown >`. The caller receives one shared owner and releases it when finished.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub get: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0: <u32 as ::neocogi_com::ComValue>::Wire,
        __nc_out: *mut ::neocogi_com::ComRaw<::neocogi_com::IUnknown>,
    ) -> ::neocogi_com::NcStatus,
}
unsafe impl ::neocogi_com::ComInterface for IObjectChildrenAbi {
    const IID: ::neocogi_com::NcGuid = ::neocogi_com::NcGuid {
        data1: 3044267551u32,
        data2: 19810u16,
        data3: 22379u16,
        data4: [135u8, 249u8, 36u8, 136u8, 103u8, 45u8, 179u8, 255u8],
    };
    const INTERFACE_ID: ::neocogi_com::NcInterfaceId = ::neocogi_com::NcInterfaceId::new(
        ::neocogi_com::NcGuid {
            data1: 3044267551u32,
            data2: 19810u16,
            data3: 22379u16,
            data4: [135u8, 249u8, 36u8, 136u8, 103u8, 45u8, 179u8, 255u8],
        },
        IObjectChildrenAbi::ABI_SIGNATURE,
    );
    type VTable = IObjectChildrenVTable;
    type Threading = ::neocogi_com::ThreadLocal;
}
#[repr(transparent)]
///Borrowed static method view for [`IObjectChildren`].
pub struct IObjectChildrenClient<
    __NcThreading: ::neocogi_com::ThreadingModel = ::neocogi_com::ThreadLocal,
> {
    /// Certified transparent storage supplied only by a live COM pointer borrow.
    __nc_pointer: ::neocogi_com::NcClientPointer<IObjectChildrenAbi, __NcThreading>,
}
unsafe impl ::neocogi_com::ComClientInterface for IObjectChildrenAbi {
    type Client<__NcThreading: ::neocogi_com::ThreadingModel> = IObjectChildrenClient<
        __NcThreading,
    >;
}
const _: () = {
    #[allow(unused_imports)]
    use ::core::convert::{
        identity as __nc_dispatch_object, identity as __nc_dispatch_value,
        identity as __nc_arg_0, identity as __nc_arg_0_owner_slot,
        identity as __nc_arg_0_offset, identity as __nc_arg_0_len,
        identity as __nc_arg_0_ptr, identity as __nc_arg_0_values,
        identity as __nc_arg_0_written, identity as __nc_this, identity as __nc_status,
        identity as __nc_value, identity as __nc_raw, identity as __nc_result,
        identity as __nc_values, identity as __nc_writer, identity as __nc_out,
        identity as __nc_identity, identity as __nc_this_0, identity as __nc_status_0,
        identity as __nc_result_0, identity as __nc_prepared, identity as __nc_out_0,
        identity as __nc_this_1, identity as __nc_status_1, identity as __nc_result_1,
        identity as __nc_prepared_0, identity as __nc_out_1,
    };
    impl<
        __NcThreading: ::neocogi_com::ThreadingModel,
    > IObjectChildrenClient<__NcThreading> {
        /// Returns the number of fully initialized child objects.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:5:8` (module `neocogi.com.persistence`).
        ///
        ///Calls [`IObjectChildren::count`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `u32` from the ABI out slot.
        pub fn count(&self) -> ::core::result::Result<u32, ::neocogi_com::NcStatus> {
            let mut __nc_out_0 = ::core::mem::MaybeUninit::<
                <u32 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_0 = self.__nc_pointer.as_raw();
            let __nc_status_0 = unsafe {
                ((*__nc_this_0.vtable)
                    .count)(__nc_this_0.object, __nc_out_0.as_mut_ptr())
            };
            if __nc_status_0.is_err() {
                return ::core::result::Result::Err(__nc_status_0);
            }
            let __nc_result_0 = (unsafe {
                ::neocogi_com::decode_inline::<u32>(__nc_out_0.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_0)
        }
        /// Returns an owned reference; indices must be less than count.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:7:8` (module `neocogi.com.persistence`).
        ///
        ///Calls [`IObjectChildren::get`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `index` is passed by value as `u32`.
        ///
        ///On success, returns `:: neocogi_com :: ComPtr < :: neocogi_com :: IUnknown >`. The caller receives one shared owner and releases it when finished.
        pub fn get(
            &self,
            __nc_arg_0: u32,
        ) -> ::core::result::Result<
            ::neocogi_com::ComPtr<::neocogi_com::IUnknown>,
            ::neocogi_com::NcStatus,
        > {
            let __nc_arg_0 = ::neocogi_com::encode_inline(&__nc_arg_0)?;
            let mut __nc_out_1 = ::core::mem::MaybeUninit::<
                ::neocogi_com::ComRaw<::neocogi_com::IUnknown>,
            >::uninit();
            let __nc_this_1 = self.__nc_pointer.as_raw();
            let __nc_status_1 = unsafe {
                ((*__nc_this_1.vtable)
                    .get)(__nc_this_1.object, __nc_arg_0, __nc_out_1.as_mut_ptr())
            };
            if __nc_status_1.is_err() {
                return ::core::result::Result::Err(__nc_status_1);
            }
            let __nc_result_1 = (unsafe {
                <::neocogi_com::ComPtr<
                    ::neocogi_com::IUnknown,
                >>::__nc_from_raw_owned_checked(__nc_out_1.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_1)
        }
    }
};
impl IObjectChildrenAbi {
    ///Canonical validation form of this interface's typed ABI contract.
    pub const ABI_SIGNATURE: ::neocogi_com::NcAbiSignature = ::neocogi_com::NcAbiSignature::from_static(
        b"\x19\x90\x98fW\x9D\x12\x9DI\xD6,{\xDA\xD7[\x8Bt6\xD1*\xF6\xED\x04r\xD2*o'\xDF|s\x84",
    );
    ///Static descriptors for this interface's ABI methods.
    pub const METHOD_DESCRIPTORS: [::neocogi_com::NcMethodDescriptor; 2usize] = [
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("count"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<u32>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("get"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("index"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<u32>(),
                    ),
                },
            ] as &'static [::neocogi_com::NcMethodParamDescriptor])
                .as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(::neocogi_com::NcAbiPassing::ComPtrOut {
                    mode: <::neocogi_com::ThreadLocal as ::neocogi_com::ThreadingModel>::ABI_THREADING_MODE,
                }),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_interface(
                    <::neocogi_com::IUnknown as ::neocogi_com::ComInterface>::INTERFACE_ID,
                ),
            },
        },
    ];
    ///Static ABI descriptor for this interface.
    pub const INTERFACE_DESCRIPTOR: ::neocogi_com::NcInterfaceDescriptor = ::neocogi_com::NcInterfaceDescriptor {
        interface_id: <Self as ::neocogi_com::ComInterface>::IID,
        qualified_name: ::neocogi_com::NcStaticUtf8::from_static(
            "neocogi.com.persistence.IObjectChildren",
        ),
        abi_signature: Self::ABI_SIGNATURE,
        base: ::neocogi_com::NcInterfaceId::NONE,
        method_count: 2u32,
        methods: Self::METHOD_DESCRIPTORS.as_ptr(),
    };
    /// Distinct projections in derived-first, declared-parent order.
    /// Shared ancestors occur once; repeated identities must have the same ABI.
    pub const INTERFACE_CHAIN: [&'static ::neocogi_com::NcInterfaceDescriptor; ::neocogi_com::__nc_object_interface_count(
        &[&[&Self::INTERFACE_DESCRIPTOR]],
    )] = ::neocogi_com::__nc_interface_chain(&Self::INTERFACE_DESCRIPTOR, &[]);
}
/// Optional snapshot capability for an owned construction tree.
/// IDL source: `neocogi/com/persistence/objects.ncidl:12:11` (module `neocogi.com.persistence`).
pub trait IPersistable {
    /// Identifies the registered construction recipe, not this object instance.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:14:8` (module `neocogi.com.persistence`).
    fn type_id(
        &self,
    ) -> ::core::result::Result<::neocogi_com::NcGuid, ::neocogi_com::NcStatus>;
    /// Identifies the persistent initialization schema, independently of COM ABI.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:16:8` (module `neocogi.com.persistence`).
    fn schema_version(&self) -> ::core::result::Result<u32, ::neocogi_com::NcStatus>;
    /// Exports initialization data only: no receivers, runtime pointers or caches.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:18:8` (module `neocogi.com.persistence`).
    fn save_state(
        &self,
    ) -> ::core::result::Result<
        ::neocogi_com::ComPtr<::neocogi_com::INcBytesAbi>,
        ::neocogi_com::NcStatus,
    >;
    /// Returns the number of owned persistent children; parent links are excluded.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:20:8` (module `neocogi.com.persistence`).
    fn child_count(&self) -> ::core::result::Result<u32, ::neocogi_com::NcStatus>;
    /// Returns one child in the order described by the parent's initialization.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:22:8` (module `neocogi.com.persistence`).
    fn child(
        &self,
        index: u32,
    ) -> ::core::result::Result<
        ::neocogi_com::ComPtr<::neocogi_com::IUnknown>,
        ::neocogi_com::NcStatus,
    >;
}
///Interface marker for [`IPersistable`]; handles carry its object/table pair.
pub enum IPersistableAbi {}
#[repr(C)]
///C-compatible vtable for [`IPersistable`].
pub struct IPersistableVTable {
    /// Common identity, query and reference-count slots.
    pub unknown: ::neocogi_com::IUnknownVTable,
    /// Identifies the registered construction recipe, not this object instance.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:14:8` (module `neocogi.com.persistence`).
    ///Raw COM ABI vtable slot for [`IPersistable::type_id`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: neocogi_com :: NcGuid as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub type_id: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::neocogi_com::NcGuid as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Identifies the persistent initialization schema, independently of COM ABI.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:16:8` (module `neocogi.com.persistence`).
    ///Raw COM ABI vtable slot for [`IPersistable::schema_version`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< u32 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub schema_version: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Exports initialization data only: no receivers, runtime pointers or caches.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:18:8` (module `neocogi.com.persistence`).
    ///Raw COM ABI vtable slot for [`IPersistable::save_state`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `:: neocogi_com :: ComRaw < :: neocogi_com :: INcBytesAbi >` for `:: neocogi_com :: ComPtr < :: neocogi_com :: INcBytesAbi >`. The caller receives one shared owner and releases it when finished.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub save_state: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut ::neocogi_com::ComRaw<::neocogi_com::INcBytesAbi>,
    ) -> ::neocogi_com::NcStatus,
    /// Returns the number of owned persistent children; parent links are excluded.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:20:8` (module `neocogi.com.persistence`).
    ///Raw COM ABI vtable slot for [`IPersistable::child_count`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< u32 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub child_count: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Returns one child in the order described by the parent's initialization.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:22:8` (module `neocogi.com.persistence`).
    ///Raw COM ABI vtable slot for [`IPersistable::child`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `index` uses ABI storage `< u32 as :: neocogi_com :: ComValue > :: Wire` for `u32`. Passed by value.
    ///
    ///On success, `out` receives `:: neocogi_com :: ComRaw < :: neocogi_com :: IUnknown >` for `:: neocogi_com :: ComPtr < :: neocogi_com :: IUnknown >`. The caller receives one shared owner and releases it when finished.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub child: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0: <u32 as ::neocogi_com::ComValue>::Wire,
        __nc_out: *mut ::neocogi_com::ComRaw<::neocogi_com::IUnknown>,
    ) -> ::neocogi_com::NcStatus,
}
unsafe impl ::neocogi_com::ComInterface for IPersistableAbi {
    const IID: ::neocogi_com::NcGuid = ::neocogi_com::NcGuid {
        data1: 3211114322u32,
        data2: 3431u16,
        data3: 21153u16,
        data4: [164u8, 211u8, 23u8, 32u8, 218u8, 170u8, 154u8, 0u8],
    };
    const INTERFACE_ID: ::neocogi_com::NcInterfaceId = ::neocogi_com::NcInterfaceId::new(
        ::neocogi_com::NcGuid {
            data1: 3211114322u32,
            data2: 3431u16,
            data3: 21153u16,
            data4: [164u8, 211u8, 23u8, 32u8, 218u8, 170u8, 154u8, 0u8],
        },
        IPersistableAbi::ABI_SIGNATURE,
    );
    type VTable = IPersistableVTable;
    type Threading = ::neocogi_com::ThreadLocal;
}
#[repr(transparent)]
///Borrowed static method view for [`IPersistable`].
pub struct IPersistableClient<
    __NcThreading: ::neocogi_com::ThreadingModel = ::neocogi_com::ThreadLocal,
> {
    /// Certified transparent storage supplied only by a live COM pointer borrow.
    __nc_pointer: ::neocogi_com::NcClientPointer<IPersistableAbi, __NcThreading>,
}
unsafe impl ::neocogi_com::ComClientInterface for IPersistableAbi {
    type Client<__NcThreading: ::neocogi_com::ThreadingModel> = IPersistableClient<
        __NcThreading,
    >;
}
const _: () = {
    #[allow(unused_imports)]
    use ::core::convert::{
        identity as __nc_dispatch_object, identity as __nc_dispatch_value,
        identity as __nc_arg_0, identity as __nc_arg_0_owner_slot,
        identity as __nc_arg_0_offset, identity as __nc_arg_0_len,
        identity as __nc_arg_0_ptr, identity as __nc_arg_0_values,
        identity as __nc_arg_0_written, identity as __nc_this, identity as __nc_status,
        identity as __nc_value, identity as __nc_raw, identity as __nc_result,
        identity as __nc_values, identity as __nc_writer, identity as __nc_out,
        identity as __nc_identity, identity as __nc_this_0, identity as __nc_status_0,
        identity as __nc_result_0, identity as __nc_prepared, identity as __nc_out_0,
        identity as __nc_this_1, identity as __nc_status_1, identity as __nc_result_1,
        identity as __nc_prepared_0, identity as __nc_out_1, identity as __nc_this_2,
        identity as __nc_status_2, identity as __nc_result_2,
        identity as __nc_prepared_1, identity as __nc_out_2, identity as __nc_this_3,
        identity as __nc_status_3, identity as __nc_result_3,
        identity as __nc_prepared_2, identity as __nc_out_3, identity as __nc_this_4,
        identity as __nc_status_4, identity as __nc_result_4,
        identity as __nc_prepared_3, identity as __nc_out_4,
    };
    impl<
        __NcThreading: ::neocogi_com::ThreadingModel,
    > IPersistableClient<__NcThreading> {
        /// Identifies the registered construction recipe, not this object instance.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:14:8` (module `neocogi.com.persistence`).
        ///
        ///Calls [`IPersistable::type_id`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: neocogi_com :: NcGuid` from the ABI out slot.
        pub fn type_id(
            &self,
        ) -> ::core::result::Result<::neocogi_com::NcGuid, ::neocogi_com::NcStatus> {
            let mut __nc_out_0 = ::core::mem::MaybeUninit::<
                <::neocogi_com::NcGuid as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_0 = self.__nc_pointer.as_raw();
            let __nc_status_0 = unsafe {
                ((*__nc_this_0.vtable)
                    .type_id)(__nc_this_0.object, __nc_out_0.as_mut_ptr())
            };
            if __nc_status_0.is_err() {
                return ::core::result::Result::Err(__nc_status_0);
            }
            let __nc_result_0 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::neocogi_com::NcGuid,
                >(__nc_out_0.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_0)
        }
        /// Identifies the persistent initialization schema, independently of COM ABI.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:16:8` (module `neocogi.com.persistence`).
        ///
        ///Calls [`IPersistable::schema_version`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `u32` from the ABI out slot.
        pub fn schema_version(
            &self,
        ) -> ::core::result::Result<u32, ::neocogi_com::NcStatus> {
            let mut __nc_out_1 = ::core::mem::MaybeUninit::<
                <u32 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_1 = self.__nc_pointer.as_raw();
            let __nc_status_1 = unsafe {
                ((*__nc_this_1.vtable)
                    .schema_version)(__nc_this_1.object, __nc_out_1.as_mut_ptr())
            };
            if __nc_status_1.is_err() {
                return ::core::result::Result::Err(__nc_status_1);
            }
            let __nc_result_1 = (unsafe {
                ::neocogi_com::decode_inline::<u32>(__nc_out_1.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_1)
        }
        /// Exports initialization data only: no receivers, runtime pointers or caches.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:18:8` (module `neocogi.com.persistence`).
        ///
        ///Calls [`IPersistable::save_state`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: neocogi_com :: ComPtr < :: neocogi_com :: INcBytesAbi >`. The caller receives one shared owner and releases it when finished.
        pub fn save_state(
            &self,
        ) -> ::core::result::Result<
            ::neocogi_com::ComPtr<::neocogi_com::INcBytesAbi>,
            ::neocogi_com::NcStatus,
        > {
            let mut __nc_out_2 = ::core::mem::MaybeUninit::<
                ::neocogi_com::ComRaw<::neocogi_com::INcBytesAbi>,
            >::uninit();
            let __nc_this_2 = self.__nc_pointer.as_raw();
            let __nc_status_2 = unsafe {
                ((*__nc_this_2.vtable)
                    .save_state)(__nc_this_2.object, __nc_out_2.as_mut_ptr())
            };
            if __nc_status_2.is_err() {
                return ::core::result::Result::Err(__nc_status_2);
            }
            let __nc_result_2 = (unsafe {
                <::neocogi_com::ComPtr<
                    ::neocogi_com::INcBytesAbi,
                >>::__nc_from_raw_owned_checked(__nc_out_2.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_2)
        }
        /// Returns the number of owned persistent children; parent links are excluded.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:20:8` (module `neocogi.com.persistence`).
        ///
        ///Calls [`IPersistable::child_count`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `u32` from the ABI out slot.
        pub fn child_count(
            &self,
        ) -> ::core::result::Result<u32, ::neocogi_com::NcStatus> {
            let mut __nc_out_3 = ::core::mem::MaybeUninit::<
                <u32 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_3 = self.__nc_pointer.as_raw();
            let __nc_status_3 = unsafe {
                ((*__nc_this_3.vtable)
                    .child_count)(__nc_this_3.object, __nc_out_3.as_mut_ptr())
            };
            if __nc_status_3.is_err() {
                return ::core::result::Result::Err(__nc_status_3);
            }
            let __nc_result_3 = (unsafe {
                ::neocogi_com::decode_inline::<u32>(__nc_out_3.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_3)
        }
        /// Returns one child in the order described by the parent's initialization.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:22:8` (module `neocogi.com.persistence`).
        ///
        ///Calls [`IPersistable::child`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `index` is passed by value as `u32`.
        ///
        ///On success, returns `:: neocogi_com :: ComPtr < :: neocogi_com :: IUnknown >`. The caller receives one shared owner and releases it when finished.
        pub fn child(
            &self,
            __nc_arg_0: u32,
        ) -> ::core::result::Result<
            ::neocogi_com::ComPtr<::neocogi_com::IUnknown>,
            ::neocogi_com::NcStatus,
        > {
            let __nc_arg_0 = ::neocogi_com::encode_inline(&__nc_arg_0)?;
            let mut __nc_out_4 = ::core::mem::MaybeUninit::<
                ::neocogi_com::ComRaw<::neocogi_com::IUnknown>,
            >::uninit();
            let __nc_this_4 = self.__nc_pointer.as_raw();
            let __nc_status_4 = unsafe {
                ((*__nc_this_4.vtable)
                    .child)(__nc_this_4.object, __nc_arg_0, __nc_out_4.as_mut_ptr())
            };
            if __nc_status_4.is_err() {
                return ::core::result::Result::Err(__nc_status_4);
            }
            let __nc_result_4 = (unsafe {
                <::neocogi_com::ComPtr<
                    ::neocogi_com::IUnknown,
                >>::__nc_from_raw_owned_checked(__nc_out_4.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_4)
        }
    }
};
impl IPersistableAbi {
    ///Canonical validation form of this interface's typed ABI contract.
    pub const ABI_SIGNATURE: ::neocogi_com::NcAbiSignature = ::neocogi_com::NcAbiSignature::from_static(
        b"\xDD-g\xB5n\xEF\xB3\xCB)P\x02\xFA\xCE\x1C\x86\xAE_9\xD4}{P\x03\xF7~\xDF\xCC#\xE6<\x07\xC1",
    );
    ///Static descriptors for this interface's ABI methods.
    pub const METHOD_DESCRIPTORS: [::neocogi_com::NcMethodDescriptor; 5usize] = [
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("type_id"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::neocogi_com::NcGuid>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("schema_version"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<u32>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("save_state"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(::neocogi_com::NcAbiPassing::ComPtrOut {
                    mode: <::neocogi_com::ThreadLocal as ::neocogi_com::ThreadingModel>::ABI_THREADING_MODE,
                }),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_interface(
                    <::neocogi_com::INcBytesAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("child_count"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<u32>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("child"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("index"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<u32>(),
                    ),
                },
            ] as &'static [::neocogi_com::NcMethodParamDescriptor])
                .as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(::neocogi_com::NcAbiPassing::ComPtrOut {
                    mode: <::neocogi_com::ThreadLocal as ::neocogi_com::ThreadingModel>::ABI_THREADING_MODE,
                }),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_interface(
                    <::neocogi_com::IUnknown as ::neocogi_com::ComInterface>::INTERFACE_ID,
                ),
            },
        },
    ];
    ///Static ABI descriptor for this interface.
    pub const INTERFACE_DESCRIPTOR: ::neocogi_com::NcInterfaceDescriptor = ::neocogi_com::NcInterfaceDescriptor {
        interface_id: <Self as ::neocogi_com::ComInterface>::IID,
        qualified_name: ::neocogi_com::NcStaticUtf8::from_static(
            "neocogi.com.persistence.IPersistable",
        ),
        abi_signature: Self::ABI_SIGNATURE,
        base: ::neocogi_com::NcInterfaceId::NONE,
        method_count: 5u32,
        methods: Self::METHOD_DESCRIPTORS.as_ptr(),
    };
    /// Distinct projections in derived-first, declared-parent order.
    /// Shared ancestors occur once; repeated identities must have the same ABI.
    pub const INTERFACE_CHAIN: [&'static ::neocogi_com::NcInterfaceDescriptor; ::neocogi_com::__nc_object_interface_count(
        &[&[&Self::INTERFACE_DESCRIPTOR]],
    )] = ::neocogi_com::__nc_interface_chain(&Self::INTERFACE_DESCRIPTOR, &[]);
}
/// A provider bound to the runtime services required by its construction recipe.
/// IDL source: `neocogi/com/persistence/objects.ncidl:27:11` (module `neocogi.com.persistence`).
pub trait IPersistenceFactory {
    /// Rejects unsupported revisions before any child object is constructed.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:29:8` (module `neocogi.com.persistence`).
    fn validate(
        &self,
        version: u32,
        state: ::neocogi_com::NcCopyFromBufferOffset<'_, u8>,
        child_count: u32,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Returns a fully initialized object; every child is already constructed.
    /// Failure must release resources acquired by this call and publish no root.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:32:8` (module `neocogi.com.persistence`).
    fn restore(
        &self,
        version: u32,
        state: ::neocogi_com::NcCopyFromBufferOffset<'_, u8>,
        children: ::neocogi_com::ComRef<'_, self::IObjectChildrenAbi>,
    ) -> ::core::result::Result<
        ::neocogi_com::ComPtr<::neocogi_com::IUnknown>,
        ::neocogi_com::NcStatus,
    >;
}
///Interface marker for [`IPersistenceFactory`]; handles carry its object/table pair.
pub enum IPersistenceFactoryAbi {}
#[repr(C)]
///C-compatible vtable for [`IPersistenceFactory`].
pub struct IPersistenceFactoryVTable {
    /// Common identity, query and reference-count slots.
    pub unknown: ::neocogi_com::IUnknownVTable,
    /// Rejects unsupported revisions before any child object is constructed.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:29:8` (module `neocogi.com.persistence`).
    ///Raw COM ABI vtable slot for [`IPersistenceFactory::validate`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `version` uses ABI storage `< u32 as :: neocogi_com :: ComValue > :: Wire` for `u32`. Passed by value.
    ///- `state` uses ABI storage `* const < u8 as :: neocogi_com :: ComValue > :: Wire` for `:: neocogi_com :: NcCopyFromBufferOffset < '_ , u8 >`. The readable buffer pointer is accompanied by `offset: u64` and `len: u64`.
    ///- `child_count` uses ABI storage `< u32 as :: neocogi_com :: ComValue > :: Wire` for `u32`. Passed by value.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub validate: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0: <u32 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_1_offset: u64,
        __nc_arg_1_len: u64,
        __nc_arg_1_ptr: *const <u8 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_2: <u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Returns a fully initialized object; every child is already constructed.
    /// Failure must release resources acquired by this call and publish no root.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:32:8` (module `neocogi.com.persistence`).
    ///Raw COM ABI vtable slot for [`IPersistenceFactory::restore`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `version` uses ABI storage `< u32 as :: neocogi_com :: ComValue > :: Wire` for `u32`. Passed by value.
    ///- `state` uses ABI storage `* const < u8 as :: neocogi_com :: ComValue > :: Wire` for `:: neocogi_com :: NcCopyFromBufferOffset < '_ , u8 >`. The readable buffer pointer is accompanied by `offset: u64` and `len: u64`.
    ///- `children` uses ABI storage `:: neocogi_com :: ComRaw < self :: IObjectChildrenAbi >` for `:: neocogi_com :: ComRef < '_ , self :: IObjectChildrenAbi >`. This is a borrowed object/table pair. Both fields must be non-null. The callee must call `AddRef` before retaining the interface after the call returns.
    ///
    ///On success, `out` receives `:: neocogi_com :: ComRaw < :: neocogi_com :: IUnknown >` for `:: neocogi_com :: ComPtr < :: neocogi_com :: IUnknown >`. The caller receives one shared owner and releases it when finished.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub restore: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_0: <u32 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_1_0_offset: u64,
        __nc_arg_1_0_len: u64,
        __nc_arg_1_0_ptr: *const <u8 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_2_0: ::neocogi_com::ComRaw<self::IObjectChildrenAbi>,
        __nc_out: *mut ::neocogi_com::ComRaw<::neocogi_com::IUnknown>,
    ) -> ::neocogi_com::NcStatus,
}
unsafe impl ::neocogi_com::ComInterface for IPersistenceFactoryAbi {
    const IID: ::neocogi_com::NcGuid = ::neocogi_com::NcGuid {
        data1: 1927987790u32,
        data2: 295u16,
        data3: 23997u16,
        data4: [164u8, 199u8, 47u8, 172u8, 222u8, 225u8, 180u8, 131u8],
    };
    const INTERFACE_ID: ::neocogi_com::NcInterfaceId = ::neocogi_com::NcInterfaceId::new(
        ::neocogi_com::NcGuid {
            data1: 1927987790u32,
            data2: 295u16,
            data3: 23997u16,
            data4: [164u8, 199u8, 47u8, 172u8, 222u8, 225u8, 180u8, 131u8],
        },
        IPersistenceFactoryAbi::ABI_SIGNATURE,
    );
    type VTable = IPersistenceFactoryVTable;
    type Threading = ::neocogi_com::ThreadLocal;
}
#[repr(transparent)]
///Borrowed static method view for [`IPersistenceFactory`].
pub struct IPersistenceFactoryClient<
    __NcThreading: ::neocogi_com::ThreadingModel = ::neocogi_com::ThreadLocal,
> {
    /// Certified transparent storage supplied only by a live COM pointer borrow.
    __nc_pointer: ::neocogi_com::NcClientPointer<IPersistenceFactoryAbi, __NcThreading>,
}
unsafe impl ::neocogi_com::ComClientInterface for IPersistenceFactoryAbi {
    type Client<__NcThreading: ::neocogi_com::ThreadingModel> = IPersistenceFactoryClient<
        __NcThreading,
    >;
}
const _: () = {
    #[allow(unused_imports)]
    use ::core::convert::{
        identity as __nc_dispatch_object, identity as __nc_dispatch_value,
        identity as __nc_arg_0, identity as __nc_arg_0_owner_slot,
        identity as __nc_arg_0_offset, identity as __nc_arg_0_len,
        identity as __nc_arg_0_ptr, identity as __nc_arg_0_values,
        identity as __nc_arg_0_written, identity as __nc_arg_1,
        identity as __nc_arg_1_owner_slot, identity as __nc_arg_1_offset,
        identity as __nc_arg_1_len, identity as __nc_arg_1_ptr,
        identity as __nc_arg_1_values, identity as __nc_arg_1_written,
        identity as __nc_arg_2, identity as __nc_arg_2_owner_slot,
        identity as __nc_arg_2_offset, identity as __nc_arg_2_len,
        identity as __nc_arg_2_ptr, identity as __nc_arg_2_values,
        identity as __nc_arg_2_written, identity as __nc_arg_0_0,
        identity as __nc_arg_0_0_owner_slot, identity as __nc_arg_0_0_offset,
        identity as __nc_arg_0_0_len, identity as __nc_arg_0_0_ptr,
        identity as __nc_arg_0_0_values, identity as __nc_arg_0_0_written,
        identity as __nc_arg_1_0, identity as __nc_arg_1_0_owner_slot,
        identity as __nc_arg_1_0_offset, identity as __nc_arg_1_0_len,
        identity as __nc_arg_1_0_ptr, identity as __nc_arg_1_0_values,
        identity as __nc_arg_1_0_written, identity as __nc_arg_2_0,
        identity as __nc_arg_2_0_owner_slot, identity as __nc_arg_2_0_offset,
        identity as __nc_arg_2_0_len, identity as __nc_arg_2_0_ptr,
        identity as __nc_arg_2_0_values, identity as __nc_arg_2_0_written,
        identity as __nc_this, identity as __nc_status, identity as __nc_value,
        identity as __nc_raw, identity as __nc_result, identity as __nc_values,
        identity as __nc_writer, identity as __nc_out, identity as __nc_identity,
        identity as __nc_this_0, identity as __nc_status_0, identity as __nc_result_0,
        identity as __nc_prepared, identity as __nc_out_0, identity as __nc_this_1,
        identity as __nc_status_1, identity as __nc_result_1,
        identity as __nc_prepared_0, identity as __nc_out_1,
    };
    impl<
        __NcThreading: ::neocogi_com::ThreadingModel,
    > IPersistenceFactoryClient<__NcThreading> {
        /// Rejects unsupported revisions before any child object is constructed.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:29:8` (module `neocogi.com.persistence`).
        ///
        ///Calls [`IPersistenceFactory::validate`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `version` is passed by value as `u32`.
        ///- `state` is passed as a safe readable `u8` buffer plus offset.
        ///- `child_count` is passed by value as `u32`.
        ///
        ///On success, returns `()`.
        pub fn validate(
            &self,
            __nc_arg_0: u32,
            __nc_arg_1: ::neocogi_com::NcCopyFromBufferOffset<'_, u8>,
            __nc_arg_2: u32,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0 = ::neocogi_com::encode_inline(&__nc_arg_0)?;
            let __nc_arg_1 = ::neocogi_com::EncodedInput::new(__nc_arg_1)?;
            let __nc_arg_2 = ::neocogi_com::encode_inline(&__nc_arg_2)?;
            let __nc_this_0 = self.__nc_pointer.as_raw();
            let __nc_status_0 = unsafe {
                ((*__nc_this_0.vtable)
                    .validate)(
                    __nc_this_0.object,
                    __nc_arg_0,
                    __nc_arg_1.offset(),
                    __nc_arg_1.len(),
                    __nc_arg_1.as_ptr(),
                    __nc_arg_2,
                )
            };
            if __nc_status_0.is_err() {
                return ::core::result::Result::Err(__nc_status_0);
            }
            let __nc_result_0 = ();
            ::core::result::Result::Ok(__nc_result_0)
        }
        /// Returns a fully initialized object; every child is already constructed.
        /// Failure must release resources acquired by this call and publish no root.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:32:8` (module `neocogi.com.persistence`).
        ///
        ///Calls [`IPersistenceFactory::restore`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `version` is passed by value as `u32`.
        ///- `state` is passed as a safe readable `u8` buffer plus offset.
        ///- `children` borrows a `self :: IObjectChildrenAbi` interface for the duration of the call. An interface is required. The callee must call `AddRef` before retaining a present interface.
        ///
        ///On success, returns `:: neocogi_com :: ComPtr < :: neocogi_com :: IUnknown >`. The caller receives one shared owner and releases it when finished.
        pub fn restore(
            &self,
            __nc_arg_0_0: u32,
            __nc_arg_1_0: ::neocogi_com::NcCopyFromBufferOffset<'_, u8>,
            __nc_arg_2_0: ::neocogi_com::ComRef<'_, self::IObjectChildrenAbi>,
        ) -> ::core::result::Result<
            ::neocogi_com::ComPtr<::neocogi_com::IUnknown>,
            ::neocogi_com::NcStatus,
        > {
            let __nc_arg_0_0 = ::neocogi_com::encode_inline(&__nc_arg_0_0)?;
            let __nc_arg_1_0 = ::neocogi_com::EncodedInput::new(__nc_arg_1_0)?;
            let mut __nc_out_1 = ::core::mem::MaybeUninit::<
                ::neocogi_com::ComRaw<::neocogi_com::IUnknown>,
            >::uninit();
            let __nc_this_1 = self.__nc_pointer.as_raw();
            let __nc_status_1 = unsafe {
                ((*__nc_this_1.vtable)
                    .restore)(
                    __nc_this_1.object,
                    __nc_arg_0_0,
                    __nc_arg_1_0.offset(),
                    __nc_arg_1_0.len(),
                    __nc_arg_1_0.as_ptr(),
                    __nc_arg_2_0.as_raw(),
                    __nc_out_1.as_mut_ptr(),
                )
            };
            if __nc_status_1.is_err() {
                return ::core::result::Result::Err(__nc_status_1);
            }
            let __nc_result_1 = (unsafe {
                <::neocogi_com::ComPtr<
                    ::neocogi_com::IUnknown,
                >>::__nc_from_raw_owned_checked(__nc_out_1.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_1)
        }
    }
};
impl IPersistenceFactoryAbi {
    ///Canonical validation form of this interface's typed ABI contract.
    pub const ABI_SIGNATURE: ::neocogi_com::NcAbiSignature = ::neocogi_com::NcAbiSignature::from_static(
        b"\x8B\xB3\x1F\xFD_\xFB\x10\xF3\x8A\xD5G\xE87\x929\x03\xF0\xE1|\xDA\x02%_\xA5\xE5H\xAEo\0\x04\xE9\xCE",
    );
    ///Static descriptors for this interface's ABI methods.
    pub const METHOD_DESCRIPTORS: [::neocogi_com::NcMethodDescriptor; 2usize] = [
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("validate"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 3u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("version"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<u32>(),
                    ),
                },
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("state"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::CopyFromBufferOffset,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_plain_type::<u8>(),
                    ),
                },
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("child_count"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<u32>(),
                    ),
                },
            ] as &'static [::neocogi_com::NcMethodParamDescriptor])
                .as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Void,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::NcAbiType::Void,
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("restore"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 3u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("version"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<u32>(),
                    ),
                },
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("state"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::CopyFromBufferOffset,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_plain_type::<u8>(),
                    ),
                },
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("children"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(::neocogi_com::NcAbiPassing::ComRef {
                        mode: <::neocogi_com::ThreadLocal as ::neocogi_com::ThreadingModel>::ABI_THREADING_MODE,
                    }),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_interface(
                        <self::IObjectChildrenAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
                    ),
                },
            ] as &'static [::neocogi_com::NcMethodParamDescriptor])
                .as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(::neocogi_com::NcAbiPassing::ComPtrOut {
                    mode: <::neocogi_com::ThreadLocal as ::neocogi_com::ThreadingModel>::ABI_THREADING_MODE,
                }),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_interface(
                    <::neocogi_com::IUnknown as ::neocogi_com::ComInterface>::INTERFACE_ID,
                ),
            },
        },
    ];
    ///Static ABI descriptor for this interface.
    pub const INTERFACE_DESCRIPTOR: ::neocogi_com::NcInterfaceDescriptor = ::neocogi_com::NcInterfaceDescriptor {
        interface_id: <Self as ::neocogi_com::ComInterface>::IID,
        qualified_name: ::neocogi_com::NcStaticUtf8::from_static(
            "neocogi.com.persistence.IPersistenceFactory",
        ),
        abi_signature: Self::ABI_SIGNATURE,
        base: ::neocogi_com::NcInterfaceId::NONE,
        method_count: 2u32,
        methods: Self::METHOD_DESCRIPTORS.as_ptr(),
    };
    /// Distinct projections in derived-first, declared-parent order.
    /// Shared ancestors occur once; repeated identities must have the same ABI.
    pub const INTERFACE_CHAIN: [&'static ::neocogi_com::NcInterfaceDescriptor; ::neocogi_com::__nc_object_interface_count(
        &[&[&Self::INTERFACE_DESCRIPTOR]],
    )] = ::neocogi_com::__nc_interface_chain(&Self::INTERFACE_DESCRIPTOR, &[]);
}
