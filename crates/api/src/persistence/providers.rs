// Generated from core/idl/neocogi/com/persistence/objects.ncidl.
// Regenerate: NC_UPDATE_PERSISTENCE_BINDINGS=1 cargo test -p neocogi-com-macros persistence_bindings_match_idl

use super::*;
#[doc(hidden)]
/// Statically typed vtable and lifecycle adapters for one provider type.
pub struct IObjectChildrenVtblFor<T>(
    /// Associates dispatch with T without allocating or owning a value.
    ::core::marker::PhantomData<T>,
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
                query_interface: ::neocogi_com::lifetime::nc_query_interface,
                add_ref: ::neocogi_com::lifetime::nc_strong_retain,
                release: ::neocogi_com::lifetime::nc_strong_release,
                downgrade: ::core::option::Option::Some(
                    ::neocogi_com::lifetime::nc_downgrade,
                ),
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
        this: *mut ::core::ffi::c_void,
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
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(status) => return status,
        };
        let result = <T as IObjectChildren>::count(&*__nc_dispatch_value);
        ::core::mem::drop(__nc_dispatch_value);
        if result.is_ok() {}
        match result {
            ::core::result::Result::Ok(value) => {
                let raw = match ::neocogi_com::encode_inline(&value) {
                    ::core::result::Result::Ok(raw) => raw,
                    ::core::result::Result::Err(status) => return status,
                };
                unsafe {
                    *__nc_out = raw;
                }
                ::neocogi_com::NC_OK
            }
            ::core::result::Result::Err(status) => status.normalize_error(),
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
        this: *mut ::core::ffi::c_void,
        __nc_arg_0: <u32 as ::neocogi_com::ComValue>::Wire,
        __nc_out: *mut ::neocogi_com::ComRaw<::neocogi_com::IUnknown>,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() || __nc_out.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = this;
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_arg_0 = match unsafe {
            ::neocogi_com::decode_inline::<u32>(__nc_arg_0)
        } {
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(status) => return status,
        };
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(status) => return status,
        };
        let result = <T as IObjectChildren>::get(&*__nc_dispatch_value, __nc_arg_0);
        ::core::mem::drop(__nc_dispatch_value);
        if result.is_ok() {}
        match result {
            ::core::result::Result::Ok(value) => {
                unsafe {
                    *__nc_out = value.into_raw();
                }
                ::neocogi_com::NC_OK
            }
            ::core::result::Result::Err(status) => status.normalize_error(),
        }
    }
}
#[doc(hidden)]
/// Statically typed vtable and lifecycle adapters for one provider type.
pub struct IPersistableVtblFor<T>(
    /// Associates dispatch with T without allocating or owning a value.
    ::core::marker::PhantomData<T>,
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
                query_interface: ::neocogi_com::lifetime::nc_query_interface,
                add_ref: ::neocogi_com::lifetime::nc_strong_retain,
                release: ::neocogi_com::lifetime::nc_strong_release,
                downgrade: ::core::option::Option::Some(
                    ::neocogi_com::lifetime::nc_downgrade,
                ),
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
        this: *mut ::core::ffi::c_void,
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
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(status) => return status,
        };
        let result = <T as IPersistable>::type_id(&*__nc_dispatch_value);
        ::core::mem::drop(__nc_dispatch_value);
        if result.is_ok() {}
        match result {
            ::core::result::Result::Ok(value) => {
                let raw = match ::neocogi_com::encode_inline(&value) {
                    ::core::result::Result::Ok(raw) => raw,
                    ::core::result::Result::Err(status) => return status,
                };
                unsafe {
                    *__nc_out = raw;
                }
                ::neocogi_com::NC_OK
            }
            ::core::result::Result::Err(status) => status.normalize_error(),
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
        this: *mut ::core::ffi::c_void,
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
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(status) => return status,
        };
        let result = <T as IPersistable>::schema_version(&*__nc_dispatch_value);
        ::core::mem::drop(__nc_dispatch_value);
        if result.is_ok() {}
        match result {
            ::core::result::Result::Ok(value) => {
                let raw = match ::neocogi_com::encode_inline(&value) {
                    ::core::result::Result::Ok(raw) => raw,
                    ::core::result::Result::Err(status) => return status,
                };
                unsafe {
                    *__nc_out = raw;
                }
                ::neocogi_com::NC_OK
            }
            ::core::result::Result::Err(status) => status.normalize_error(),
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
        this: *mut ::core::ffi::c_void,
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
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(status) => return status,
        };
        let result = <T as IPersistable>::save_state(&*__nc_dispatch_value);
        ::core::mem::drop(__nc_dispatch_value);
        if result.is_ok() {}
        match result {
            ::core::result::Result::Ok(value) => {
                unsafe {
                    *__nc_out = value.into_raw();
                }
                ::neocogi_com::NC_OK
            }
            ::core::result::Result::Err(status) => status.normalize_error(),
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
        this: *mut ::core::ffi::c_void,
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
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(status) => return status,
        };
        let result = <T as IPersistable>::child_count(&*__nc_dispatch_value);
        ::core::mem::drop(__nc_dispatch_value);
        if result.is_ok() {}
        match result {
            ::core::result::Result::Ok(value) => {
                let raw = match ::neocogi_com::encode_inline(&value) {
                    ::core::result::Result::Ok(raw) => raw,
                    ::core::result::Result::Err(status) => return status,
                };
                unsafe {
                    *__nc_out = raw;
                }
                ::neocogi_com::NC_OK
            }
            ::core::result::Result::Err(status) => status.normalize_error(),
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
        this: *mut ::core::ffi::c_void,
        __nc_arg_0: <u32 as ::neocogi_com::ComValue>::Wire,
        __nc_out: *mut ::neocogi_com::ComRaw<::neocogi_com::IUnknown>,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() || __nc_out.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = this;
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_arg_0 = match unsafe {
            ::neocogi_com::decode_inline::<u32>(__nc_arg_0)
        } {
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(status) => return status,
        };
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(status) => return status,
        };
        let result = <T as IPersistable>::child(&*__nc_dispatch_value, __nc_arg_0);
        ::core::mem::drop(__nc_dispatch_value);
        if result.is_ok() {}
        match result {
            ::core::result::Result::Ok(value) => {
                unsafe {
                    *__nc_out = value.into_raw();
                }
                ::neocogi_com::NC_OK
            }
            ::core::result::Result::Err(status) => status.normalize_error(),
        }
    }
}
#[doc(hidden)]
/// Statically typed vtable and lifecycle adapters for one provider type.
pub struct IPersistenceFactoryVtblFor<T>(
    /// Associates dispatch with T without allocating or owning a value.
    ::core::marker::PhantomData<T>,
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
                query_interface: ::neocogi_com::lifetime::nc_query_interface,
                add_ref: ::neocogi_com::lifetime::nc_strong_retain,
                release: ::neocogi_com::lifetime::nc_strong_release,
                downgrade: ::core::option::Option::Some(
                    ::neocogi_com::lifetime::nc_downgrade,
                ),
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
        this: *mut ::core::ffi::c_void,
        __nc_arg_0: <u32 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_1_offset: u64,
        __nc_arg_1_len: u64,
        __nc_arg_1_ptr: *const <u8 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_2: <u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = this;
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_arg_0 = match unsafe {
            ::neocogi_com::decode_inline::<u32>(__nc_arg_0)
        } {
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(status) => return status,
        };
        let mut __nc_arg_1_values = match unsafe {
            ::neocogi_com::DecodedInput::<u8>::from_raw(__nc_arg_1_ptr, __nc_arg_1_len)
        } {
            ::core::result::Result::Ok(values) => values,
            ::core::result::Result::Err(status) => return status,
        };
        let __nc_arg_1 = ::neocogi_com::NcCopyFromBufferOffset::new(
            __nc_arg_1_offset,
            __nc_arg_1_values.as_slice(),
        );
        let __nc_arg_2 = match unsafe {
            ::neocogi_com::decode_inline::<u32>(__nc_arg_2)
        } {
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(status) => return status,
        };
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(status) => return status,
        };
        let result = <T as IPersistenceFactory>::validate(
            &*__nc_dispatch_value,
            __nc_arg_0,
            __nc_arg_1,
            __nc_arg_2,
        );
        ::core::mem::drop(__nc_dispatch_value);
        if result.is_ok() {}
        match result {
            ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
            ::core::result::Result::Err(status) => status.normalize_error(),
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
        this: *mut ::core::ffi::c_void,
        __nc_arg_0: <u32 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_1_offset: u64,
        __nc_arg_1_len: u64,
        __nc_arg_1_ptr: *const <u8 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_2: ::neocogi_com::ComRaw<IObjectChildrenAbi>,
        __nc_out: *mut ::neocogi_com::ComRaw<::neocogi_com::IUnknown>,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() || __nc_out.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = this;
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_arg_0 = match unsafe {
            ::neocogi_com::decode_inline::<u32>(__nc_arg_0)
        } {
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(status) => return status,
        };
        let mut __nc_arg_1_values = match unsafe {
            ::neocogi_com::DecodedInput::<u8>::from_raw(__nc_arg_1_ptr, __nc_arg_1_len)
        } {
            ::core::result::Result::Ok(values) => values,
            ::core::result::Result::Err(status) => return status,
        };
        let __nc_arg_1 = ::neocogi_com::NcCopyFromBufferOffset::new(
            __nc_arg_1_offset,
            __nc_arg_1_values.as_slice(),
        );
        let __nc_arg_2 = match unsafe {
            ::neocogi_com::ComRef::<
                IObjectChildrenAbi,
                ::neocogi_com::ThreadLocal,
            >::__nc_from_raw_borrowed_checked(__nc_arg_2)
        } {
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(status) => return status,
        };
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(status) => return status,
        };
        let result = <T as IPersistenceFactory>::restore(
            &*__nc_dispatch_value,
            __nc_arg_0,
            __nc_arg_1,
            __nc_arg_2,
        );
        ::core::mem::drop(__nc_dispatch_value);
        if result.is_ok() {}
        match result {
            ::core::result::Result::Ok(value) => {
                unsafe {
                    *__nc_out = value.into_raw();
                }
                ::neocogi_com::NC_OK
            }
            ::core::result::Result::Err(status) => status.normalize_error(),
        }
    }
}
