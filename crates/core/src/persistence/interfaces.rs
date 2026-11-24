// Generated from core/schemas/neocogi/com/persistence/objects.ncidl.
// Regenerate: NC_UPDATE_PERSISTENCE_BINDINGS=1 cargo test -p neocogi-idl persistence_bindings_match_idl

/// Stable logical identity of this generated contract module.
pub const MODULE_ID: crate::NcGuid = crate::NcGuid::from_fields(
    3545662286u32,
    20204u16,
    22946u16,
    [140u8, 82u8, 62u8, 53u8, 229u8, 28u8, 242u8, 16u8],
);
/// Transitive canonical ABI fingerprint of this generated module.
pub const ABI_FINGERPRINT: [::core::primitive::u8; 32] = [
    63u8, 92u8, 248u8, 3u8, 103u8, 175u8, 80u8, 64u8, 18u8, 43u8, 76u8, 95u8, 116u8,
    78u8, 127u8, 153u8, 18u8, 50u8, 206u8, 176u8, 21u8, 233u8, 81u8, 222u8, 235u8, 243u8,
    184u8, 237u8, 188u8, 237u8, 197u8, 192u8,
];
/// Exact dotted module name derived from the normalized IDL path.
pub const MODULE_NAME: &::core::primitive::str = "neocogi.com.persistence";
/// An ordered, owning list used only while constructing a persistent parent.
/// IDL source: `neocogi/com/persistence/objects.ncidl:2:11` (module `neocogi.com.persistence`).
pub trait IObjectChildren {
    /// Returns the number of fully initialized child objects.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:4:8` (module `neocogi.com.persistence`).
    fn count(&self) -> ::core::result::Result<::core::primitive::u32, crate::NcStatus>;
    /// Returns an owned reference; indices must be less than count.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:6:8` (module `neocogi.com.persistence`).
    fn get(
        &self,
        index: ::core::primitive::u32,
    ) -> ::core::result::Result<crate::ComPtr<crate::IUnknown>, crate::NcStatus>;
}
///Interface marker for [`IObjectChildren`]; handles carry its object/table pair.
pub enum IObjectChildrenAbi {}
#[repr(C)]
///C-compatible vtable for [`IObjectChildren`].
pub struct IObjectChildrenVTable {
    /// Common identity, query and reference-count slots.
    pub unknown: crate::IUnknownVTable,
    /// Returns the number of fully initialized child objects.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:4:8` (module `neocogi.com.persistence`).
    ///Raw COM ABI vtable slot for [`IObjectChildren::count`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u32 as crate :: ComValue > :: AbiRepr`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub count: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::u32 as crate::ComValue>::AbiRepr,
    ) -> crate::NcStatus,
    /// Returns an owned reference; indices must be less than count.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:6:8` (module `neocogi.com.persistence`).
    ///Raw COM ABI vtable slot for [`IObjectChildren::get`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage.
    ///- `index` uses ABI storage `< :: core :: primitive :: u32 as crate :: ComValue > :: AbiRepr` for `:: core :: primitive :: u32`. Passed by value.
    ///
    ///On success, `out` receives `crate :: ComRaw < crate :: IUnknown >` for `crate :: ComPtr < crate :: IUnknown >`. The caller receives one shared owner and releases it when finished.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub get: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0: <::core::primitive::u32 as crate::ComValue>::AbiRepr,
        __nc_out: *mut crate::ComRaw<crate::IUnknown>,
    ) -> crate::NcStatus,
}
unsafe impl crate::ComInterface for IObjectChildrenAbi {
    const IID: crate::NcGuid = crate::NcGuid {
        data1: 2422821477u32,
        data2: 38314u16,
        data3: 22971u16,
        data4: [152u8, 38u8, 218u8, 182u8, 237u8, 193u8, 229u8, 147u8],
    };
    const INTERFACE_ID: crate::NcInterfaceId = crate::NcInterfaceId::new(
        crate::NcGuid {
            data1: 2422821477u32,
            data2: 38314u16,
            data3: 22971u16,
            data4: [152u8, 38u8, 218u8, 182u8, 237u8, 193u8, 229u8, 147u8],
        },
        IObjectChildrenAbi::ABI_SIGNATURE,
    );
    type VTable = IObjectChildrenVTable;
    type Threading = crate::ThreadLocal;
}
#[repr(transparent)]
///Borrowed Rust method wrapper for [`IObjectChildren`].
pub struct IObjectChildrenWrapper<
    __NcThreading: crate::ThreadingModel = crate::ThreadLocal,
