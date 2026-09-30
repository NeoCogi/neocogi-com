// Generated from core/idl/neocogi/com/persistence/objects.ncidl.
// Regenerate: NC_UPDATE_PERSISTENCE_BINDINGS=1 cargo test -p neocogi-com-macros persistence_bindings_match_idl

/// Stable logical identity of this generated contract module.
pub const MODULE_ID: crate::NcGuid = crate::NcGuid::from_fields(
    3545662286u32,
    20204u16,
    22946u16,
    [140u8, 82u8, 62u8, 53u8, 229u8, 28u8, 242u8, 16u8],
);
/// Transitive canonical ABI fingerprint of this generated module.
pub const ABI_FINGERPRINT: [u8; 32] = [
    152u8, 71u8, 223u8, 219u8, 28u8, 77u8, 174u8, 177u8, 106u8, 32u8, 131u8, 31u8, 232u8,
    5u8, 180u8, 53u8, 246u8, 230u8, 178u8, 165u8, 171u8, 92u8, 224u8, 120u8, 210u8,
    250u8, 22u8, 3u8, 115u8, 130u8, 172u8, 141u8,
];
/// Exact dotted module name derived from the normalized IDL path.
pub const MODULE_NAME: &str = "neocogi.com.persistence";
/// An ordered, owning list used only while constructing a persistent parent.
/// IDL source: `neocogi/com/persistence/objects.ncidl:3:11`.
pub trait IObjectChildren {
    /// Returns the number of fully initialized child objects.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:5:8`.
    fn count(&self) -> Result<u32, crate::NcStatus>;
    /// Returns an owned reference; indices must be less than count.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:7:8`.
    fn get(&self, index: u32) -> Result<crate::ComPtr<crate::IUnknown>, crate::NcStatus>;
}
/// Exact signature bytes assigned by the IDL compiler.
#[allow(non_upper_case_globals)]
static __nc_IObjectChildrenAbi_abi_signature_bytes: [u8; crate::NcAbiSignature::SHA256_LEN] = *b"\x85}\xF5^\x06\x08~@\x0C\xE7\x07\t\xFF\x17ZR\xAD\x98\xF4\xA8\x92JB\x0F_C\xC9c\xA5\0\xB3\x12";
#[doc = concat!(
    "Interface marker for [`", stringify!(IObjectChildren),
    "`]; handles carry its object/table pair."
)]
pub enum IObjectChildrenAbi {}
#[repr(C)]
#[doc = concat!("C-compatible vtable for [`", stringify!(IObjectChildren), "`].")]
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
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call; `Option<ComRef>` additionally permits both fields null for absence; half-null pairs are invalid.
    #[doc = concat!(
        "On success, `out` receives `", stringify!(< u32 as crate ::ComValue > ::Wire),
        "`."
    )]
    pub count: unsafe extern "C" fn(
        this: *mut core::ffi::c_void,
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
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call; `Option<ComRef>` additionally permits both fields null for absence; half-null pairs are invalid.
    #[doc = concat!(
        "- `", stringify!(index), "` is passed by value as `", stringify!(< u32 as crate
        ::ComValue > ::Wire), "`."
    )]
    #[doc = concat!(
        "On success, `out` receives an owned `", stringify!(crate ::IUnknown),
        "` object/table pair transferred to the caller."
    )]
    pub get: unsafe extern "C" fn(
        this: *mut core::ffi::c_void,
        index: <u32 as crate::ComValue>::Wire,
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
#[doc = concat!(
    "Borrowed static method view for [`", stringify!(IObjectChildren), "`]."
)]
pub struct IObjectChildrenClient<M: crate::ThreadingModel = crate::ThreadLocal> {
    /// Certified transparent storage supplied only by a live COM pointer borrow.
    __nc_pointer: crate::NcClientPointer<IObjectChildrenAbi, M>,
}
unsafe impl crate::ComClientInterface for IObjectChildrenAbi {
    type Client<M: crate::ThreadingModel> = IObjectChildrenClient<M>;
}
impl<M: crate::ThreadingModel> IObjectChildrenClient<M> {
    /// Returns the number of fully initialized child objects.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:5:8`.
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
    pub fn count(&self) -> Result<u32, crate::NcStatus> {
        let mut __nc_out = core::mem::MaybeUninit::<
            <u32 as crate::ComValue>::Wire,
        >::uninit();
        let this = self.__nc_pointer.as_raw();
        let status = unsafe {
            ((*this.vtable).count)(this.object, __nc_out.as_mut_ptr())
        };
        if status.is_err() {
            return Err(status);
        }
        let result = (unsafe { crate::decode_inline::<u32>(__nc_out.assume_init()) })?;
        Ok(result)
    }
    /// Returns an owned reference; indices must be less than count.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:7:8`.
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
        "` interface pointer returned through `out` as `", stringify!(crate ::ComPtr <
        crate ::IUnknown >), "`."
    )]
    pub fn get(
        &self,
        index: u32,
    ) -> Result<crate::ComPtr<crate::IUnknown>, crate::NcStatus> {
        let index = crate::encode_inline(&index)?;
        let mut __nc_out = core::mem::MaybeUninit::<
            crate::ComRaw<crate::IUnknown>,
        >::uninit();
        let this = self.__nc_pointer.as_raw();
        let status = unsafe {
            ((*this.vtable).get)(this.object, index, __nc_out.as_mut_ptr())
        };
        if status.is_err() {
            return Err(status);
        }
        let result = (unsafe {
            <crate::ComPtr<
                crate::IUnknown,
            >>::__nc_from_raw_owned_checked(__nc_out.assume_init())
        })?;
        Ok(result)
    }
}
impl IObjectChildrenAbi {
    ///Canonical validation form of this interface's typed ABI contract.
    pub const ABI_SIGNATURE: crate::NcAbiSignature = crate::NcAbiSignature::from_static(
        &__nc_IObjectChildrenAbi_abi_signature_bytes,
    );
    #[doc(hidden)]
    pub const __NC_METHOD_0_PARAMS: [crate::NcMethodParamDescriptor; 0usize] = [];
    #[doc(hidden)]
    pub const __NC_METHOD_0_RETURN: crate::NcMethodReturnDescriptor = crate::NcMethodReturnDescriptor {
        passing: crate::NcAbiPassingDescriptor::from_abi_passing(
            crate::NcAbiPassing::Out,
        ),
        ty: crate::NcAbiTypeDescriptor::from_abi_type(crate::__nc_abi_type::<u32>()),
    };
    #[doc(hidden)]
    pub const __NC_METHOD_1_PARAMS: [crate::NcMethodParamDescriptor; 1usize] = [
        crate::NcMethodParamDescriptor {
            name: crate::NcStaticUtf8::from_static(stringify!(index)),
            passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                crate::NcAbiPassing::Value,
            ),
            ty: crate::NcAbiTypeDescriptor::from_abi_type(crate::__nc_abi_type::<u32>()),
        },
    ];
    #[doc(hidden)]
    pub const __NC_METHOD_1_RETURN: crate::NcMethodReturnDescriptor = crate::NcMethodReturnDescriptor {
        passing: crate::NcAbiPassingDescriptor::from_abi_passing(crate::NcAbiPassing::ComPtrOut {
            mode: <crate::ThreadLocal as crate::ThreadingModel>::ABI_THREADING_MODE,
        }),
        ty: crate::NcAbiTypeDescriptor::from_interface(
            <crate::IUnknown as crate::ComInterface>::INTERFACE_ID,
        ),
    };
    ///Static descriptors for this interface's ABI methods.
    pub const METHOD_DESCRIPTORS: [crate::NcMethodDescriptor; 2usize] = [
        crate::NcMethodDescriptor {
            name: crate::NcStaticUtf8::from_static(stringify!(count)),
            receiver: crate::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: Self::__NC_METHOD_0_PARAMS.as_ptr(),
            returns: Self::__NC_METHOD_0_RETURN,
        },
        crate::NcMethodDescriptor {
            name: crate::NcStaticUtf8::from_static(stringify!(get)),
            receiver: crate::NC_ABI_RECEIVER_SHARED,
            param_count: 1u32,
            params: Self::__NC_METHOD_1_PARAMS.as_ptr(),
            returns: Self::__NC_METHOD_1_RETURN,
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
/// IDL source: `neocogi/com/persistence/objects.ncidl:12:11`.
pub trait IPersistable {
    /// Identifies the registered construction recipe, not this object instance.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:14:8`.
    fn type_id(&self) -> Result<crate::NcGuid, crate::NcStatus>;
    /// Identifies the persistent initialization schema, independently of COM ABI.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:16:8`.
    fn schema_version(&self) -> Result<u32, crate::NcStatus>;
    /// Exports initialization data only: no receivers, runtime pointers or caches.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:18:8`.
    fn save_state(&self) -> Result<crate::ComPtr<crate::INcBytesAbi>, crate::NcStatus>;
    /// Returns the number of owned persistent children; parent links are excluded.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:20:8`.
    fn child_count(&self) -> Result<u32, crate::NcStatus>;
    /// Returns one child in the order described by the parent's initialization.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:22:8`.
    fn child(
        &self,
        index: u32,
    ) -> Result<crate::ComPtr<crate::IUnknown>, crate::NcStatus>;
}
/// Exact signature bytes assigned by the IDL compiler.
#[allow(non_upper_case_globals)]
static __nc_IPersistableAbi_abi_signature_bytes: [u8; crate::NcAbiSignature::SHA256_LEN] = *b"\xB3[\x13\xD5\x02\xCD\x13\x91>\xA5\xF4\xB1\x08\xB2\xB8\xF5ma\xABj\xAB!\xD9\x06\xDE\xF1\xD3\xAC\xBF\xC4\xFD^";
#[doc = concat!(
    "Interface marker for [`", stringify!(IPersistable),
    "`]; handles carry its object/table pair."
)]
pub enum IPersistableAbi {}
#[repr(C)]
#[doc = concat!("C-compatible vtable for [`", stringify!(IPersistable), "`].")]
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
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call; `Option<ComRef>` additionally permits both fields null for absence; half-null pairs are invalid.
    #[doc = concat!(
        "On success, `out` receives `", stringify!(< crate ::NcGuid as crate ::ComValue >
        ::Wire), "`."
    )]
    pub type_id: unsafe extern "C" fn(
        this: *mut core::ffi::c_void,
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
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call; `Option<ComRef>` additionally permits both fields null for absence; half-null pairs are invalid.
    #[doc = concat!(
        "On success, `out` receives `", stringify!(< u32 as crate ::ComValue > ::Wire),
        "`."
    )]
    pub schema_version: unsafe extern "C" fn(
        this: *mut core::ffi::c_void,
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
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call; `Option<ComRef>` additionally permits both fields null for absence; half-null pairs are invalid.
    #[doc = concat!(
        "On success, `out` receives an owned `", stringify!(crate ::INcBytesAbi),
        "` object/table pair transferred to the caller."
    )]
    pub save_state: unsafe extern "C" fn(
        this: *mut core::ffi::c_void,
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
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call; `Option<ComRef>` additionally permits both fields null for absence; half-null pairs are invalid.
    #[doc = concat!(
        "On success, `out` receives `", stringify!(< u32 as crate ::ComValue > ::Wire),
        "`."
    )]
    pub child_count: unsafe extern "C" fn(
        this: *mut core::ffi::c_void,
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
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call; `Option<ComRef>` additionally permits both fields null for absence; half-null pairs are invalid.
    #[doc = concat!(
        "- `", stringify!(index), "` is passed by value as `", stringify!(< u32 as crate
        ::ComValue > ::Wire), "`."
    )]
    #[doc = concat!(
        "On success, `out` receives an owned `", stringify!(crate ::IUnknown),
        "` object/table pair transferred to the caller."
    )]
    pub child: unsafe extern "C" fn(
        this: *mut core::ffi::c_void,
        index: <u32 as crate::ComValue>::Wire,
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
#[doc = concat!("Borrowed static method view for [`", stringify!(IPersistable), "`].")]
pub struct IPersistableClient<M: crate::ThreadingModel = crate::ThreadLocal> {
    /// Certified transparent storage supplied only by a live COM pointer borrow.
    __nc_pointer: crate::NcClientPointer<IPersistableAbi, M>,
}
unsafe impl crate::ComClientInterface for IPersistableAbi {
    type Client<M: crate::ThreadingModel> = IPersistableClient<M>;
}
impl<M: crate::ThreadingModel> IPersistableClient<M> {
    /// Identifies the registered construction recipe, not this object instance.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:14:8`.
    ///
    #[doc = concat!(
        "Calls [`", stringify!(IPersistable), "::", stringify!(type_id),
        "`] through the generated COM ABI vtable."
    )]
    ///
    ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
    ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
    #[doc = concat!(
        "On success, returns `", stringify!(crate ::NcGuid), "` from the ABI out slot."
    )]
    pub fn type_id(&self) -> Result<crate::NcGuid, crate::NcStatus> {
        let mut __nc_out = core::mem::MaybeUninit::<
            <crate::NcGuid as crate::ComValue>::Wire,
        >::uninit();
        let this = self.__nc_pointer.as_raw();
        let status = unsafe {
            ((*this.vtable).type_id)(this.object, __nc_out.as_mut_ptr())
        };
        if status.is_err() {
            return Err(status);
        }
        let result = (unsafe {
            crate::decode_inline::<crate::NcGuid>(__nc_out.assume_init())
        })?;
        Ok(result)
    }
    /// Identifies the persistent initialization schema, independently of COM ABI.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:16:8`.
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
    pub fn schema_version(&self) -> Result<u32, crate::NcStatus> {
        let mut __nc_out = core::mem::MaybeUninit::<
            <u32 as crate::ComValue>::Wire,
        >::uninit();
        let this = self.__nc_pointer.as_raw();
        let status = unsafe {
            ((*this.vtable).schema_version)(this.object, __nc_out.as_mut_ptr())
        };
        if status.is_err() {
            return Err(status);
        }
        let result = (unsafe { crate::decode_inline::<u32>(__nc_out.assume_init()) })?;
        Ok(result)
    }
    /// Exports initialization data only: no receivers, runtime pointers or caches.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:18:8`.
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
        "` interface pointer returned through `out` as `", stringify!(crate ::ComPtr <
        crate ::INcBytesAbi >), "`."
    )]
    pub fn save_state(
        &self,
    ) -> Result<crate::ComPtr<crate::INcBytesAbi>, crate::NcStatus> {
        let mut __nc_out = core::mem::MaybeUninit::<
            crate::ComRaw<crate::INcBytesAbi>,
        >::uninit();
        let this = self.__nc_pointer.as_raw();
        let status = unsafe {
            ((*this.vtable).save_state)(this.object, __nc_out.as_mut_ptr())
        };
        if status.is_err() {
            return Err(status);
        }
        let result = (unsafe {
            <crate::ComPtr<
                crate::INcBytesAbi,
            >>::__nc_from_raw_owned_checked(__nc_out.assume_init())
        })?;
        Ok(result)
    }
    /// Returns the number of owned persistent children; parent links are excluded.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:20:8`.
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
    pub fn child_count(&self) -> Result<u32, crate::NcStatus> {
        let mut __nc_out = core::mem::MaybeUninit::<
            <u32 as crate::ComValue>::Wire,
        >::uninit();
        let this = self.__nc_pointer.as_raw();
        let status = unsafe {
            ((*this.vtable).child_count)(this.object, __nc_out.as_mut_ptr())
        };
        if status.is_err() {
            return Err(status);
        }
        let result = (unsafe { crate::decode_inline::<u32>(__nc_out.assume_init()) })?;
        Ok(result)
    }
    /// Returns one child in the order described by the parent's initialization.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:22:8`.
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
        "` interface pointer returned through `out` as `", stringify!(crate ::ComPtr <
        crate ::IUnknown >), "`."
    )]
    pub fn child(
        &self,
        index: u32,
    ) -> Result<crate::ComPtr<crate::IUnknown>, crate::NcStatus> {
        let index = crate::encode_inline(&index)?;
        let mut __nc_out = core::mem::MaybeUninit::<
            crate::ComRaw<crate::IUnknown>,
        >::uninit();
        let this = self.__nc_pointer.as_raw();
        let status = unsafe {
            ((*this.vtable).child)(this.object, index, __nc_out.as_mut_ptr())
        };
        if status.is_err() {
            return Err(status);
        }
        let result = (unsafe {
            <crate::ComPtr<
                crate::IUnknown,
            >>::__nc_from_raw_owned_checked(__nc_out.assume_init())
        })?;
        Ok(result)
    }
}
impl IPersistableAbi {
    ///Canonical validation form of this interface's typed ABI contract.
    pub const ABI_SIGNATURE: crate::NcAbiSignature = crate::NcAbiSignature::from_static(
        &__nc_IPersistableAbi_abi_signature_bytes,
    );
    #[doc(hidden)]
    pub const __NC_METHOD_0_PARAMS: [crate::NcMethodParamDescriptor; 0usize] = [];
    #[doc(hidden)]
    pub const __NC_METHOD_0_RETURN: crate::NcMethodReturnDescriptor = crate::NcMethodReturnDescriptor {
        passing: crate::NcAbiPassingDescriptor::from_abi_passing(
            crate::NcAbiPassing::Out,
        ),
        ty: crate::NcAbiTypeDescriptor::from_abi_type(
            crate::__nc_abi_type::<crate::NcGuid>(),
        ),
    };
    #[doc(hidden)]
    pub const __NC_METHOD_1_PARAMS: [crate::NcMethodParamDescriptor; 0usize] = [];
    #[doc(hidden)]
    pub const __NC_METHOD_1_RETURN: crate::NcMethodReturnDescriptor = crate::NcMethodReturnDescriptor {
        passing: crate::NcAbiPassingDescriptor::from_abi_passing(
            crate::NcAbiPassing::Out,
        ),
        ty: crate::NcAbiTypeDescriptor::from_abi_type(crate::__nc_abi_type::<u32>()),
    };
    #[doc(hidden)]
    pub const __NC_METHOD_2_PARAMS: [crate::NcMethodParamDescriptor; 0usize] = [];
    #[doc(hidden)]
    pub const __NC_METHOD_2_RETURN: crate::NcMethodReturnDescriptor = crate::NcMethodReturnDescriptor {
        passing: crate::NcAbiPassingDescriptor::from_abi_passing(crate::NcAbiPassing::ComPtrOut {
            mode: <crate::ThreadLocal as crate::ThreadingModel>::ABI_THREADING_MODE,
        }),
        ty: crate::NcAbiTypeDescriptor::from_interface(
            <crate::INcBytesAbi as crate::ComInterface>::INTERFACE_ID,
        ),
    };
    #[doc(hidden)]
    pub const __NC_METHOD_3_PARAMS: [crate::NcMethodParamDescriptor; 0usize] = [];
    #[doc(hidden)]
    pub const __NC_METHOD_3_RETURN: crate::NcMethodReturnDescriptor = crate::NcMethodReturnDescriptor {
        passing: crate::NcAbiPassingDescriptor::from_abi_passing(
            crate::NcAbiPassing::Out,
        ),
        ty: crate::NcAbiTypeDescriptor::from_abi_type(crate::__nc_abi_type::<u32>()),
    };
    #[doc(hidden)]
    pub const __NC_METHOD_4_PARAMS: [crate::NcMethodParamDescriptor; 1usize] = [
        crate::NcMethodParamDescriptor {
            name: crate::NcStaticUtf8::from_static(stringify!(index)),
            passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                crate::NcAbiPassing::Value,
            ),
            ty: crate::NcAbiTypeDescriptor::from_abi_type(crate::__nc_abi_type::<u32>()),
        },
    ];
    #[doc(hidden)]
    pub const __NC_METHOD_4_RETURN: crate::NcMethodReturnDescriptor = crate::NcMethodReturnDescriptor {
        passing: crate::NcAbiPassingDescriptor::from_abi_passing(crate::NcAbiPassing::ComPtrOut {
            mode: <crate::ThreadLocal as crate::ThreadingModel>::ABI_THREADING_MODE,
        }),
        ty: crate::NcAbiTypeDescriptor::from_interface(
            <crate::IUnknown as crate::ComInterface>::INTERFACE_ID,
        ),
    };
    ///Static descriptors for this interface's ABI methods.
    pub const METHOD_DESCRIPTORS: [crate::NcMethodDescriptor; 5usize] = [
        crate::NcMethodDescriptor {
            name: crate::NcStaticUtf8::from_static(stringify!(type_id)),
            receiver: crate::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: Self::__NC_METHOD_0_PARAMS.as_ptr(),
            returns: Self::__NC_METHOD_0_RETURN,
        },
        crate::NcMethodDescriptor {
            name: crate::NcStaticUtf8::from_static(stringify!(schema_version)),
            receiver: crate::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: Self::__NC_METHOD_1_PARAMS.as_ptr(),
            returns: Self::__NC_METHOD_1_RETURN,
        },
        crate::NcMethodDescriptor {
            name: crate::NcStaticUtf8::from_static(stringify!(save_state)),
            receiver: crate::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: Self::__NC_METHOD_2_PARAMS.as_ptr(),
            returns: Self::__NC_METHOD_2_RETURN,
        },
        crate::NcMethodDescriptor {
            name: crate::NcStaticUtf8::from_static(stringify!(child_count)),
            receiver: crate::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: Self::__NC_METHOD_3_PARAMS.as_ptr(),
            returns: Self::__NC_METHOD_3_RETURN,
        },
        crate::NcMethodDescriptor {
            name: crate::NcStaticUtf8::from_static(stringify!(child)),
            receiver: crate::NC_ABI_RECEIVER_SHARED,
            param_count: 1u32,
            params: Self::__NC_METHOD_4_PARAMS.as_ptr(),
            returns: Self::__NC_METHOD_4_RETURN,
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
/// IDL source: `neocogi/com/persistence/objects.ncidl:27:11`.
pub trait IPersistenceFactory {
    /// Rejects unsupported revisions before any child object is constructed.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:29:8`.
    fn validate(
        &self,
        version: u32,
        state: crate::NcCopyFromBufferOffset<'_, u8>,
        child_count: u32,
    ) -> Result<(), crate::NcStatus>;
    /// Returns a fully initialized object; every child is already constructed.
    /// Failure must release resources acquired by this call and publish no root.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:32:8`.
    fn restore(
        &self,
        version: u32,
        state: crate::NcCopyFromBufferOffset<'_, u8>,
        children: crate::ComRef<'_, IObjectChildrenAbi>,
    ) -> Result<crate::ComPtr<crate::IUnknown>, crate::NcStatus>;
}
/// Exact signature bytes assigned by the IDL compiler.
#[allow(non_upper_case_globals)]
static __nc_IPersistenceFactoryAbi_abi_signature_bytes: [u8; crate::NcAbiSignature::SHA256_LEN] = *b"\xB8\xE4\xBD\xAA\x01\xF7\x87\xD5\xAE\x80e\xE7d\\\xAD\x11\xA0=\x83\x8AX4\x90x\x13\x19\xF7\x04\xEAFj4";
#[doc = concat!(
    "Interface marker for [`", stringify!(IPersistenceFactory),
    "`]; handles carry its object/table pair."
)]
pub enum IPersistenceFactoryAbi {}
#[repr(C)]
#[doc = concat!("C-compatible vtable for [`", stringify!(IPersistenceFactory), "`].")]
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
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call; `Option<ComRef>` additionally permits both fields null for absence; half-null pairs are invalid.
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
        this: *mut core::ffi::c_void,
        version: <u32 as crate::ComValue>::Wire,
        __nc_state_offset: u64,
        __nc_state_len: u64,
        __nc_state_ptr: *const <u8 as crate::ComValue>::Wire,
        child_count: <u32 as crate::ComValue>::Wire,
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
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call; `Option<ComRef>` additionally permits both fields null for absence; half-null pairs are invalid.
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
        "- `", stringify!(children), "` is a borrowed `", stringify!(IObjectChildrenAbi),
        "` object/table pair.", " Both fields must be non-null.",
        " The callee must call `AddRef` before retaining a present interface after the call returns."
    )]
    #[doc = concat!(
        "On success, `out` receives an owned `", stringify!(crate ::IUnknown),
        "` object/table pair transferred to the caller."
    )]
    pub restore: unsafe extern "C" fn(
        this: *mut core::ffi::c_void,
        version: <u32 as crate::ComValue>::Wire,
        __nc_state_offset: u64,
        __nc_state_len: u64,
        __nc_state_ptr: *const <u8 as crate::ComValue>::Wire,
        children: crate::ComRaw<IObjectChildrenAbi>,
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
#[doc = concat!(
    "Borrowed static method view for [`", stringify!(IPersistenceFactory), "`]."
)]
pub struct IPersistenceFactoryClient<M: crate::ThreadingModel = crate::ThreadLocal> {
    /// Certified transparent storage supplied only by a live COM pointer borrow.
    __nc_pointer: crate::NcClientPointer<IPersistenceFactoryAbi, M>,
}
unsafe impl crate::ComClientInterface for IPersistenceFactoryAbi {
    type Client<M: crate::ThreadingModel> = IPersistenceFactoryClient<M>;
}
impl<M: crate::ThreadingModel> IPersistenceFactoryClient<M> {
    /// Rejects unsupported revisions before any child object is constructed.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:29:8`.
    ///
    #[doc = concat!(
        "Calls [`", stringify!(IPersistenceFactory), "::", stringify!(validate),
        "`] through the generated COM ABI vtable."
    )]
    ///
    ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
    ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
    #[doc = concat!(
        "- `", stringify!(version), "` is passed by value as `", stringify!(u32), "`."
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
        version: u32,
        state: crate::NcCopyFromBufferOffset<'_, u8>,
        child_count: u32,
    ) -> Result<(), crate::NcStatus> {
        let version = crate::encode_inline(&version)?;
        let state = crate::EncodedInput::new(state)?;
        let child_count = crate::encode_inline(&child_count)?;
        let this = self.__nc_pointer.as_raw();
        let status = unsafe {
            ((*this.vtable)
                .validate)(
                this.object,
                version,
                state.offset(),
                state.len(),
                state.as_ptr(),
                child_count,
            )
        };
        if status.is_err() {
            return Err(status);
        }
        let result = ();
        Ok(result)
    }
    /// Returns a fully initialized object; every child is already constructed.
    /// Failure must release resources acquired by this call and publish no root.
    /// IDL source: `neocogi/com/persistence/objects.ncidl:32:8`.
    ///
    #[doc = concat!(
        "Calls [`", stringify!(IPersistenceFactory), "::", stringify!(restore),
        "`] through the generated COM ABI vtable."
    )]
    ///
    ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
    ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
    #[doc = concat!(
        "- `", stringify!(version), "` is passed by value as `", stringify!(u32), "`."
    )]
    #[doc = concat!(
        "- `", stringify!(state), "` is passed as a safe readable `", stringify!(u8),
        "` buffer plus offset."
    )]
    #[doc = concat!(
        "- `", stringify!(children), "` borrows a `", stringify!(IObjectChildrenAbi),
        "` interface for the duration of the call.", " An interface is required.",
        " The callee must call `AddRef` before retaining a present interface."
    )]
    #[doc = concat!(
        "On success, adopts the owned `", stringify!(crate ::IUnknown),
        "` interface pointer returned through `out` as `", stringify!(crate ::ComPtr <
        crate ::IUnknown >), "`."
    )]
    pub fn restore(
        &self,
        version: u32,
        state: crate::NcCopyFromBufferOffset<'_, u8>,
        children: crate::ComRef<'_, IObjectChildrenAbi>,
    ) -> Result<crate::ComPtr<crate::IUnknown>, crate::NcStatus> {
        let version = crate::encode_inline(&version)?;
        let state = crate::EncodedInput::new(state)?;
        let mut __nc_out = core::mem::MaybeUninit::<
            crate::ComRaw<crate::IUnknown>,
        >::uninit();
        let this = self.__nc_pointer.as_raw();
        let status = unsafe {
            ((*this.vtable)
                .restore)(
                this.object,
                version,
                state.offset(),
                state.len(),
                state.as_ptr(),
                children.as_raw(),
                __nc_out.as_mut_ptr(),
            )
        };
        if status.is_err() {
            return Err(status);
        }
        let result = (unsafe {
            <crate::ComPtr<
                crate::IUnknown,
            >>::__nc_from_raw_owned_checked(__nc_out.assume_init())
        })?;
        Ok(result)
    }
}
impl IPersistenceFactoryAbi {
    ///Canonical validation form of this interface's typed ABI contract.
    pub const ABI_SIGNATURE: crate::NcAbiSignature = crate::NcAbiSignature::from_static(
        &__nc_IPersistenceFactoryAbi_abi_signature_bytes,
    );
    #[doc(hidden)]
    pub const __NC_METHOD_0_PARAMS: [crate::NcMethodParamDescriptor; 3usize] = [
        crate::NcMethodParamDescriptor {
            name: crate::NcStaticUtf8::from_static(stringify!(version)),
            passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                crate::NcAbiPassing::Value,
            ),
            ty: crate::NcAbiTypeDescriptor::from_abi_type(crate::__nc_abi_type::<u32>()),
        },
        crate::NcMethodParamDescriptor {
            name: crate::NcStaticUtf8::from_static(stringify!(state)),
            passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                crate::NcAbiPassing::CopyFromBufferOffset,
            ),
            ty: crate::NcAbiTypeDescriptor::from_abi_type(
                crate::__nc_abi_plain_type::<u8>(),
            ),
        },
        crate::NcMethodParamDescriptor {
            name: crate::NcStaticUtf8::from_static(stringify!(child_count)),
            passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                crate::NcAbiPassing::Value,
            ),
            ty: crate::NcAbiTypeDescriptor::from_abi_type(crate::__nc_abi_type::<u32>()),
        },
    ];
    #[doc(hidden)]
    pub const __NC_METHOD_0_RETURN: crate::NcMethodReturnDescriptor = crate::NcMethodReturnDescriptor {
        passing: crate::NcAbiPassingDescriptor::from_abi_passing(
            crate::NcAbiPassing::Void,
        ),
        ty: crate::NcAbiTypeDescriptor::from_abi_type(crate::NcAbiType::Void),
    };
    #[doc(hidden)]
    pub const __NC_METHOD_1_PARAMS: [crate::NcMethodParamDescriptor; 3usize] = [
        crate::NcMethodParamDescriptor {
            name: crate::NcStaticUtf8::from_static(stringify!(version)),
            passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                crate::NcAbiPassing::Value,
            ),
            ty: crate::NcAbiTypeDescriptor::from_abi_type(crate::__nc_abi_type::<u32>()),
        },
        crate::NcMethodParamDescriptor {
            name: crate::NcStaticUtf8::from_static(stringify!(state)),
            passing: crate::NcAbiPassingDescriptor::from_abi_passing(
                crate::NcAbiPassing::CopyFromBufferOffset,
            ),
            ty: crate::NcAbiTypeDescriptor::from_abi_type(
                crate::__nc_abi_plain_type::<u8>(),
            ),
        },
        crate::NcMethodParamDescriptor {
            name: crate::NcStaticUtf8::from_static(stringify!(children)),
            passing: crate::NcAbiPassingDescriptor::from_abi_passing(crate::NcAbiPassing::ComRef {
                mode: <crate::ThreadLocal as crate::ThreadingModel>::ABI_THREADING_MODE,
            }),
            ty: crate::NcAbiTypeDescriptor::from_interface(
                <IObjectChildrenAbi as crate::ComInterface>::INTERFACE_ID,
            ),
        },
    ];
    #[doc(hidden)]
    pub const __NC_METHOD_1_RETURN: crate::NcMethodReturnDescriptor = crate::NcMethodReturnDescriptor {
        passing: crate::NcAbiPassingDescriptor::from_abi_passing(crate::NcAbiPassing::ComPtrOut {
            mode: <crate::ThreadLocal as crate::ThreadingModel>::ABI_THREADING_MODE,
        }),
        ty: crate::NcAbiTypeDescriptor::from_interface(
            <crate::IUnknown as crate::ComInterface>::INTERFACE_ID,
        ),
    };
    ///Static descriptors for this interface's ABI methods.
    pub const METHOD_DESCRIPTORS: [crate::NcMethodDescriptor; 2usize] = [
        crate::NcMethodDescriptor {
            name: crate::NcStaticUtf8::from_static(stringify!(validate)),
            receiver: crate::NC_ABI_RECEIVER_SHARED,
            param_count: 3u32,
            params: Self::__NC_METHOD_0_PARAMS.as_ptr(),
            returns: Self::__NC_METHOD_0_RETURN,
        },
        crate::NcMethodDescriptor {
            name: crate::NcStaticUtf8::from_static(stringify!(restore)),
            receiver: crate::NC_ABI_RECEIVER_SHARED,
            param_count: 3u32,
            params: Self::__NC_METHOD_1_PARAMS.as_ptr(),
            returns: Self::__NC_METHOD_1_RETURN,
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
