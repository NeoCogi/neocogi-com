// Generated from core/idl/neocogi/com/persistence/objects.ncidl.
// Regenerate: NC_UPDATE_PERSISTENCE_BINDINGS=1 cargo test -p neocogi-idl persistence_bindings_match_idl

/// Stable logical identity of this generated contract module.
pub const MODULE_ID: crate::NcGuid = crate::NcGuid::from_fields(
    3545662286u32,
    20204u16,
    22946u16,
    [140u8, 82u8, 62u8, 53u8, 229u8, 28u8, 242u8, 16u8],
);
/// Transitive canonical ABI fingerprint of this generated module.
pub const ABI_FINGERPRINT: [u8; 32] = [
    104u8, 91u8, 140u8, 108u8, 106u8, 78u8, 177u8, 127u8, 34u8, 151u8, 216u8, 174u8,
    10u8, 125u8, 94u8, 46u8, 43u8, 247u8, 49u8, 109u8, 194u8, 220u8, 238u8, 3u8, 16u8,
    46u8, 3u8, 92u8, 48u8, 177u8, 173u8, 142u8,
];
/// Exact dotted module name derived from the normalized IDL path.
pub const MODULE_NAME: &str = "neocogi.com.persistence";
/// An ordered, owning list used only while constructing a persistent parent.
/// IDL source: `neocogi/com/persistence/objects.ncidl:3:11` (module `neocogi.com.persistence`).
pub trait IObjectChildren {
    /// Returns the number of fully initialized child objects.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:5:8` (module `neocogi.com.persistence`).
    fn count(&self) -> ::core::result::Result<u32, crate::NcStatus>;
    /// Returns an owned reference; indices must be less than count.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:7:8` (module `neocogi.com.persistence`).
    fn get(
        &self,
        index: u32,
    ) -> ::core::result::Result<crate::ComPtr<crate::IUnknown>, crate::NcStatus>;
}
#[doc = ::core::concat!(
    "Interface marker for [`", ::core::stringify!(IObjectChildren),
    "`]; handles carry its object/table pair."
)]
pub enum IObjectChildrenAbi {}
#[repr(C)]
#[doc = ::core::concat!(
    "C-compatible vtable for [`", ::core::stringify!(IObjectChildren), "`]."
)]
pub struct IObjectChildrenVTable {
    /// Common identity, query and reference-count slots.
    pub unknown: crate::IUnknownVTable,
    #[doc = concat!(
        "Raw COM ABI vtable slot for [`", stringify!(IObjectChildren), "::",
        stringify!(count), "`]."
    )]
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    #[doc = concat!(
        "On success, `out` receives `", stringify!(< u32 as crate ::ComValue > ::Wire),
        "`."
    )]
    pub count: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <u32 as crate::ComValue>::Wire,
    ) -> crate::NcStatus,
    #[doc = concat!(
        "Raw COM ABI vtable slot for [`", stringify!(IObjectChildren), "::",
        stringify!(get), "`]."
    )]
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    #[doc = concat!(
        "- `", stringify!(index), "` is passed by value as `", stringify!(< u32 as crate
        ::ComValue > ::Wire), "`."
    )]
    #[doc = concat!(
        "On success, `out` receives an owned `", stringify!(crate ::IUnknown),
        "` object/table pair transferred to the caller."
    )]
    pub get: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0: <u32 as crate::ComValue>::Wire,
        __nc_out: *mut crate::ComRaw<crate::IUnknown>,
    ) -> crate::NcStatus,
}
unsafe impl crate::ComInterface for IObjectChildrenAbi {
    const IID: crate::NcGuid = crate::NcGuid {
        data1: 3044267551u32,
        data2: 19810u16,
        data3: 22379u16,
        data4: [135u8, 249u8, 36u8, 136u8, 103u8, 45u8, 179u8, 255u8],
    };
    const INTERFACE_ID: crate::NcInterfaceId = crate::NcInterfaceId::new(
        crate::NcGuid {
            data1: 3044267551u32,
            data2: 19810u16,
            data3: 22379u16,
            data4: [135u8, 249u8, 36u8, 136u8, 103u8, 45u8, 179u8, 255u8],
        },
        IObjectChildrenAbi::ABI_SIGNATURE,
    );
    type VTable = IObjectChildrenVTable;
    type Threading = crate::ThreadLocal;
}
#[repr(transparent)]
#[doc = ::core::concat!(
    "Borrowed static method view for [`", ::core::stringify!(IObjectChildren), "`]."
)]
pub struct IObjectChildrenClient<
    __NcThreading: crate::ThreadingModel = crate::ThreadLocal,