> {
    /// Opaque pointer borrowed from retained interface/control storage; provider entry admits value access.
    __nc_pointer: crate::ComInterfacePointer<IObjectChildrenAbi, __NcThreading>,
}
unsafe impl crate::ComRustInterface for IObjectChildrenAbi {
    type Wrapper<__NcThreading: crate::ThreadingModel> = IObjectChildrenWrapper<
        __NcThreading,
    >;
    fn __nc_wrapper_ref<__NcThreading: crate::ThreadingModel>(
        pointer: &crate::ComInterfacePointer<Self, __NcThreading>,
    ) -> &Self::Wrapper<__NcThreading> {
        unsafe {
            &*(::core::ptr::from_ref(pointer)
                as *const IObjectChildrenWrapper<__NcThreading>)
        }
    }
    fn __nc_wrapper_mut<__NcThreading: crate::ThreadingModel>(
        pointer: &mut crate::ComInterfacePointer<Self, __NcThreading>,
    ) -> &mut Self::Wrapper<__NcThreading> {
        unsafe {
            &mut *(::core::ptr::from_mut(pointer)
                as *mut IObjectChildrenWrapper<__NcThreading>)
        }
    }
}
const _: () = {
    #[allow(unused_imports)]
    use ::core::convert::{
        identity as __nc_dispatch_object, identity as __nc_dispatch_value,
        identity as __nc_arg_0, identity as __nc_arg_0_owner_slot,
        identity as __nc_arg_0_owner_table, identity as __nc_arg_0_owner_original,
        identity as __nc_arg_0_offset, identity as __nc_arg_0_len,
        identity as __nc_arg_0_ptr, identity as __nc_arg_0_values,
        identity as __nc_arg_0_written, identity as __nc_this, identity as __nc_status,
        identity as __nc_value, identity as __nc_raw, identity as __nc_result,
        identity as __nc_values, identity as __nc_writer, identity as __nc_out,
        identity as __nc_scoped_context, identity as __nc_scoped_callback,
        identity as __nc_identity, identity as __nc_this_0, identity as __nc_status_0,
        identity as __nc_result_0, identity as __nc_prepared, identity as __nc_out_0,
        identity as __nc_this_1, identity as __nc_status_1, identity as __nc_result_1,
        identity as __nc_prepared_0, identity as __nc_out_1,
    };
    impl<__NcThreading: crate::ThreadingModel> IObjectChildrenWrapper<__NcThreading> {
        /// Returns the number of fully initialized child objects.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:4:8` (module `neocogi.com.persistence`).
        ///
        ///Calls [`IObjectChildren::count`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Call errors remain visible. Generated Rust thunks normalize non-negative `Result::Err` values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: u32` from the ABI out slot.
        pub fn count(
            &self,
        ) -> ::core::result::Result<::core::primitive::u32, crate::NcStatus> {
            let mut __nc_out_0 = ::core::mem::MaybeUninit::<
                <::core::primitive::u32 as crate::ComValue>::AbiRepr,
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
                crate::decode_inline::<
                    ::core::primitive::u32,
                >(__nc_out_0.assume_init_ref())
            })?;
            ::core::result::Result::Ok(__nc_result_0)
        }
        /// Returns an owned reference; indices must be less than count.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:6:8` (module `neocogi.com.persistence`).
        ///
        ///Calls [`IObjectChildren::get`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Call errors remain visible. Generated Rust thunks normalize non-negative `Result::Err` values to `NC_E_INVALID_ARG`.
        ///- `index` is passed by value as `:: core :: primitive :: u32`.
        ///
        ///On success, returns `crate :: ComPtr < crate :: IUnknown >`. The caller receives one shared owner and releases it when finished.
        pub fn get(
            &self,
            __nc_arg_0: ::core::primitive::u32,
        ) -> ::core::result::Result<crate::ComPtr<crate::IUnknown>, crate::NcStatus> {
            let __nc_arg_0 = crate::encode_inline(&__nc_arg_0)?;
            let mut __nc_out_1 = ::core::mem::MaybeUninit::<
                crate::ComRaw<crate::IUnknown>,
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
                <crate::ComPtr<
                    crate::IUnknown,
                >>::__nc_from_raw_owned_checked(__nc_out_1.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_1)
        }
    }
};
impl IObjectChildrenAbi {
    ///Canonical validation form of this interface's typed ABI contract.
    pub const ABI_SIGNATURE: crate::NcAbiDigest = crate::NcAbiDigest::from_static(
        b"\xB0HY_r\xA6\x86\t\xD5q\x82\xE6\x17\x18\x0C\x81\xB0%\xB7\x92Ia\xD6\x95\xFBQ\x86j\xD1\xA3\xA2\xEF",
    );
    ///Static descriptors for this interface's ABI methods.
    pub const METHOD_DESCRIPTORS: [crate::NcMethodDescriptor; 2usize] = [
        crate::NcMethodDescriptor {
            name: crate::NcStaticUtf8::from_static("count"),
            receiver: crate::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: (&[] as &'static [crate::NcMethodParamDescriptor]).as_ptr(),
            returns: crate::NcMethodReturnDescriptor {
                passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                    crate::NcAbiPassing::Out,
                ),
                ty: crate::NcAbiTypeDescriptor::from_abi_type(
                    crate::__nc_abi_type::<::core::primitive::u32>(),
                ),
            },
        },
        crate::NcMethodDescriptor {
            name: crate::NcStaticUtf8::from_static("get"),
            receiver: crate::NC_ABI_RECEIVER_SHARED,
            param_count: 1u32,
            params: (&[
                crate::NcMethodParamDescriptor {
                    name: crate::NcStaticUtf8::from_static("index"),
                    passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                        crate::NcAbiPassing::Value,
                    ),
                    ty: crate::NcAbiTypeDescriptor::from_abi_type(
                        crate::__nc_abi_type::<::core::primitive::u32>(),
                    ),
                },
            ] as &'static [crate::NcMethodParamDescriptor])
                .as_ptr(),
            returns: crate::NcMethodReturnDescriptor {
                passing: crate::NcAbiPassingDescriptor::from_abi_passing(crate::NcAbiPassing::ComPtrOut {
                    mode: <crate::ThreadLocal as crate::ThreadingModel>::ABI_THREADING_MODE,
                }),
                ty: crate::NcAbiTypeDescriptor::from_interface(
                    <crate::IUnknown as crate::ComInterface>::INTERFACE_ID,
                ),
            },
        },
    ];
    ///Static ABI descriptor for this interface.
    pub const INTERFACE_DESCRIPTOR: crate::NcInterfaceDescriptor = crate::NcInterfaceDescriptor {
        interface_id: <Self as crate::ComInterface>::IID,
        qualified_name: crate::NcStaticUtf8::from_static(
            "neocogi.com.persistence.IObjectChildren",
        ),
        abi_signature: Self::ABI_SIGNATURE,
        base: crate::NcInterfaceId::NONE,
        method_count: 2u32,
        methods: Self::METHOD_DESCRIPTORS.as_ptr(),
    };
    /// Distinct projections in derived-first, declared-parent order.
    /// Shared ancestors occur once; repeated identities must have the same ABI.
    pub const INTERFACE_CHAIN: [&'static crate::NcInterfaceDescriptor; crate::__nc_object_interface_count(
        &[&[&Self::INTERFACE_DESCRIPTOR]],
    )] = crate::__nc_interface_chain(&Self::INTERFACE_DESCRIPTOR, &[]);
}
impl IObjectChildrenAbi {
    #[doc(hidden)]
    pub const __NC_RUST_BINDING: [::core::primitive::u8; 32] = [
        88u8, 108u8, 235u8, 200u8, 101u8, 70u8, 211u8, 68u8, 94u8, 119u8, 141u8, 106u8,
        75u8, 151u8, 29u8, 87u8, 116u8, 65u8, 88u8, 202u8, 84u8, 21u8, 193u8, 132u8,
        105u8, 180u8, 17u8, 164u8, 206u8, 80u8, 9u8, 125u8,
    ];
}
/// Optional capability that supplies the data needed to save an owned object tree.
/// IDL source: `neocogi/com/persistence/objects.ncidl:10:11` (module `neocogi.com.persistence`).
pub trait IPersistable {
    /// Identifies the registered construction recipe, not this object instance.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:12:8` (module `neocogi.com.persistence`).
    fn type_id(&self) -> ::core::result::Result<crate::NcGuid, crate::NcStatus>;
    /// Identifies the persistent initialization schema, independently of COM ABI.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:14:8` (module `neocogi.com.persistence`).
    fn schema_version(
        &self,
    ) -> ::core::result::Result<::core::primitive::u32, crate::NcStatus>;
    /// Exports initialization data only: no receivers, runtime pointers or caches.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:16:8` (module `neocogi.com.persistence`).
    fn save_state(
        &self,
    ) -> ::core::result::Result<crate::ComPtr<crate::INcBytesAbi>, crate::NcStatus>;
    /// Returns the number of owned persistent children; parent links are excluded.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:18:8` (module `neocogi.com.persistence`).
    fn child_count(
        &self,
    ) -> ::core::result::Result<::core::primitive::u32, crate::NcStatus>;
    /// Returns one child in the order described by the parent's initialization.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:20:8` (module `neocogi.com.persistence`).
    fn child(
        &self,
        index: ::core::primitive::u32,
    ) -> ::core::result::Result<crate::ComPtr<crate::IUnknown>, crate::NcStatus>;
}
///Interface marker for [`IPersistable`]; handles carry its object/table pair.
pub enum IPersistableAbi {}
#[repr(C)]
///C-compatible vtable for [`IPersistable`].
pub struct IPersistableVTable {
    /// Common identity, query and reference-count slots.
    pub unknown: crate::IUnknownVTable,
    /// Identifies the registered construction recipe, not this object instance.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:12:8` (module `neocogi.com.persistence`).
    ///Raw COM ABI vtable slot for [`IPersistable::type_id`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage.
    ///
    ///On success, `out` receives `< crate :: NcGuid as crate :: ComValue > :: AbiRepr`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub type_id: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <crate::NcGuid as crate::ComValue>::AbiRepr,
    ) -> crate::NcStatus,
    /// Identifies the persistent initialization schema, independently of COM ABI.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:14:8` (module `neocogi.com.persistence`).
    ///Raw COM ABI vtable slot for [`IPersistable::schema_version`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u32 as crate :: ComValue > :: AbiRepr`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub schema_version: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::u32 as crate::ComValue>::AbiRepr,
    ) -> crate::NcStatus,
    /// Exports initialization data only: no receivers, runtime pointers or caches.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:16:8` (module `neocogi.com.persistence`).
    ///Raw COM ABI vtable slot for [`IPersistable::save_state`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage.
    ///
    ///On success, `out` receives `crate :: ComRaw < crate :: INcBytesAbi >` for `crate :: ComPtr < crate :: INcBytesAbi >`. The caller receives one shared owner and releases it when finished.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub save_state: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut crate::ComRaw<crate::INcBytesAbi>,
    ) -> crate::NcStatus,
    /// Returns the number of owned persistent children; parent links are excluded.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:18:8` (module `neocogi.com.persistence`).
    ///Raw COM ABI vtable slot for [`IPersistable::child_count`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u32 as crate :: ComValue > :: AbiRepr`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub child_count: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::u32 as crate::ComValue>::AbiRepr,
    ) -> crate::NcStatus,
    /// Returns one child in the order described by the parent's initialization.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:20:8` (module `neocogi.com.persistence`).
    ///Raw COM ABI vtable slot for [`IPersistable::child`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage.
    ///- `index` uses ABI storage `< :: core :: primitive :: u32 as crate :: ComValue > :: AbiRepr` for `:: core :: primitive :: u32`. Passed by value.
    ///
    ///On success, `out` receives `crate :: ComRaw < crate :: IUnknown >` for `crate :: ComPtr < crate :: IUnknown >`. The caller receives one shared owner and releases it when finished.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub child: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0: <::core::primitive::u32 as crate::ComValue>::AbiRepr,
        __nc_out: *mut crate::ComRaw<crate::IUnknown>,
    ) -> crate::NcStatus,
}
unsafe impl crate::ComInterface for IPersistableAbi {
    const IID: crate::NcGuid = crate::NcGuid {
        data1: 1888826366u32,
        data2: 2270u16,
        data3: 21695u16,
        data4: [141u8, 234u8, 184u8, 83u8, 224u8, 13u8, 180u8, 88u8],
    };
    const INTERFACE_ID: crate::NcInterfaceId = crate::NcInterfaceId::new(
        crate::NcGuid {
            data1: 1888826366u32,
            data2: 2270u16,
            data3: 21695u16,
            data4: [141u8, 234u8, 184u8, 83u8, 224u8, 13u8, 180u8, 88u8],
        },
        IPersistableAbi::ABI_SIGNATURE,
    );
    type VTable = IPersistableVTable;
    type Threading = crate::ThreadLocal;
}
#[repr(transparent)]
///Borrowed Rust method wrapper for [`IPersistable`].
pub struct IPersistableWrapper<
    __NcThreading: crate::ThreadingModel = crate::ThreadLocal,
