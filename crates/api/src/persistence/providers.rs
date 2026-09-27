// Generated from core/idl/neocogi/com/persistence/objects.ncidl.
// Regenerate: NC_UPDATE_PERSISTENCE_BINDINGS=1 cargo test -p neocogi-com-macros persistence_bindings_match_idl

use super::*;
#[doc(hidden)]
/// Statically typed vtable and lifecycle adapters for one provider type.
pub struct IObjectChildrenVtblFor<T>(
    /// Associates dispatch with T without allocating or owning a value.
    core::marker::PhantomData<T>,
);
impl<T> IObjectChildrenVtblFor<T>
where
    T: IObjectChildren + ::neocogi_com::ComAbiObject,
{
    /// Distinct provider bindings in the interface's canonical metadata order.
    /// T fixes the thunk implementation; no object headers are stored here.
    pub const PROJECTIONS: [::neocogi_com::__ComProjection<
        T,
    >; IObjectChildrenAbi::INTERFACE_CHAIN.len()] = ::neocogi_com::__nc_projection_table(
        &IObjectChildrenAbi::INTERFACE_CHAIN,
        unsafe {
            ::neocogi_com::__ComProjection::new(
                <IObjectChildrenAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
                &Self::VTABLE as *const IObjectChildrenVTable
                    as *const ::neocogi_com::IUnknownVTable,
            )
        },
        &[],
    );
    /// Table for a standalone projection of this interface.
    pub const VTABLE: IObjectChildrenVTable = Self::with_identity(
        &<IObjectChildrenAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
    );
    /// Builds an inherited prefix advertising the actual outer projection.
    pub const fn with_identity(
        identity: &'static ::neocogi_com::NcInterfaceId,
    ) -> IObjectChildrenVTable {
        IObjectChildrenVTable {
            unknown: ::neocogi_com::IUnknownVTable {
                interface_id: identity,
                query_interface: <T as ::neocogi_com::ComAbiObject>::query_interface_from_abi_object,
                add_ref: <T as ::neocogi_com::ComAbiObject>::add_ref_from_abi_object,
                release: <T as ::neocogi_com::ComAbiObject>::release_from_abi_object,
            },
            count: Self::count_thunk,
            get: Self::get_thunk,
        }
    }
    /// Validates raw pointers, borrows the concrete provider through its
    /// object-wide gate, invokes its typed method, and lowers the result.
    /// The gate is released before transferring an owned result; errors
    /// are normalized so an unwritten output can never look successful.
    ///
    /// # Safety
    /// The interface belongs to T's live generated storage. Non-null ABI
    /// arguments must satisfy their method's readable/writable contracts.
    unsafe extern "C" fn count_thunk(
        this: *mut core::ffi::c_void,
        __nc_out: *mut <u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() || __nc_out.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = this;
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let result = <T as IObjectChildren>::count(&*__nc_dispatch_value);
        drop(__nc_dispatch_value);
        if result.is_ok() {}
        match result {
            Ok(value) => {
                let raw = match ::neocogi_com::encode_inline(&value) {
                    Ok(raw) => raw,
                    Err(status) => return status,
                };
                unsafe {
                    *__nc_out = raw;
                }
                ::neocogi_com::NC_OK
            }
            Err(status) => status.normalize_error(),
        }
    }
    /// Validates raw pointers, borrows the concrete provider through its
    /// object-wide gate, invokes its typed method, and lowers the result.
    /// The gate is released before transferring an owned result; errors
    /// are normalized so an unwritten output can never look successful.
    ///
    /// # Safety
    /// The interface belongs to T's live generated storage. Non-null ABI
    /// arguments must satisfy their method's readable/writable contracts.
    unsafe extern "C" fn get_thunk(
        this: *mut core::ffi::c_void,
        index: <u32 as ::neocogi_com::ComValue>::Wire,
        __nc_out: *mut ::neocogi_com::ComRaw<::neocogi_com::IUnknown>,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() || __nc_out.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = this;
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let index = match unsafe { ::neocogi_com::decode_inline::<u32>(index) } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let result = <T as IObjectChildren>::get(&*__nc_dispatch_value, index);
        drop(__nc_dispatch_value);
        if result.is_ok() {}
        match result {
            Ok(value) => {
                unsafe {
                    *__nc_out = value.into_raw();
                }
                ::neocogi_com::NC_OK
            }
            Err(status) => status.normalize_error(),
        }
    }
}
#[doc(hidden)]
/// Statically typed vtable and lifecycle adapters for one provider type.
pub struct IPersistableVtblFor<T>(
    /// Associates dispatch with T without allocating or owning a value.
    core::marker::PhantomData<T>,
);
impl<T> IPersistableVtblFor<T>
where
    T: IPersistable + ::neocogi_com::ComAbiObject,
{
    /// Distinct provider bindings in the interface's canonical metadata order.
    /// T fixes the thunk implementation; no object headers are stored here.
    pub const PROJECTIONS: [::neocogi_com::__ComProjection<
        T,
    >; IPersistableAbi::INTERFACE_CHAIN.len()] = ::neocogi_com::__nc_projection_table(
        &IPersistableAbi::INTERFACE_CHAIN,
        unsafe {
            ::neocogi_com::__ComProjection::new(
                <IPersistableAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
                &Self::VTABLE as *const IPersistableVTable
                    as *const ::neocogi_com::IUnknownVTable,
            )
        },
        &[],
    );
    /// Table for a standalone projection of this interface.
    pub const VTABLE: IPersistableVTable = Self::with_identity(
        &<IPersistableAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
    );
    /// Builds an inherited prefix advertising the actual outer projection.
    pub const fn with_identity(
        identity: &'static ::neocogi_com::NcInterfaceId,
    ) -> IPersistableVTable {
        IPersistableVTable {
            unknown: ::neocogi_com::IUnknownVTable {
                interface_id: identity,
                query_interface: <T as ::neocogi_com::ComAbiObject>::query_interface_from_abi_object,
                add_ref: <T as ::neocogi_com::ComAbiObject>::add_ref_from_abi_object,
                release: <T as ::neocogi_com::ComAbiObject>::release_from_abi_object,
            },
            type_id: Self::type_id_thunk,
            schema_version: Self::schema_version_thunk,
            save_state: Self::save_state_thunk,
            child_count: Self::child_count_thunk,
            child: Self::child_thunk,
        }
    }
    /// Validates raw pointers, borrows the concrete provider through its
    /// object-wide gate, invokes its typed method, and lowers the result.
    /// The gate is released before transferring an owned result; errors
    /// are normalized so an unwritten output can never look successful.
    ///
    /// # Safety
    /// The interface belongs to T's live generated storage. Non-null ABI
    /// arguments must satisfy their method's readable/writable contracts.
    unsafe extern "C" fn type_id_thunk(
        this: *mut core::ffi::c_void,
        __nc_out: *mut <::neocogi_com::NcGuid as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() || __nc_out.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = this;
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let result = <T as IPersistable>::type_id(&*__nc_dispatch_value);
        drop(__nc_dispatch_value);
        if result.is_ok() {}
        match result {
            Ok(value) => {
                let raw = match ::neocogi_com::encode_inline(&value) {
                    Ok(raw) => raw,
                    Err(status) => return status,
                };
                unsafe {
                    *__nc_out = raw;
                }
                ::neocogi_com::NC_OK
            }
            Err(status) => status.normalize_error(),
        }
    }
    /// Validates raw pointers, borrows the concrete provider through its
    /// object-wide gate, invokes its typed method, and lowers the result.
    /// The gate is released before transferring an owned result; errors
    /// are normalized so an unwritten output can never look successful.
    ///
    /// # Safety
    /// The interface belongs to T's live generated storage. Non-null ABI
    /// arguments must satisfy their method's readable/writable contracts.
    unsafe extern "C" fn schema_version_thunk(
        this: *mut core::ffi::c_void,
        __nc_out: *mut <u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() || __nc_out.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = this;
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let result = <T as IPersistable>::schema_version(&*__nc_dispatch_value);
        drop(__nc_dispatch_value);
        if result.is_ok() {}
        match result {
            Ok(value) => {
                let raw = match ::neocogi_com::encode_inline(&value) {
                    Ok(raw) => raw,
                    Err(status) => return status,
                };
                unsafe {
                    *__nc_out = raw;
                }
                ::neocogi_com::NC_OK
            }
            Err(status) => status.normalize_error(),
        }
    }
    /// Validates raw pointers, borrows the concrete provider through its
    /// object-wide gate, invokes its typed method, and lowers the result.
    /// The gate is released before transferring an owned result; errors
    /// are normalized so an unwritten output can never look successful.
    ///
    /// # Safety
    /// The interface belongs to T's live generated storage. Non-null ABI
    /// arguments must satisfy their method's readable/writable contracts.
    unsafe extern "C" fn save_state_thunk(
        this: *mut core::ffi::c_void,
        __nc_out: *mut ::neocogi_com::ComRaw<::neocogi_com::INcBytesAbi>,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() || __nc_out.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = this;
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let result = <T as IPersistable>::save_state(&*__nc_dispatch_value);
        drop(__nc_dispatch_value);
        if result.is_ok() {}
        match result {
            Ok(value) => {
                unsafe {
                    *__nc_out = value.into_raw();
                }
                ::neocogi_com::NC_OK
            }
            Err(status) => status.normalize_error(),
        }
    }
    /// Validates raw pointers, borrows the concrete provider through its
    /// object-wide gate, invokes its typed method, and lowers the result.
    /// The gate is released before transferring an owned result; errors
    /// are normalized so an unwritten output can never look successful.
    ///
    /// # Safety
    /// The interface belongs to T's live generated storage. Non-null ABI
    /// arguments must satisfy their method's readable/writable contracts.
    unsafe extern "C" fn child_count_thunk(
        this: *mut core::ffi::c_void,
        __nc_out: *mut <u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() || __nc_out.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = this;
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let result = <T as IPersistable>::child_count(&*__nc_dispatch_value);
        drop(__nc_dispatch_value);
        if result.is_ok() {}
        match result {
            Ok(value) => {
                let raw = match ::neocogi_com::encode_inline(&value) {
                    Ok(raw) => raw,
                    Err(status) => return status,
                };
                unsafe {
                    *__nc_out = raw;
                }
                ::neocogi_com::NC_OK
            }
            Err(status) => status.normalize_error(),
        }
    }
    /// Validates raw pointers, borrows the concrete provider through its
    /// object-wide gate, invokes its typed method, and lowers the result.
    /// The gate is released before transferring an owned result; errors
    /// are normalized so an unwritten output can never look successful.
    ///
    /// # Safety
    /// The interface belongs to T's live generated storage. Non-null ABI
    /// arguments must satisfy their method's readable/writable contracts.
    unsafe extern "C" fn child_thunk(
        this: *mut core::ffi::c_void,
        index: <u32 as ::neocogi_com::ComValue>::Wire,
        __nc_out: *mut ::neocogi_com::ComRaw<::neocogi_com::IUnknown>,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() || __nc_out.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = this;
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let index = match unsafe { ::neocogi_com::decode_inline::<u32>(index) } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let result = <T as IPersistable>::child(&*__nc_dispatch_value, index);
        drop(__nc_dispatch_value);
        if result.is_ok() {}
        match result {
            Ok(value) => {
                unsafe {
                    *__nc_out = value.into_raw();
                }
                ::neocogi_com::NC_OK
            }
            Err(status) => status.normalize_error(),
        }
    }
}
#[doc(hidden)]
/// Statically typed vtable and lifecycle adapters for one provider type.
pub struct IPersistenceFactoryVtblFor<T>(
    /// Associates dispatch with T without allocating or owning a value.
    core::marker::PhantomData<T>,
);
impl<T> IPersistenceFactoryVtblFor<T>
where
    T: IPersistenceFactory + ::neocogi_com::ComAbiObject,
{
    /// Distinct provider bindings in the interface's canonical metadata order.
    /// T fixes the thunk implementation; no object headers are stored here.
    pub const PROJECTIONS: [::neocogi_com::__ComProjection<
        T,
    >; IPersistenceFactoryAbi::INTERFACE_CHAIN.len()] = ::neocogi_com::__nc_projection_table(
        &IPersistenceFactoryAbi::INTERFACE_CHAIN,
        unsafe {
            ::neocogi_com::__ComProjection::new(
                <IPersistenceFactoryAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
                &Self::VTABLE as *const IPersistenceFactoryVTable
                    as *const ::neocogi_com::IUnknownVTable,
            )
        },
        &[],
    );
    /// Table for a standalone projection of this interface.
    pub const VTABLE: IPersistenceFactoryVTable = Self::with_identity(
        &<IPersistenceFactoryAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
    );
    /// Builds an inherited prefix advertising the actual outer projection.
    pub const fn with_identity(
        identity: &'static ::neocogi_com::NcInterfaceId,
    ) -> IPersistenceFactoryVTable {
        IPersistenceFactoryVTable {
            unknown: ::neocogi_com::IUnknownVTable {
                interface_id: identity,
                query_interface: <T as ::neocogi_com::ComAbiObject>::query_interface_from_abi_object,
                add_ref: <T as ::neocogi_com::ComAbiObject>::add_ref_from_abi_object,
                release: <T as ::neocogi_com::ComAbiObject>::release_from_abi_object,
            },
            validate: Self::validate_thunk,
            restore: Self::restore_thunk,
        }
    }
    /// Validates raw pointers, borrows the concrete provider through its
    /// object-wide gate, invokes its typed method, and lowers the result.
    /// The gate is released before transferring an owned result; errors
    /// are normalized so an unwritten output can never look successful.
    ///
    /// # Safety
    /// The interface belongs to T's live generated storage. Non-null ABI
    /// arguments must satisfy their method's readable/writable contracts.
    unsafe extern "C" fn validate_thunk(
        this: *mut core::ffi::c_void,
        version: <u32 as ::neocogi_com::ComValue>::Wire,
        __nc_state_offset: u64,
        __nc_state_len: u64,
        __nc_state_ptr: *const <u8 as ::neocogi_com::ComValue>::Wire,
        child_count: <u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = this;
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let version = match unsafe { ::neocogi_com::decode_inline::<u32>(version) } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let mut __nc_state_values = match unsafe {
            ::neocogi_com::DecodedInput::<u8>::from_raw(__nc_state_ptr, __nc_state_len)
        } {
            Ok(values) => values,
            Err(status) => return status,
        };
        let state = ::neocogi_com::NcCopyFromBufferOffset::new(
            __nc_state_offset,
            __nc_state_values.as_slice(),
        );
        let child_count = match unsafe {
            ::neocogi_com::decode_inline::<u32>(child_count)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let result = <T as IPersistenceFactory>::validate(
            &*__nc_dispatch_value,
            version,
            state,
            child_count,
        );
        drop(__nc_dispatch_value);
        if result.is_ok() {}
        match result {
            Ok(()) => ::neocogi_com::NC_OK,
            Err(status) => status.normalize_error(),
        }
    }
    /// Validates raw pointers, borrows the concrete provider through its
    /// object-wide gate, invokes its typed method, and lowers the result.
    /// The gate is released before transferring an owned result; errors
    /// are normalized so an unwritten output can never look successful.
    ///
    /// # Safety
    /// The interface belongs to T's live generated storage. Non-null ABI
    /// arguments must satisfy their method's readable/writable contracts.
    unsafe extern "C" fn restore_thunk(
        this: *mut core::ffi::c_void,
        version: <u32 as ::neocogi_com::ComValue>::Wire,
        __nc_state_offset: u64,
        __nc_state_len: u64,
        __nc_state_ptr: *const <u8 as ::neocogi_com::ComValue>::Wire,
        children: ::neocogi_com::ComRaw<IObjectChildrenAbi>,
        __nc_out: *mut ::neocogi_com::ComRaw<::neocogi_com::IUnknown>,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() || __nc_out.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = this;
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let version = match unsafe { ::neocogi_com::decode_inline::<u32>(version) } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let mut __nc_state_values = match unsafe {
            ::neocogi_com::DecodedInput::<u8>::from_raw(__nc_state_ptr, __nc_state_len)
        } {
            Ok(values) => values,
            Err(status) => return status,
        };
        let state = ::neocogi_com::NcCopyFromBufferOffset::new(
            __nc_state_offset,
            __nc_state_values.as_slice(),
        );
        let children = match unsafe {
            ::neocogi_com::ComRef::<
                IObjectChildrenAbi,
                ::neocogi_com::ThreadLocal,
            >::__nc_from_raw_borrowed_checked(children)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let result = <T as IPersistenceFactory>::restore(
            &*__nc_dispatch_value,
            version,
            state,
            children,
        );
        drop(__nc_dispatch_value);
        if result.is_ok() {}
        match result {
            Ok(value) => {
                unsafe {
                    *__nc_out = value.into_raw();
                }
                ::neocogi_com::NC_OK
            }
            Err(status) => status.normalize_error(),
        }
    }
}