> {
    /// Certified transparent storage supplied only by a live COM pointer borrow.
    __nc_pointer: crate::NcClientPointer<IObjectChildrenAbi, __NcThreading>,
}
unsafe impl crate::ComClientInterface for IObjectChildrenAbi {
    type Client<__NcThreading: crate::ThreadingModel> = IObjectChildrenClient<
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
    impl<__NcThreading: crate::ThreadingModel> IObjectChildrenClient<__NcThreading> {
        /// Returns the number of fully initialized child objects.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:5:8` (module `neocogi.com.persistence`).
        ///
        #[doc = concat!(
            "Calls [`", stringify!(IObjectChildren), "::", stringify!(count),
            "`] through the generated COM ABI vtable."
        )]
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        #[doc = concat!(
            "On success, returns `", stringify!(u32), "` from the ABI out slot."
        )]
        pub fn count(&self) -> ::core::result::Result<u32, crate::NcStatus> {
            let mut __nc_out_0 = ::core::mem::MaybeUninit::<
                <u32 as crate::ComValue>::Wire,
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
                crate::decode_inline::<u32>(__nc_out_0.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_0)
        }
        /// Returns an owned reference; indices must be less than count.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:7:8` (module `neocogi.com.persistence`).
        ///
        #[doc = concat!(
            "Calls [`", stringify!(IObjectChildren), "::", stringify!(get),
            "`] through the generated COM ABI vtable."
        )]
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        #[doc = concat!(
            "- `", stringify!(index), "` is passed by value as `", stringify!(u32), "`."
        )]
        #[doc = concat!(
            "On success, adopts the owned `", stringify!(crate ::IUnknown),
            "` interface pointer returned through `out` as `", stringify!(crate ::ComPtr
            < crate ::IUnknown >), "`."
        )]
        pub fn get(
            &self,
            __nc_arg_0: u32,
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
    pub const ABI_SIGNATURE: crate::NcAbiSignature = crate::NcAbiSignature::from_static(
        b"}S\xC8\xEB\xF2i\x81\xC9\xAD\xD9\xB8:\xFC\xC7&\x18U\xA9\xDC:\x18?\x102P^=]\xE6\\\xCA\xD9",
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
                    crate::__nc_abi_type::<u32>(),
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
                        crate::__nc_abi_type::<u32>(),
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
/// Optional snapshot capability for an owned construction tree.
/// IDL source: `neocogi/com/persistence/objects.ncidl:12:11` (module `neocogi.com.persistence`).
pub trait IPersistable {
    /// Identifies the registered construction recipe, not this object instance.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:14:8` (module `neocogi.com.persistence`).
    fn type_id(&self) -> ::core::result::Result<crate::NcGuid, crate::NcStatus>;
    /// Identifies the persistent initialization schema, independently of COM ABI.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:16:8` (module `neocogi.com.persistence`).
    fn schema_version(&self) -> ::core::result::Result<u32, crate::NcStatus>;
    /// Exports initialization data only: no receivers, runtime pointers or caches.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:18:8` (module `neocogi.com.persistence`).
    fn save_state(
        &self,
    ) -> ::core::result::Result<crate::ComPtr<crate::INcBytesAbi>, crate::NcStatus>;
    /// Returns the number of owned persistent children; parent links are excluded.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:20:8` (module `neocogi.com.persistence`).
    fn child_count(&self) -> ::core::result::Result<u32, crate::NcStatus>;
    /// Returns one child in the order described by the parent's initialization.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:22:8` (module `neocogi.com.persistence`).
    fn child(
        &self,
        index: u32,
    ) -> ::core::result::Result<crate::ComPtr<crate::IUnknown>, crate::NcStatus>;
}
#[doc = ::core::concat!(
    "Interface marker for [`", ::core::stringify!(IPersistable),
    "`]; handles carry its object/table pair."
)]
pub enum IPersistableAbi {}
#[repr(C)]
#[doc = ::core::concat!(
    "C-compatible vtable for [`", ::core::stringify!(IPersistable), "`]."
)]
pub struct IPersistableVTable {
    /// Common identity, query and reference-count slots.
    pub unknown: crate::IUnknownVTable,
    #[doc = concat!(
        "Raw COM ABI vtable slot for [`", stringify!(IPersistable), "::",
        stringify!(type_id), "`]."
    )]
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    #[doc = concat!(
        "On success, `out` receives `", stringify!(< crate ::NcGuid as crate ::ComValue >
        ::Wire), "`."
    )]
    pub type_id: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <crate::NcGuid as crate::ComValue>::Wire,
    ) -> crate::NcStatus,
    #[doc = concat!(
        "Raw COM ABI vtable slot for [`", stringify!(IPersistable), "::",
        stringify!(schema_version), "`]."
    )]
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    #[doc = concat!(
        "On success, `out` receives `", stringify!(< u32 as crate ::ComValue > ::Wire),
        "`."
    )]
    pub schema_version: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <u32 as crate::ComValue>::Wire,
    ) -> crate::NcStatus,
    #[doc = concat!(
        "Raw COM ABI vtable slot for [`", stringify!(IPersistable), "::",
        stringify!(save_state), "`]."
    )]
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    #[doc = concat!(
        "On success, `out` receives an owned `", stringify!(crate ::INcBytesAbi),
        "` object/table pair transferred to the caller."
    )]
    pub save_state: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut crate::ComRaw<crate::INcBytesAbi>,
    ) -> crate::NcStatus,
    #[doc = concat!(
        "Raw COM ABI vtable slot for [`", stringify!(IPersistable), "::",
        stringify!(child_count), "`]."
    )]
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    #[doc = concat!(
        "On success, `out` receives `", stringify!(< u32 as crate ::ComValue > ::Wire),
        "`."
    )]
    pub child_count: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <u32 as crate::ComValue>::Wire,
    ) -> crate::NcStatus,
    #[doc = concat!(
        "Raw COM ABI vtable slot for [`", stringify!(IPersistable), "::",
        stringify!(child), "`]."
    )]
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    #[doc = concat!(
        "- `", stringify!(index), "` is passed by value as `", stringify!(< u32 as crate
        ::ComValue > ::Wire), "`."
    )]
    #[doc = concat!(
        "On success, `out` receives an owned `", stringify!(crate ::IUnknown),
        "` object/table pair transferred to the caller."
    )]
    pub child: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0: <u32 as crate::ComValue>::Wire,
        __nc_out: *mut crate::ComRaw<crate::IUnknown>,
    ) -> crate::NcStatus,
}
unsafe impl crate::ComInterface for IPersistableAbi {
    const IID: crate::NcGuid = crate::NcGuid {
        data1: 3211114322u32,
        data2: 3431u16,
        data3: 21153u16,
        data4: [164u8, 211u8, 23u8, 32u8, 218u8, 170u8, 154u8, 0u8],
    };
    const INTERFACE_ID: crate::NcInterfaceId = crate::NcInterfaceId::new(
        crate::NcGuid {
            data1: 3211114322u32,
            data2: 3431u16,
            data3: 21153u16,
            data4: [164u8, 211u8, 23u8, 32u8, 218u8, 170u8, 154u8, 0u8],
        },
        IPersistableAbi::ABI_SIGNATURE,
    );
    type VTable = IPersistableVTable;
    type Threading = crate::ThreadLocal;
}
#[repr(transparent)]
#[doc = ::core::concat!(
    "Borrowed static method view for [`", ::core::stringify!(IPersistable), "`]."
)]
pub struct IPersistableClient<
    __NcThreading: crate::ThreadingModel = crate::ThreadLocal,