> {
    /// Opaque pointer borrowed from retained interface/control storage; provider entry admits value access.
    __nc_pointer: crate::ComInterfacePointer<IPersistableAbi, __NcThreading>,
}
unsafe impl crate::ComRustInterface for IPersistableAbi {
    type Wrapper<__NcThreading: crate::ThreadingModel> = IPersistableWrapper<
        __NcThreading,
    >;
    fn __nc_wrapper_ref<__NcThreading: crate::ThreadingModel>(
        pointer: &crate::ComInterfacePointer<Self, __NcThreading>,
    ) -> &Self::Wrapper<__NcThreading> {
        unsafe {
            &*(::core::ptr::from_ref(pointer)
                as *const IPersistableWrapper<__NcThreading>)
        }
    }
    fn __nc_wrapper_mut<__NcThreading: crate::ThreadingModel>(
        pointer: &mut crate::ComInterfacePointer<Self, __NcThreading>,
    ) -> &mut Self::Wrapper<__NcThreading> {
        unsafe {
            &mut *(::core::ptr::from_mut(pointer)
                as *mut IPersistableWrapper<__NcThreading>)
        }
    }
}
const _: () = {
    #[allow(unused_imports)]
    use ::core::convert::{
        identity as __nc_dispatch_object, identity as __nc_dispatch_value,
        identity as __nc_arg_0, identity as __nc_arg_0_owner_slot,
        identity as __nc_arg_0_owner_table, identity as __nc_arg_0_owner_original,
        identity as __nc_arg_0_offset, identity as __nc_arg_0_len,
        identity as __nc_arg_0_ptr, identity as __nc_arg_0_values,
        identity as __nc_arg_0_written, identity as __nc_this, identity as __nc_status,
        identity as __nc_value, identity as __nc_raw, identity as __nc_result,
        identity as __nc_values, identity as __nc_writer, identity as __nc_out,
        identity as __nc_scoped_context, identity as __nc_scoped_callback,
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
    impl<__NcThreading: crate::ThreadingModel> IPersistableWrapper<__NcThreading> {
        /// Identifies the registered construction recipe, not this object instance.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:12:8` (module `neocogi.com.persistence`).
        ///
        ///Calls [`IPersistable::type_id`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Call errors remain visible. Generated Rust thunks normalize non-negative `Result::Err` values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `crate :: NcGuid` from the ABI out slot.
        pub fn type_id(&self) -> ::core::result::Result<crate::NcGuid, crate::NcStatus> {
            let mut __nc_out_0 = ::core::mem::MaybeUninit::<
                <crate::NcGuid as crate::ComValue>::AbiRepr,
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
                crate::decode_inline::<crate::NcGuid>(__nc_out_0.assume_init_ref())
            })?;
            ::core::result::Result::Ok(__nc_result_0)
        }
        /// Identifies the persistent initialization schema, independently of COM ABI.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:14:8` (module `neocogi.com.persistence`).
        ///
        ///Calls [`IPersistable::schema_version`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Call errors remain visible. Generated Rust thunks normalize non-negative `Result::Err` values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: u32` from the ABI out slot.
        pub fn schema_version(
            &self,
        ) -> ::core::result::Result<::core::primitive::u32, crate::NcStatus> {
            let mut __nc_out_1 = ::core::mem::MaybeUninit::<
                <::core::primitive::u32 as crate::ComValue>::AbiRepr,
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
                crate::decode_inline::<
                    ::core::primitive::u32,
                >(__nc_out_1.assume_init_ref())
            })?;
            ::core::result::Result::Ok(__nc_result_1)
        }
        /// Exports initialization data only: no receivers, runtime pointers or caches.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:16:8` (module `neocogi.com.persistence`).
        ///
        ///Calls [`IPersistable::save_state`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Call errors remain visible. Generated Rust thunks normalize non-negative `Result::Err` values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `crate :: ComPtr < crate :: INcBytesAbi >`. The caller receives one shared owner and releases it when finished.
        pub fn save_state(
            &self,
        ) -> ::core::result::Result<crate::ComPtr<crate::INcBytesAbi>, crate::NcStatus> {
            let mut __nc_out_2 = ::core::mem::MaybeUninit::<
                crate::ComRaw<crate::INcBytesAbi>,
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
                <crate::ComPtr<
                    crate::INcBytesAbi,
                >>::__nc_from_raw_owned_checked(__nc_out_2.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_2)
        }
        /// Returns the number of owned persistent children; parent links are excluded.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:18:8` (module `neocogi.com.persistence`).
        ///
        ///Calls [`IPersistable::child_count`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Call errors remain visible. Generated Rust thunks normalize non-negative `Result::Err` values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: u32` from the ABI out slot.
        pub fn child_count(
            &self,
        ) -> ::core::result::Result<::core::primitive::u32, crate::NcStatus> {
            let mut __nc_out_3 = ::core::mem::MaybeUninit::<
                <::core::primitive::u32 as crate::ComValue>::AbiRepr,
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
                crate::decode_inline::<
                    ::core::primitive::u32,
                >(__nc_out_3.assume_init_ref())
            })?;
            ::core::result::Result::Ok(__nc_result_3)
        }
        /// Returns one child in the order described by the parent's initialization.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:20:8` (module `neocogi.com.persistence`).
        ///
        ///Calls [`IPersistable::child`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Call errors remain visible. Generated Rust thunks normalize non-negative `Result::Err` values to `NC_E_INVALID_ARG`.
        ///- `index` is passed by value as `:: core :: primitive :: u32`.
        ///
        ///On success, returns `crate :: ComPtr < crate :: IUnknown >`. The caller receives one shared owner and releases it when finished.
        pub fn child(
            &self,
            __nc_arg_0: ::core::primitive::u32,
        ) -> ::core::result::Result<crate::ComPtr<crate::IUnknown>, crate::NcStatus> {
            let __nc_arg_0 = crate::encode_inline(&__nc_arg_0)?;
            let mut __nc_out_4 = ::core::mem::MaybeUninit::<
                crate::ComRaw<crate::IUnknown>,
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
                <crate::ComPtr<
                    crate::IUnknown,
                >>::__nc_from_raw_owned_checked(__nc_out_4.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_4)
        }
    }
};
impl IPersistableAbi {
    ///Canonical validation form of this interface's typed ABI contract.
    pub const ABI_SIGNATURE: crate::NcAbiDigest = crate::NcAbiDigest::from_static(
        b"`\xEE\xD86`i\x9A\xAA\xB9-Y\x9C\xB1\xBA\x12s\xD4d0xN\x8Bc\xCB\xC9\xA4j\x8F\xFD!\xF4E",
    );
    ///Static descriptors for this interface's ABI methods.
    pub const METHOD_DESCRIPTORS: [crate::NcMethodDescriptor; 5usize] = [
        crate::NcMethodDescriptor {
            name: crate::NcStaticUtf8::from_static("type_id"),
            receiver: crate::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: (&[] as &'static [crate::NcMethodParamDescriptor]).as_ptr(),
            returns: crate::NcMethodReturnDescriptor {
                passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                    crate::NcAbiPassing::Out,
                ),
                ty: crate::NcAbiTypeDescriptor::from_abi_type(
                    crate::__nc_abi_type::<crate::NcGuid>(),
                ),
            },
        },
        crate::NcMethodDescriptor {
            name: crate::NcStaticUtf8::from_static("schema_version"),
            receiver: crate::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: (&[] as &'static [crate::NcMethodParamDescriptor]).as_ptr(),
            returns: crate::NcMethodReturnDescriptor {
                passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                    crate::NcAbiPassing::Out,
                ),
                ty: crate::NcAbiTypeDescriptor::from_abi_type(
                    crate::__nc_abi_type::<::core::primitive::u32>(),
                ),
            },
        },
        crate::NcMethodDescriptor {
            name: crate::NcStaticUtf8::from_static("save_state"),
            receiver: crate::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: (&[] as &'static [crate::NcMethodParamDescriptor]).as_ptr(),
            returns: crate::NcMethodReturnDescriptor {
                passing: crate::NcAbiPassingDescriptor::from_abi_passing(crate::NcAbiPassing::ComPtrOut {
                    mode: <crate::ThreadLocal as crate::ThreadingModel>::ABI_THREADING_MODE,
                }),
                ty: crate::NcAbiTypeDescriptor::from_interface(
                    <crate::INcBytesAbi as crate::ComInterface>::INTERFACE_ID,
                ),
            },
        },
        crate::NcMethodDescriptor {
            name: crate::NcStaticUtf8::from_static("child_count"),
            receiver: crate::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: (&[] as &'static [crate::NcMethodParamDescriptor]).as_ptr(),
            returns: crate::NcMethodReturnDescriptor {
                passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                    crate::NcAbiPassing::Out,
                ),
                ty: crate::NcAbiTypeDescriptor::from_abi_type(
                    crate::__nc_abi_type::<::core::primitive::u32>(),
                ),
            },
        },
        crate::NcMethodDescriptor {
            name: crate::NcStaticUtf8::from_static("child"),
            receiver: crate::NC_ABI_RECEIVER_SHARED,
            param_count: 1u32,
            params: (&[
                crate::NcMethodParamDescriptor {
                    name: crate::NcStaticUtf8::from_static("index"),
                    passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                        crate::NcAbiPassing::Value,
                    ),
                    ty: crate::NcAbiTypeDescriptor::from_abi_type(
                        crate::__nc_abi_type::<::core::primitive::u32>(),
                    ),
                },
            ] as &'static [crate::NcMethodParamDescriptor])
                .as_ptr(),
            returns: crate::NcMethodReturnDescriptor {
                passing: crate::NcAbiPassingDescriptor::from_abi_passing(crate::NcAbiPassing::ComPtrOut {
                    mode: <crate::ThreadLocal as crate::ThreadingModel>::ABI_THREADING_MODE,
                }),
                ty: crate::NcAbiTypeDescriptor::from_interface(
                    <crate::IUnknown as crate::ComInterface>::INTERFACE_ID,
                ),
            },
        },
    ];
    ///Static ABI descriptor for this interface.
    pub const INTERFACE_DESCRIPTOR: crate::NcInterfaceDescriptor = crate::NcInterfaceDescriptor {
        interface_id: <Self as crate::ComInterface>::IID,
        qualified_name: crate::NcStaticUtf8::from_static(
            "neocogi.com.persistence.IPersistable",
        ),
        abi_signature: Self::ABI_SIGNATURE,
        base: crate::NcInterfaceId::NONE,
        method_count: 5u32,
        methods: Self::METHOD_DESCRIPTORS.as_ptr(),
    };
    /// Distinct projections in derived-first, declared-parent order.
    /// Shared ancestors occur once; repeated identities must have the same ABI.
    pub const INTERFACE_CHAIN: [&'static crate::NcInterfaceDescriptor; crate::__nc_object_interface_count(
        &[&[&Self::INTERFACE_DESCRIPTOR]],
    )] = crate::__nc_interface_chain(&Self::INTERFACE_DESCRIPTOR, &[]);
}
impl IPersistableAbi {
    #[doc(hidden)]
    pub const __NC_RUST_BINDING: [::core::primitive::u8; 32] = [
        11u8, 231u8, 55u8, 21u8, 117u8, 37u8, 133u8, 32u8, 191u8, 51u8, 109u8, 60u8,
        121u8, 128u8, 121u8, 175u8, 189u8, 188u8, 182u8, 73u8, 111u8, 66u8, 93u8, 74u8,
        198u8, 82u8, 174u8, 139u8, 251u8, 28u8, 243u8, 253u8,
    ];
}
/// A provider bound to the runtime services required by its construction recipe.
/// IDL source: `neocogi/com/persistence/objects.ncidl:24:11` (module `neocogi.com.persistence`).
pub trait IPersistenceFactory {
    /// Rejects unsupported revisions before any child object is constructed.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:26:8` (module `neocogi.com.persistence`).
    fn validate(
        &self,
        version: ::core::primitive::u32,
        state: crate::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
        child_count: ::core::primitive::u32,
    ) -> ::core::result::Result<(), crate::NcStatus>;
    /// Returns a fully initialized object; every child is already constructed.
    /// Failure must release resources acquired by this call and publish no root.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:29:8` (module `neocogi.com.persistence`).
    fn restore(
        &self,
        version: ::core::primitive::u32,
        state: crate::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
        children: crate::ComRef<'_, self::IObjectChildrenAbi>,
    ) -> ::core::result::Result<crate::ComPtr<crate::IUnknown>, crate::NcStatus>;
}
///Interface marker for [`IPersistenceFactory`]; handles carry its object/table pair.
pub enum IPersistenceFactoryAbi {}
#[repr(C)]
///C-compatible vtable for [`IPersistenceFactory`].
pub struct IPersistenceFactoryVTable {
    /// Common identity, query and reference-count slots.
    pub unknown: crate::IUnknownVTable,
    /// Rejects unsupported revisions before any child object is constructed.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:26:8` (module `neocogi.com.persistence`).
    ///Raw COM ABI vtable slot for [`IPersistenceFactory::validate`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table.
    ///- `version` uses ABI storage `< :: core :: primitive :: u32 as crate :: ComValue > :: AbiRepr` for `:: core :: primitive :: u32`. Passed by value.
    ///- `state` uses ABI storage `* const < :: core :: primitive :: u8 as crate :: ComValue > :: AbiRepr` for `crate :: NcCopyFromBufferOffset < '_ , :: core :: primitive :: u8 >`. The readable buffer pointer is accompanied by `offset: u64` and `len: u64`.
    ///- `child_count` uses ABI storage `< :: core :: primitive :: u32 as crate :: ComValue > :: AbiRepr` for `:: core :: primitive :: u32`. Passed by value.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub validate: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0: <::core::primitive::u32 as crate::ComValue>::AbiRepr,
        __nc_arg_1_offset: ::core::primitive::u64,
        __nc_arg_1_len: ::core::primitive::u64,
        __nc_arg_1_ptr: *const <::core::primitive::u8 as crate::ComValue>::AbiRepr,
        __nc_arg_2: <::core::primitive::u32 as crate::ComValue>::AbiRepr,
    ) -> crate::NcStatus,
    /// Returns a fully initialized object; every child is already constructed.
    /// Failure must release resources acquired by this call and publish no root.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:29:8` (module `neocogi.com.persistence`).
    ///Raw COM ABI vtable slot for [`IPersistenceFactory::restore`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage.
    ///- `version` uses ABI storage `< :: core :: primitive :: u32 as crate :: ComValue > :: AbiRepr` for `:: core :: primitive :: u32`. Passed by value.
    ///- `state` uses ABI storage `* const < :: core :: primitive :: u8 as crate :: ComValue > :: AbiRepr` for `crate :: NcCopyFromBufferOffset < '_ , :: core :: primitive :: u8 >`. The readable buffer pointer is accompanied by `offset: u64` and `len: u64`.
    ///- `children` uses ABI storage `crate :: ComRaw < self :: IObjectChildrenAbi >` for `crate :: ComRef < '_ , self :: IObjectChildrenAbi >`. The non-null object/table pair lends live shared access through the call.
    ///
    ///On success, `out` receives `crate :: ComRaw < crate :: IUnknown >` for `crate :: ComPtr < crate :: IUnknown >`. The caller receives one shared owner and releases it when finished.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub restore: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_0: <::core::primitive::u32 as crate::ComValue>::AbiRepr,
        __nc_arg_1_0_offset: ::core::primitive::u64,
        __nc_arg_1_0_len: ::core::primitive::u64,
        __nc_arg_1_0_ptr: *const <::core::primitive::u8 as crate::ComValue>::AbiRepr,
        __nc_arg_2_0: crate::ComRaw<self::IObjectChildrenAbi>,
        __nc_out: *mut crate::ComRaw<crate::IUnknown>,
    ) -> crate::NcStatus,
}
unsafe impl crate::ComInterface for IPersistenceFactoryAbi {
    const IID: crate::NcGuid = crate::NcGuid {
        data1: 3732855007u32,
        data2: 16637u16,
        data3: 23717u16,
        data4: [187u8, 2u8, 64u8, 199u8, 96u8, 184u8, 142u8, 53u8],
    };
    const INTERFACE_ID: crate::NcInterfaceId = crate::NcInterfaceId::new(
        crate::NcGuid {
            data1: 3732855007u32,
            data2: 16637u16,
            data3: 23717u16,
            data4: [187u8, 2u8, 64u8, 199u8, 96u8, 184u8, 142u8, 53u8],
        },
        IPersistenceFactoryAbi::ABI_SIGNATURE,
    );
    type VTable = IPersistenceFactoryVTable;
    type Threading = crate::ThreadLocal;
}
#[repr(transparent)]
///Borrowed Rust method wrapper for [`IPersistenceFactory`].
pub struct IPersistenceFactoryWrapper<
    __NcThreading: crate::ThreadingModel = crate::ThreadLocal,
