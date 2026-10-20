// Generated from core/idl/neocogi/com/persistence/objects.ncidl.
// Regenerate: NC_UPDATE_PERSISTENCE_BINDINGS=1 cargo test -p neocogi-com-codegen persistence_bindings_match_idl

use super::*;
#[doc(hidden)]
/// Statically typed vtable and lifecycle adapters for one provider type.
pub struct IObjectChildrenVtblFor<__NcProvider>(
    /// Associates dispatch with T without allocating or owning a value.
    ::core::marker::PhantomData<__NcProvider>,
);
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
        identity as __nc_identity, identity as __nc_value_0, identity as __nc_this_0,
        identity as __nc_status_0, identity as __nc_result_0, identity as __nc_prepared,
        identity as __nc_out_0, identity as __nc_value_1, identity as __nc_this_1,
        identity as __nc_status_1, identity as __nc_result_1,
        identity as __nc_prepared_0, identity as __nc_out_1,
    };
    impl<__NcProvider> IObjectChildrenVtblFor<__NcProvider>
    where
        __NcProvider: IObjectChildren + ::neocogi_com::ComObject,
    {
        /// Distinct provider bindings in the interface's canonical metadata order.
        /// T fixes the thunk implementation; no object headers are stored here.
        pub const PROJECTIONS: [::neocogi_com::__ComProjection<
            __NcProvider,
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
            __nc_identity: &'static ::neocogi_com::NcInterfaceId,
        ) -> IObjectChildrenVTable {
            IObjectChildrenVTable {
                unknown: ::neocogi_com::IUnknownVTable {
                    interface_id: __nc_identity,
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
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <u32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_ref(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IObjectChildren>::count(
                &*__nc_dispatch_value,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(__nc_value) => {
                    let __nc_raw = match ::neocogi_com::encode_inline(&__nc_value) {
                        ::core::result::Result::Ok(__nc_raw) => __nc_raw,
                        ::core::result::Result::Err(__nc_status) => return __nc_status,
                    };
                    unsafe {
                        *__nc_out = __nc_raw;
                    }
                    ::neocogi_com::NC_OK
                }
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
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
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0: <u32 as ::neocogi_com::ComValue>::Wire,
            __nc_out: *mut ::neocogi_com::ComRaw<::neocogi_com::IUnknown>,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0 = match unsafe {
                ::neocogi_com::decode_inline::<u32>(__nc_arg_0)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_ref(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IObjectChildren>::get(
                &*__nc_dispatch_value,
                __nc_arg_0,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(__nc_value) => {
                    unsafe {
                        *__nc_out = __nc_value.into_raw();
                    }
                    ::neocogi_com::NC_OK
                }
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
    }
};
#[doc(hidden)]
/// Statically typed vtable and lifecycle adapters for one provider type.
pub struct IPersistableVtblFor<__NcProvider>(
    /// Associates dispatch with T without allocating or owning a value.
    ::core::marker::PhantomData<__NcProvider>,
);
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
        identity as __nc_identity, identity as __nc_value_0, identity as __nc_this_0,
        identity as __nc_status_0, identity as __nc_result_0, identity as __nc_prepared,
        identity as __nc_out_0, identity as __nc_value_1, identity as __nc_this_1,
        identity as __nc_status_1, identity as __nc_result_1,
        identity as __nc_prepared_0, identity as __nc_out_1, identity as __nc_value_2,
        identity as __nc_this_2, identity as __nc_status_2, identity as __nc_result_2,
        identity as __nc_prepared_1, identity as __nc_out_2, identity as __nc_value_3,
        identity as __nc_this_3, identity as __nc_status_3, identity as __nc_result_3,
        identity as __nc_prepared_2, identity as __nc_out_3, identity as __nc_value_4,
        identity as __nc_this_4, identity as __nc_status_4, identity as __nc_result_4,
        identity as __nc_prepared_3, identity as __nc_out_4,
    };
    impl<__NcProvider> IPersistableVtblFor<__NcProvider>
    where
        __NcProvider: IPersistable + ::neocogi_com::ComObject,
    {
        /// Distinct provider bindings in the interface's canonical metadata order.
        /// T fixes the thunk implementation; no object headers are stored here.
        pub const PROJECTIONS: [::neocogi_com::__ComProjection<
            __NcProvider,
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
            __nc_identity: &'static ::neocogi_com::NcInterfaceId,
        ) -> IPersistableVTable {
            IPersistableVTable {
                unknown: ::neocogi_com::IUnknownVTable {
                    interface_id: __nc_identity,
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
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::neocogi_com::NcGuid as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_ref(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IPersistable>::type_id(
                &*__nc_dispatch_value,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(__nc_value) => {
                    let __nc_raw = match ::neocogi_com::encode_inline(&__nc_value) {
                        ::core::result::Result::Ok(__nc_raw) => __nc_raw,
                        ::core::result::Result::Err(__nc_status) => return __nc_status,
                    };
                    unsafe {
                        *__nc_out = __nc_raw;
                    }
                    ::neocogi_com::NC_OK
                }
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
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
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <u32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_ref(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IPersistable>::schema_version(
                &*__nc_dispatch_value,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(__nc_value) => {
                    let __nc_raw = match ::neocogi_com::encode_inline(&__nc_value) {
                        ::core::result::Result::Ok(__nc_raw) => __nc_raw,
                        ::core::result::Result::Err(__nc_status) => return __nc_status,
                    };
                    unsafe {
                        *__nc_out = __nc_raw;
                    }
                    ::neocogi_com::NC_OK
                }
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
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
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut ::neocogi_com::ComRaw<::neocogi_com::INcBytesAbi>,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_ref(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IPersistable>::save_state(
                &*__nc_dispatch_value,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(__nc_value) => {
                    unsafe {
                        *__nc_out = __nc_value.into_raw();
                    }
                    ::neocogi_com::NC_OK
                }
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
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
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <u32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_ref(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IPersistable>::child_count(
                &*__nc_dispatch_value,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(__nc_value) => {
                    let __nc_raw = match ::neocogi_com::encode_inline(&__nc_value) {
                        ::core::result::Result::Ok(__nc_raw) => __nc_raw,
                        ::core::result::Result::Err(__nc_status) => return __nc_status,
                    };
                    unsafe {
                        *__nc_out = __nc_raw;
                    }
                    ::neocogi_com::NC_OK
                }
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
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
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0: <u32 as ::neocogi_com::ComValue>::Wire,
            __nc_out: *mut ::neocogi_com::ComRaw<::neocogi_com::IUnknown>,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0 = match unsafe {
                ::neocogi_com::decode_inline::<u32>(__nc_arg_0)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_ref(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IPersistable>::child(
                &*__nc_dispatch_value,
                __nc_arg_0,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(__nc_value) => {
                    unsafe {
                        *__nc_out = __nc_value.into_raw();
                    }
                    ::neocogi_com::NC_OK
                }
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
    }
};
#[doc(hidden)]
/// Statically typed vtable and lifecycle adapters for one provider type.
pub struct IPersistenceFactoryVtblFor<__NcProvider>(
    /// Associates dispatch with T without allocating or owning a value.
    ::core::marker::PhantomData<__NcProvider>,
);
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
        identity as __nc_value_0, identity as __nc_this_0, identity as __nc_status_0,
        identity as __nc_result_0, identity as __nc_prepared, identity as __nc_out_0,
        identity as __nc_value_1, identity as __nc_this_1, identity as __nc_status_1,
        identity as __nc_result_1, identity as __nc_prepared_0, identity as __nc_out_1,
    };
    impl<__NcProvider> IPersistenceFactoryVtblFor<__NcProvider>
    where
        __NcProvider: IPersistenceFactory + ::neocogi_com::ComObject,
    {
        /// Distinct provider bindings in the interface's canonical metadata order.
        /// T fixes the thunk implementation; no object headers are stored here.
        pub const PROJECTIONS: [::neocogi_com::__ComProjection<
            __NcProvider,
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
            __nc_identity: &'static ::neocogi_com::NcInterfaceId,
        ) -> IPersistenceFactoryVTable {
            IPersistenceFactoryVTable {
                unknown: ::neocogi_com::IUnknownVTable {
                    interface_id: __nc_identity,
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
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0: <u32 as ::neocogi_com::ComValue>::Wire,
            __nc_arg_1_offset: u64,
            __nc_arg_1_len: u64,
            __nc_arg_1_ptr: *const <u8 as ::neocogi_com::ComValue>::Wire,
            __nc_arg_2: <u32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0 = match unsafe {
                ::neocogi_com::decode_inline::<u32>(__nc_arg_0)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_1_values = match unsafe {
                ::neocogi_com::DecodedInput::<
                    u8,
                >::from_raw(__nc_arg_1_ptr, __nc_arg_1_len)
            } {
                ::core::result::Result::Ok(__nc_values) => __nc_values,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_1 = ::neocogi_com::NcCopyFromBufferOffset::new(
                __nc_arg_1_offset,
                __nc_arg_1_values.as_slice(),
            );
            let __nc_arg_2 = match unsafe {
                ::neocogi_com::decode_inline::<u32>(__nc_arg_2)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_ref(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IPersistenceFactory>::validate(
                &*__nc_dispatch_value,
                __nc_arg_0,
                __nc_arg_1,
                __nc_arg_2,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
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
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_0: <u32 as ::neocogi_com::ComValue>::Wire,
            __nc_arg_1_0_offset: u64,
            __nc_arg_1_0_len: u64,
            __nc_arg_1_0_ptr: *const <u8 as ::neocogi_com::ComValue>::Wire,
            __nc_arg_2_0: ::neocogi_com::ComRaw<self::IObjectChildrenAbi>,
            __nc_out: *mut ::neocogi_com::ComRaw<::neocogi_com::IUnknown>,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_0 = match unsafe {
                ::neocogi_com::decode_inline::<u32>(__nc_arg_0_0)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_1_0_values = match unsafe {
                ::neocogi_com::DecodedInput::<
                    u8,
                >::from_raw(__nc_arg_1_0_ptr, __nc_arg_1_0_len)
            } {
                ::core::result::Result::Ok(__nc_values) => __nc_values,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_1_0 = ::neocogi_com::NcCopyFromBufferOffset::new(
                __nc_arg_1_0_offset,
                __nc_arg_1_0_values.as_slice(),
            );
            let __nc_arg_2_0 = match unsafe {
                ::neocogi_com::ComRef::<
                    self::IObjectChildrenAbi,
                    ::neocogi_com::ThreadLocal,
                >::__nc_from_raw_borrowed_checked(__nc_arg_2_0)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_ref(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IPersistenceFactory>::restore(
                &*__nc_dispatch_value,
                __nc_arg_0_0,
                __nc_arg_1_0,
                __nc_arg_2_0,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(__nc_value) => {
                    unsafe {
                        *__nc_out = __nc_value.into_raw();
                    }
                    ::neocogi_com::NC_OK
                }
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
    }
};