> {
    /// Certified transparent storage supplied only by a live COM pointer borrow.
    __nc_pointer: crate::NcClientPointer<IPersistableAbi, __NcThreading>,
}
unsafe impl crate::ComClientInterface for IPersistableAbi {
    type Client<__NcThreading: crate::ThreadingModel> = IPersistableClient<
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
    impl<__NcThreading: crate::ThreadingModel> IPersistableClient<__NcThreading> {
        /// Identifies the registered construction recipe, not this object instance.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:14:8` (module `neocogi.com.persistence`).
        ///
        #[doc = concat!(
            "Calls [`", stringify!(IPersistable), "::", stringify!(type_id),
            "`] through the generated COM ABI vtable."
        )]
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        #[doc = concat!(
            "On success, returns `", stringify!(crate ::NcGuid),
            "` from the ABI out slot."
        )]
        pub fn type_id(&self) -> ::core::result::Result<crate::NcGuid, crate::NcStatus> {
            let mut __nc_out_0 = ::core::mem::MaybeUninit::<
                <crate::NcGuid as crate::ComValue>::Wire,
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
                crate::decode_inline::<crate::NcGuid>(__nc_out_0.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_0)
        }
        /// Identifies the persistent initialization schema, independently of COM ABI.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:16:8` (module `neocogi.com.persistence`).
        ///
        #[doc = concat!(
            "Calls [`", stringify!(IPersistable), "::", stringify!(schema_version),
            "`] through the generated COM ABI vtable."
        )]
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        #[doc = concat!(
            "On success, returns `", stringify!(u32), "` from the ABI out slot."
        )]
        pub fn schema_version(&self) -> ::core::result::Result<u32, crate::NcStatus> {
            let mut __nc_out_1 = ::core::mem::MaybeUninit::<
                <u32 as crate::ComValue>::Wire,
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
                crate::decode_inline::<u32>(__nc_out_1.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_1)
        }
        /// Exports initialization data only: no receivers, runtime pointers or caches.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:18:8` (module `neocogi.com.persistence`).
        ///
        #[doc = concat!(
            "Calls [`", stringify!(IPersistable), "::", stringify!(save_state),
            "`] through the generated COM ABI vtable."
        )]
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        #[doc = concat!(
            "On success, adopts the owned `", stringify!(crate ::INcBytesAbi),
            "` interface pointer returned through `out` as `", stringify!(crate ::ComPtr
            < crate ::INcBytesAbi >), "`."
        )]
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
        /// IDL source: `neocogi/com/persistence/objects.ncidl:20:8` (module `neocogi.com.persistence`).
        ///
        #[doc = concat!(
            "Calls [`", stringify!(IPersistable), "::", stringify!(child_count),
            "`] through the generated COM ABI vtable."
        )]
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        #[doc = concat!(
            "On success, returns `", stringify!(u32), "` from the ABI out slot."
        )]
        pub fn child_count(&self) -> ::core::result::Result<u32, crate::NcStatus> {
            let mut __nc_out_3 = ::core::mem::MaybeUninit::<
                <u32 as crate::ComValue>::Wire,
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
                crate::decode_inline::<u32>(__nc_out_3.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_3)
        }
        /// Returns one child in the order described by the parent's initialization.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:22:8` (module `neocogi.com.persistence`).
        ///
        #[doc = concat!(
            "Calls [`", stringify!(IPersistable), "::", stringify!(child),
            "`] through the generated COM ABI vtable."
        )]
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        #[doc = concat!(
            "- `", stringify!(index), "` is passed by value as `", stringify!(u32), "`."
        )]
        #[doc = concat!(
            "On success, adopts the owned `", stringify!(crate ::IUnknown),
            "` interface pointer returned through `out` as `", stringify!(crate ::ComPtr
            < crate ::IUnknown >), "`."
        )]
        pub fn child(
            &self,
            __nc_arg_0: u32,
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
    pub const ABI_SIGNATURE: crate::NcAbiSignature = crate::NcAbiSignature::from_static(
        b"\x95\xEDx\x0Ft\xDFD4\xEF\xCB\xC8D\x93f`\xCF`-\x90\xC8*yA\x88\xB1Zk!*W\x8F\xDB",
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
                    crate::__nc_abi_type::<u32>(),
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
                    crate::__nc_abi_type::<u32>(),
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
                        crate::__nc_abi_type::<u32>(),
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
/// A provider bound to the runtime services required by its construction recipe.
/// IDL source: `neocogi/com/persistence/objects.ncidl:27:11` (module `neocogi.com.persistence`).
pub trait IPersistenceFactory {
    /// Rejects unsupported revisions before any child object is constructed.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:29:8` (module `neocogi.com.persistence`).
    fn validate(
        &self,
        version: u32,
        state: crate::NcCopyFromBufferOffset<'_, u8>,
        child_count: u32,
    ) -> ::core::result::Result<(), crate::NcStatus>;
    /// Returns a fully initialized object; every child is already constructed.
    /// Failure must release resources acquired by this call and publish no root.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:32:8` (module `neocogi.com.persistence`).
    fn restore(
        &self,
        version: u32,
        state: crate::NcCopyFromBufferOffset<'_, u8>,
        children: crate::ComRef<'_, self::IObjectChildrenAbi>,
    ) -> ::core::result::Result<crate::ComPtr<crate::IUnknown>, crate::NcStatus>;
}
#[doc = ::core::concat!(
    "Interface marker for [`", ::core::stringify!(IPersistenceFactory),
    "`]; handles carry its object/table pair."
)]
pub enum IPersistenceFactoryAbi {}
#[repr(C)]
#[doc = ::core::concat!(
    "C-compatible vtable for [`", ::core::stringify!(IPersistenceFactory), "`]."
)]
pub struct IPersistenceFactoryVTable {
    /// Common identity, query and reference-count slots.
    pub unknown: crate::IUnknownVTable,
    #[doc = concat!(
        "Raw COM ABI vtable slot for [`", stringify!(IPersistenceFactory), "::",
        stringify!(validate), "`]."
    )]
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    #[doc = concat!(
        "- `", stringify!(version), "` is passed by value as `", stringify!(< u32 as
        crate ::ComValue > ::Wire), "`."
    )]
    #[doc = concat!(
        "- `", stringify!(state),
        "` lowers to `offset: u64`, `len: u64`, and a readable `*const ", stringify!(u8),
        "` buffer."
    )]
    #[doc = concat!(
        "- `", stringify!(child_count), "` is passed by value as `", stringify!(< u32 as
        crate ::ComValue > ::Wire), "`."
    )]
    ///On success, no output value is written.
    pub validate: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0: <u32 as crate::ComValue>::Wire,
        __nc_arg_1_offset: u64,
        __nc_arg_1_len: u64,
        __nc_arg_1_ptr: *const <u8 as crate::ComValue>::Wire,
        __nc_arg_2: <u32 as crate::ComValue>::Wire,
    ) -> crate::NcStatus,
    #[doc = concat!(
        "Raw COM ABI vtable slot for [`", stringify!(IPersistenceFactory), "::",
        stringify!(restore), "`]."
    )]
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    #[doc = concat!(
        "- `", stringify!(version), "` is passed by value as `", stringify!(< u32 as
        crate ::ComValue > ::Wire), "`."
    )]
    #[doc = concat!(
        "- `", stringify!(state),
        "` lowers to `offset: u64`, `len: u64`, and a readable `*const ", stringify!(u8),
        "` buffer."
    )]
    #[doc = concat!(
        "- `", stringify!(children), "` is a borrowed `",
        stringify!(self::IObjectChildrenAbi), "` object/table pair.",
        " Both fields must be non-null. The callee must call `AddRef` before retaining a present interface after the call returns."
    )]
    #[doc = concat!(
        "On success, `out` receives an owned `", stringify!(crate ::IUnknown),
        "` object/table pair transferred to the caller."
    )]
    pub restore: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_0: <u32 as crate::ComValue>::Wire,
        __nc_arg_1_0_offset: u64,
        __nc_arg_1_0_len: u64,
        __nc_arg_1_0_ptr: *const <u8 as crate::ComValue>::Wire,
        __nc_arg_2_0: crate::ComRaw<self::IObjectChildrenAbi>,
        __nc_out: *mut crate::ComRaw<crate::IUnknown>,
    ) -> crate::NcStatus,
}
unsafe impl crate::ComInterface for IPersistenceFactoryAbi {
    const IID: crate::NcGuid = crate::NcGuid {
        data1: 1927987790u32,
        data2: 295u16,
        data3: 23997u16,
        data4: [164u8, 199u8, 47u8, 172u8, 222u8, 225u8, 180u8, 131u8],
    };
    const INTERFACE_ID: crate::NcInterfaceId = crate::NcInterfaceId::new(
        crate::NcGuid {
            data1: 1927987790u32,
            data2: 295u16,
            data3: 23997u16,
            data4: [164u8, 199u8, 47u8, 172u8, 222u8, 225u8, 180u8, 131u8],
        },
        IPersistenceFactoryAbi::ABI_SIGNATURE,
    );
    type VTable = IPersistenceFactoryVTable;
    type Threading = crate::ThreadLocal;
}
#[repr(transparent)]
#[doc = ::core::concat!(
    "Borrowed static method view for [`", ::core::stringify!(IPersistenceFactory), "`]."
)]
pub struct IPersistenceFactoryClient<
    __NcThreading: crate::ThreadingModel = crate::ThreadLocal,