> {
    /// Opaque pointer borrowed from retained interface/control storage; provider entry admits value access.
    __nc_pointer: crate::ComInterfacePointer<IPersistenceFactoryAbi, __NcThreading>,
}
unsafe impl crate::ComRustInterface for IPersistenceFactoryAbi {
    type Wrapper<__NcThreading: crate::ThreadingModel> = IPersistenceFactoryWrapper<
        __NcThreading,
    >;
    fn __nc_wrapper_ref<__NcThreading: crate::ThreadingModel>(
        pointer: &crate::ComInterfacePointer<Self, __NcThreading>,
    ) -> &Self::Wrapper<__NcThreading> {
        unsafe {
            &*(::core::ptr::from_ref(pointer)
                as *const IPersistenceFactoryWrapper<__NcThreading>)
        }
    }
    fn __nc_wrapper_mut<__NcThreading: crate::ThreadingModel>(
        pointer: &mut crate::ComInterfacePointer<Self, __NcThreading>,
    ) -> &mut Self::Wrapper<__NcThreading> {
        unsafe {
            &mut *(::core::ptr::from_mut(pointer)
                as *mut IPersistenceFactoryWrapper<__NcThreading>)
        }
    }
}
const _: () = {
    #[allow(unused_imports)]
    use ::core::convert::{
        identity as __nc_dispatch_object, identity as __nc_dispatch_value,
        identity as __nc_arg_0, identity as __nc_arg_0_owner_slot,
        identity as __nc_arg_0_owner_table, identity as __nc_arg_0_owner_original,
        identity as __nc_arg_0_offset, identity as __nc_arg_0_len,
        identity as __nc_arg_0_ptr, identity as __nc_arg_0_values,
        identity as __nc_arg_0_written, identity as __nc_arg_1,
        identity as __nc_arg_1_owner_slot, identity as __nc_arg_1_owner_table,
        identity as __nc_arg_1_owner_original, identity as __nc_arg_1_offset,
        identity as __nc_arg_1_len, identity as __nc_arg_1_ptr,
        identity as __nc_arg_1_values, identity as __nc_arg_1_written,
        identity as __nc_arg_2, identity as __nc_arg_2_owner_slot,
        identity as __nc_arg_2_owner_table, identity as __nc_arg_2_owner_original,
        identity as __nc_arg_2_offset, identity as __nc_arg_2_len,
        identity as __nc_arg_2_ptr, identity as __nc_arg_2_values,
        identity as __nc_arg_2_written, identity as __nc_arg_0_0,
        identity as __nc_arg_0_0_owner_slot, identity as __nc_arg_0_0_owner_table,
        identity as __nc_arg_0_0_owner_original, identity as __nc_arg_0_0_offset,
        identity as __nc_arg_0_0_len, identity as __nc_arg_0_0_ptr,
        identity as __nc_arg_0_0_values, identity as __nc_arg_0_0_written,
        identity as __nc_arg_1_0, identity as __nc_arg_1_0_owner_slot,
        identity as __nc_arg_1_0_owner_table, identity as __nc_arg_1_0_owner_original,
        identity as __nc_arg_1_0_offset, identity as __nc_arg_1_0_len,
        identity as __nc_arg_1_0_ptr, identity as __nc_arg_1_0_values,
        identity as __nc_arg_1_0_written, identity as __nc_arg_2_0,
        identity as __nc_arg_2_0_owner_slot, identity as __nc_arg_2_0_owner_table,
        identity as __nc_arg_2_0_owner_original, identity as __nc_arg_2_0_offset,
        identity as __nc_arg_2_0_len, identity as __nc_arg_2_0_ptr,
        identity as __nc_arg_2_0_values, identity as __nc_arg_2_0_written,
        identity as __nc_this, identity as __nc_status, identity as __nc_value,
        identity as __nc_raw, identity as __nc_result, identity as __nc_values,
        identity as __nc_writer, identity as __nc_out, identity as __nc_scoped_context,
        identity as __nc_scoped_callback, identity as __nc_identity,
        identity as __nc_this_0, identity as __nc_status_0, identity as __nc_result_0,
        identity as __nc_prepared, identity as __nc_out_0, identity as __nc_this_1,
        identity as __nc_status_1, identity as __nc_result_1,
        identity as __nc_prepared_0, identity as __nc_out_1,
    };
    impl<
        __NcThreading: crate::ThreadingModel,
    > IPersistenceFactoryWrapper<__NcThreading> {
        /// Rejects unsupported revisions before any child object is constructed.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:26:8` (module `neocogi.com.persistence`).
        ///
        ///Calls [`IPersistenceFactory::validate`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the status-only raw vtable function.
        ///Call errors remain visible. Generated Rust thunks normalize non-negative `Result::Err` values to `NC_E_INVALID_ARG`.
        ///- `version` is passed by value as `:: core :: primitive :: u32`.
        ///- `state` is passed as a safe readable `:: core :: primitive :: u8` buffer plus offset.
        ///- `child_count` is passed by value as `:: core :: primitive :: u32`.
        ///
        ///On success, returns `()`.
        pub fn validate(
            &self,
            __nc_arg_0: ::core::primitive::u32,
            __nc_arg_1: crate::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
            __nc_arg_2: ::core::primitive::u32,
        ) -> ::core::result::Result<(), crate::NcStatus> {
            let __nc_arg_0 = crate::encode_inline(&__nc_arg_0)?;
            let __nc_arg_1 = crate::EncodedInput::new(__nc_arg_1)?;
            let __nc_arg_2 = crate::encode_inline(&__nc_arg_2)?;
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
        /// IDL source: `neocogi/com/persistence/objects.ncidl:29:8` (module `neocogi.com.persistence`).
        ///
        ///Calls [`IPersistenceFactory::restore`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Call errors remain visible. Generated Rust thunks normalize non-negative `Result::Err` values to `NC_E_INVALID_ARG`.
        ///- `version` is passed by value as `:: core :: primitive :: u32`.
        ///- `state` is passed as a safe readable `:: core :: primitive :: u8` buffer plus offset.
        ///- `children` is a required shared loan of the `self :: IObjectChildrenAbi` interface for the call.
        ///
        ///On success, returns `crate :: ComPtr < crate :: IUnknown >`. The caller receives one shared owner and releases it when finished.
        pub fn restore(
            &self,
            __nc_arg_0_0: ::core::primitive::u32,
            __nc_arg_1_0: crate::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
            __nc_arg_2_0: crate::ComRef<'_, self::IObjectChildrenAbi>,
        ) -> ::core::result::Result<crate::ComPtr<crate::IUnknown>, crate::NcStatus> {
            let __nc_arg_0_0 = crate::encode_inline(&__nc_arg_0_0)?;
            let __nc_arg_1_0 = crate::EncodedInput::new(__nc_arg_1_0)?;
            let mut __nc_out_1 = ::core::mem::MaybeUninit::<
                crate::ComRaw<crate::IUnknown>,
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
                <crate::ComPtr<
                    crate::IUnknown,
                >>::__nc_from_raw_owned_checked(__nc_out_1.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_1)
        }
    }
};
impl IPersistenceFactoryAbi {
    ///Canonical validation form of this interface's typed ABI contract.
    pub const ABI_SIGNATURE: crate::NcAbiDigest = crate::NcAbiDigest::from_static(
        b"\xC0\xA9\xE0\xCD\x15a$O\x17\xA7\xD6\x11\x10\x7F\x92>\n\xCD0\x85\n{>\xDD\x10\xD8\xC1\x12\xFF\xE6\xA1\xCC",
    );
    ///Static descriptors for this interface's ABI methods.
    pub const METHOD_DESCRIPTORS: [crate::NcMethodDescriptor; 2usize] = [
        crate::NcMethodDescriptor {
            name: crate::NcStaticUtf8::from_static("validate"),
            receiver: crate::NC_ABI_RECEIVER_SHARED,
            param_count: 3u32,
            params: (&[
                crate::NcMethodParamDescriptor {
                    name: crate::NcStaticUtf8::from_static("version"),
                    passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                        crate::NcAbiPassing::Value,
                    ),
                    ty: crate::NcAbiTypeDescriptor::from_abi_type(
                        crate::__nc_abi_type::<::core::primitive::u32>(),
                    ),
                },
                crate::NcMethodParamDescriptor {
                    name: crate::NcStaticUtf8::from_static("state"),
                    passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                        crate::NcAbiPassing::CopyFromBufferOffset,
                    ),
                    ty: crate::NcAbiTypeDescriptor::from_abi_type(
                        crate::__nc_abi_plain_type::<::core::primitive::u8>(),
                    ),
                },
                crate::NcMethodParamDescriptor {
                    name: crate::NcStaticUtf8::from_static("child_count"),
                    passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                        crate::NcAbiPassing::Value,
                    ),
                    ty: crate::NcAbiTypeDescriptor::from_abi_type(
                        crate::__nc_abi_type::<::core::primitive::u32>(),
                    ),
                },
            ] as &'static [crate::NcMethodParamDescriptor])
                .as_ptr(),
            returns: crate::NcMethodReturnDescriptor {
                passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                    crate::NcAbiPassing::Void,
                ),
                ty: crate::NcAbiTypeDescriptor::from_abi_type(crate::NcAbiType::Void),
            },
        },
        crate::NcMethodDescriptor {
            name: crate::NcStaticUtf8::from_static("restore"),
            receiver: crate::NC_ABI_RECEIVER_SHARED,
            param_count: 3u32,
            params: (&[
                crate::NcMethodParamDescriptor {
                    name: crate::NcStaticUtf8::from_static("version"),
                    passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                        crate::NcAbiPassing::Value,
                    ),
                    ty: crate::NcAbiTypeDescriptor::from_abi_type(
                        crate::__nc_abi_type::<::core::primitive::u32>(),
                    ),
                },
                crate::NcMethodParamDescriptor {
                    name: crate::NcStaticUtf8::from_static("state"),
                    passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                        crate::NcAbiPassing::CopyFromBufferOffset,
                    ),
                    ty: crate::NcAbiTypeDescriptor::from_abi_type(
                        crate::__nc_abi_plain_type::<::core::primitive::u8>(),
                    ),
                },
                crate::NcMethodParamDescriptor {
                    name: crate::NcStaticUtf8::from_static("children"),
                    passing: crate::NcAbiPassingDescriptor::from_abi_passing(crate::NcAbiPassing::ComRef {
                        mode: <crate::ThreadLocal as crate::ThreadingModel>::ABI_THREADING_MODE,
                    }),
                    ty: crate::NcAbiTypeDescriptor::from_interface(
                        <self::IObjectChildrenAbi as crate::ComInterface>::INTERFACE_ID,
                    ),
                },
            ] as &'static [crate::NcMethodParamDescriptor])
                .as_ptr(),
            returns: crate::NcMethodReturnDescriptor {
                passing: crate::NcAbiPassingDescriptor::from_abi_passing(crate::NcAbiPassing::ComPtrOut {
                    mode: <crate::ThreadLocal as crate::ThreadingModel>::ABI_THREADING_MODE,
                }),
                ty: crate::NcAbiTypeDescriptor::from_interface(
                    <crate::IUnknown as crate::ComInterface>::INTERFACE_ID,
                ),
            },
        },
    ];
    ///Static ABI descriptor for this interface.
    pub const INTERFACE_DESCRIPTOR: crate::NcInterfaceDescriptor = crate::NcInterfaceDescriptor {
        interface_id: <Self as crate::ComInterface>::IID,
        qualified_name: crate::NcStaticUtf8::from_static(
            "neocogi.com.persistence.IPersistenceFactory",
        ),
        abi_signature: Self::ABI_SIGNATURE,
        base: crate::NcInterfaceId::NONE,
        method_count: 2u32,
        methods: Self::METHOD_DESCRIPTORS.as_ptr(),
    };
    /// Distinct projections in derived-first, declared-parent order.
    /// Shared ancestors occur once; repeated identities must have the same ABI.
    pub const INTERFACE_CHAIN: [&'static crate::NcInterfaceDescriptor; crate::__nc_object_interface_count(
        &[&[&Self::INTERFACE_DESCRIPTOR]],
    )] = crate::__nc_interface_chain(&Self::INTERFACE_DESCRIPTOR, &[]);
}
impl IPersistenceFactoryAbi {
    #[doc(hidden)]
    pub const __NC_RUST_BINDING: [::core::primitive::u8; 32] = [
        106u8, 149u8, 77u8, 226u8, 122u8, 48u8, 171u8, 62u8, 4u8, 209u8, 206u8, 190u8,
        232u8, 102u8, 219u8, 143u8, 56u8, 38u8, 135u8, 234u8, 44u8, 87u8, 254u8, 105u8,
        133u8, 195u8, 203u8, 74u8, 233u8, 186u8, 54u8, 138u8,
    ];
}