> {
    /// Certified transparent storage supplied only by a live COM pointer borrow.
    __nc_pointer: crate::NcClientPointer<IPersistenceFactoryAbi, __NcThreading>,
}
unsafe impl crate::ComClientInterface for IPersistenceFactoryAbi {
    type Client<__NcThreading: crate::ThreadingModel> = IPersistenceFactoryClient<
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
    impl<__NcThreading: crate::ThreadingModel> IPersistenceFactoryClient<__NcThreading> {
        /// Rejects unsupported revisions before any child object is constructed.
        /// IDL source: `neocogi/com/persistence/objects.ncidl:29:8` (module `neocogi.com.persistence`).
        ///
        #[doc = concat!(
            "Calls [`", stringify!(IPersistenceFactory), "::", stringify!(validate),
            "`] through the generated COM ABI vtable."
        )]
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        #[doc = concat!(
            "- `", stringify!(version), "` is passed by value as `", stringify!(u32),
            "`."
        )]
        #[doc = concat!(
            "- `", stringify!(state), "` is passed as a safe readable `", stringify!(u8),
            "` buffer plus offset."
        )]
        #[doc = concat!(
            "- `", stringify!(child_count), "` is passed by value as `", stringify!(u32),
            "`."
        )]
        ///On success, returns `()`.
        pub fn validate(
            &self,
            __nc_arg_0: u32,
            __nc_arg_1: crate::NcCopyFromBufferOffset<'_, u8>,
            __nc_arg_2: u32,
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
        /// IDL source: `neocogi/com/persistence/objects.ncidl:32:8` (module `neocogi.com.persistence`).
        ///
        #[doc = concat!(
            "Calls [`", stringify!(IPersistenceFactory), "::", stringify!(restore),
            "`] through the generated COM ABI vtable."
        )]
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        #[doc = concat!(
            "- `", stringify!(version), "` is passed by value as `", stringify!(u32),
            "`."
        )]
        #[doc = concat!(
            "- `", stringify!(state), "` is passed as a safe readable `", stringify!(u8),
            "` buffer plus offset."
        )]
        #[doc = concat!(
            "- `", stringify!(children), "` borrows a `",
            stringify!(self::IObjectChildrenAbi),
            "` interface for the duration of the call.",
            " An interface is required. The callee must call `AddRef` before retaining a present interface."
        )]
        #[doc = concat!(
            "On success, adopts the owned `", stringify!(crate ::IUnknown),
            "` interface pointer returned through `out` as `", stringify!(crate ::ComPtr
            < crate ::IUnknown >), "`."
        )]
        pub fn restore(
            &self,
            __nc_arg_0_0: u32,
            __nc_arg_1_0: crate::NcCopyFromBufferOffset<'_, u8>,
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
    pub const ABI_SIGNATURE: crate::NcAbiSignature = crate::NcAbiSignature::from_static(
        b"\r{w\"1]\x134\xECk\xAC8]:f\x91\xEF\x0EI\xDAdHkb\xF7C\xF9\xFF\xD6\xCE \xB8",
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
                        crate::__nc_abi_type::<u32>(),
                    ),
                },
                crate::NcMethodParamDescriptor {
                    name: crate::NcStaticUtf8::from_static("state"),
                    passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                        crate::NcAbiPassing::CopyFromBufferOffset,
                    ),
                    ty: crate::NcAbiTypeDescriptor::from_abi_type(
                        crate::__nc_abi_plain_type::<u8>(),
                    ),
                },
                crate::NcMethodParamDescriptor {
                    name: crate::NcStaticUtf8::from_static("child_count"),
                    passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                        crate::NcAbiPassing::Value,
                    ),
                    ty: crate::NcAbiTypeDescriptor::from_abi_type(
                        crate::__nc_abi_type::<u32>(),
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
                        crate::__nc_abi_type::<u32>(),
                    ),
                },
                crate::NcMethodParamDescriptor {
                    name: crate::NcStaticUtf8::from_static("state"),
                    passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                        crate::NcAbiPassing::CopyFromBufferOffset,
                    ),
                    ty: crate::NcAbiTypeDescriptor::from_abi_type(
                        crate::__nc_abi_plain_type::<u8>(),
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
