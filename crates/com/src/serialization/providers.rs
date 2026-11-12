// Generated from core/schemas/neocogi/com/serialization/formats.ncidl.
// Regenerate: NC_UPDATE_PERSISTENCE_BINDINGS=1 cargo test -p neocogi-idl serialization_bindings_match_idl

use super::*;
#[doc(hidden)]
/// Statically typed vtable and lifecycle adapters for one provider type.
pub struct IDeserializerVtblFor<__NcProvider>(
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
        identity as __nc_arg_0_written, identity as __nc_arg_0_0,
        identity as __nc_arg_0_0_owner_slot, identity as __nc_arg_0_0_offset,
        identity as __nc_arg_0_0_len, identity as __nc_arg_0_0_ptr,
        identity as __nc_arg_0_0_values, identity as __nc_arg_0_0_written,
        identity as __nc_arg_0_1, identity as __nc_arg_0_1_owner_slot,
        identity as __nc_arg_0_1_offset, identity as __nc_arg_0_1_len,
        identity as __nc_arg_0_1_ptr, identity as __nc_arg_0_1_values,
        identity as __nc_arg_0_1_written, identity as __nc_arg_0_2,
        identity as __nc_arg_0_2_owner_slot, identity as __nc_arg_0_2_offset,
        identity as __nc_arg_0_2_len, identity as __nc_arg_0_2_ptr,
        identity as __nc_arg_0_2_values, identity as __nc_arg_0_2_written,
        identity as __nc_arg_1, identity as __nc_arg_1_owner_slot,
        identity as __nc_arg_1_offset, identity as __nc_arg_1_len,
        identity as __nc_arg_1_ptr, identity as __nc_arg_1_values,
        identity as __nc_arg_1_written, identity as __nc_arg_0_3,
        identity as __nc_arg_0_3_owner_slot, identity as __nc_arg_0_3_offset,
        identity as __nc_arg_0_3_len, identity as __nc_arg_0_3_ptr,
        identity as __nc_arg_0_3_values, identity as __nc_arg_0_3_written,
        identity as __nc_this, identity as __nc_status, identity as __nc_value,
        identity as __nc_raw, identity as __nc_result, identity as __nc_values,
        identity as __nc_writer, identity as __nc_out, identity as __nc_identity,
        identity as __nc_this_0, identity as __nc_status_0, identity as __nc_result_0,
        identity as __nc_prepared, identity as __nc_out_0, identity as __nc_this_1,
        identity as __nc_status_1, identity as __nc_result_1,
        identity as __nc_prepared_0, identity as __nc_out_1, identity as __nc_this_2,
        identity as __nc_status_2, identity as __nc_result_2,
        identity as __nc_prepared_1, identity as __nc_out_2, identity as __nc_this_3,
        identity as __nc_status_3, identity as __nc_result_3,
        identity as __nc_prepared_2, identity as __nc_out_3, identity as __nc_this_4,
        identity as __nc_status_4, identity as __nc_result_4,
        identity as __nc_prepared_3, identity as __nc_out_4, identity as __nc_this_5,
        identity as __nc_status_5, identity as __nc_result_5,
        identity as __nc_prepared_4, identity as __nc_out_5, identity as __nc_this_6,
        identity as __nc_status_6, identity as __nc_result_6,
        identity as __nc_prepared_5, identity as __nc_out_6, identity as __nc_this_7,
        identity as __nc_status_7, identity as __nc_result_7,
        identity as __nc_prepared_6, identity as __nc_out_7, identity as __nc_this_8,
        identity as __nc_status_8, identity as __nc_result_8,
        identity as __nc_prepared_7, identity as __nc_out_8, identity as __nc_this_9,
        identity as __nc_status_9, identity as __nc_result_9,
        identity as __nc_prepared_8, identity as __nc_out_9, identity as __nc_this_10,
        identity as __nc_status_10, identity as __nc_result_10,
        identity as __nc_prepared_9, identity as __nc_out_10, identity as __nc_this_11,
        identity as __nc_status_11, identity as __nc_result_11,
        identity as __nc_prepared_10, identity as __nc_out_11, identity as __nc_this_12,
        identity as __nc_status_12, identity as __nc_result_12,
        identity as __nc_prepared_11, identity as __nc_out_12, identity as __nc_this_13,
        identity as __nc_status_13, identity as __nc_result_13,
        identity as __nc_prepared_12, identity as __nc_out_13, identity as __nc_this_14,
        identity as __nc_status_14, identity as __nc_result_14,
        identity as __nc_prepared_13, identity as __nc_out_14, identity as __nc_this_15,
        identity as __nc_status_15, identity as __nc_result_15,
        identity as __nc_prepared_14, identity as __nc_out_15, identity as __nc_this_16,
        identity as __nc_status_16, identity as __nc_result_16,
        identity as __nc_prepared_15, identity as __nc_out_16, identity as __nc_this_17,
        identity as __nc_status_17, identity as __nc_result_17,
        identity as __nc_prepared_16, identity as __nc_out_17, identity as __nc_this_18,
        identity as __nc_status_18, identity as __nc_result_18,
        identity as __nc_prepared_17, identity as __nc_out_18, identity as __nc_this_19,
        identity as __nc_status_19, identity as __nc_result_19,
        identity as __nc_prepared_18, identity as __nc_out_19, identity as __nc_this_20,
        identity as __nc_status_20, identity as __nc_result_20,
        identity as __nc_prepared_19, identity as __nc_out_20, identity as __nc_this_21,
        identity as __nc_status_21, identity as __nc_result_21,
        identity as __nc_prepared_20, identity as __nc_out_21, identity as __nc_this_22,
        identity as __nc_status_22, identity as __nc_result_22,
        identity as __nc_prepared_21, identity as __nc_out_22, identity as __nc_this_23,
        identity as __nc_status_23, identity as __nc_result_23,
        identity as __nc_prepared_22, identity as __nc_out_23, identity as __nc_this_24,
        identity as __nc_status_24, identity as __nc_result_24,
        identity as __nc_prepared_23, identity as __nc_out_24, identity as __nc_this_25,
        identity as __nc_status_25, identity as __nc_result_25,
        identity as __nc_prepared_24, identity as __nc_out_25, identity as __nc_this_26,
        identity as __nc_status_26, identity as __nc_result_26,
        identity as __nc_prepared_25, identity as __nc_out_26,
    };
    impl<__NcProvider> IDeserializerVtblFor<__NcProvider>
    where
        __NcProvider: IDeserializer + ::neocogi_com::ComObject,
    {
        /// Distinct provider bindings in the interface's canonical metadata order.
        /// T fixes the thunk implementation; no object headers are stored here.
        pub const PROJECTIONS: [::neocogi_com::__ComProjection<
            __NcProvider,
        >; IDeserializerAbi::INTERFACE_CHAIN.len()] = ::neocogi_com::__nc_projection_table(
            &IDeserializerAbi::INTERFACE_CHAIN,
            unsafe {
                ::neocogi_com::__ComProjection::new(
                    <IDeserializerAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
                    &Self::VTABLE as *const IDeserializerVTable
                        as *const ::neocogi_com::IUnknownVTable,
                )
            },
            &[],
        );
        /// Table for a standalone projection of this interface.
        pub const VTABLE: IDeserializerVTable = Self::with_identity(
            &<IDeserializerAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
        );
        /// Builds an inherited prefix advertising the actual outer projection.
        pub const fn with_identity(
            __nc_identity: &'static ::neocogi_com::NcInterfaceId,
        ) -> IDeserializerVTable {
            IDeserializerVTable {
                unknown: ::neocogi_com::IUnknownVTable {
                    interface_id: __nc_identity,
                    query_interface: ::neocogi_com::lifetime::nc_query_interface,
                    add_ref: ::neocogi_com::lifetime::nc_strong_retain,
                    release: ::neocogi_com::lifetime::nc_strong_release,
                    downgrade: ::core::option::Option::Some(
                        ::neocogi_com::lifetime::nc_downgrade,
                    ),
                },
                enter_value: Self::enter_value_thunk,
                leave_value: Self::leave_value_thunk,
                deserialize_unit: Self::deserialize_unit_thunk,
                deserialize_bool: Self::deserialize_bool_thunk,
                deserialize_u8: Self::deserialize_u8_thunk,
                deserialize_u16: Self::deserialize_u16_thunk,
                deserialize_u32: Self::deserialize_u32_thunk,
                deserialize_u64: Self::deserialize_u64_thunk,
                deserialize_i8: Self::deserialize_i8_thunk,
                deserialize_i16: Self::deserialize_i16_thunk,
                deserialize_i32: Self::deserialize_i32_thunk,
                deserialize_i64: Self::deserialize_i64_thunk,
                deserialize_f32: Self::deserialize_f32_thunk,
                deserialize_f64: Self::deserialize_f64_thunk,
                string_len: Self::string_len_thunk,
                copy_string: Self::copy_string_thunk,
                option_present: Self::option_present_thunk,
                begin_record: Self::begin_record_thunk,
                field: Self::field_thunk,
                end_record: Self::end_record_thunk,
                begin_sequence: Self::begin_sequence_thunk,
                end_sequence: Self::end_sequence_thunk,
                begin_map: Self::begin_map_thunk,
                end_map: Self::end_map_thunk,
                variant_is: Self::variant_is_thunk,
                fail: Self::fail_thunk,
                finish: Self::finish_thunk,
            }
        }
        /// Opens one typed value, charging nesting and construction limits.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:60:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn enter_value_thunk(
            __nc_this: *mut ::core::ffi::c_void,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::enter_value(
                &mut *__nc_dispatch_value,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Completes the current typed value after its children have been consumed.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:62:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn leave_value_thunk(
            __nc_this: *mut ::core::ffi::c_void,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::leave_value(
                &mut *__nc_dispatch_value,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Reads a unit value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:64:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn deserialize_unit_thunk(
            __nc_this: *mut ::core::ffi::c_void,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::deserialize_unit(
                &mut *__nc_dispatch_value,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Acquires one validated bool value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:66:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn deserialize_bool_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::bool as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::deserialize_bool(
                &mut *__nc_dispatch_value,
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
        /// Acquires one validated u8 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:68:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn deserialize_u8_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::deserialize_u8(
                &mut *__nc_dispatch_value,
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
        /// Acquires one validated u16 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:70:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn deserialize_u16_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::u16 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::deserialize_u16(
                &mut *__nc_dispatch_value,
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
        /// Acquires one validated u32 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:72:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn deserialize_u32_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::deserialize_u32(
                &mut *__nc_dispatch_value,
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
        /// Acquires one validated u64 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:74:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn deserialize_u64_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::deserialize_u64(
                &mut *__nc_dispatch_value,
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
        /// Acquires one validated i8 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:76:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn deserialize_i8_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::i8 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::deserialize_i8(
                &mut *__nc_dispatch_value,
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
        /// Acquires one validated i16 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:78:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn deserialize_i16_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::i16 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::deserialize_i16(
                &mut *__nc_dispatch_value,
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
        /// Acquires one validated i32 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:80:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn deserialize_i32_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::i32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::deserialize_i32(
                &mut *__nc_dispatch_value,
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
        /// Acquires one validated i64 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:82:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn deserialize_i64_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::i64 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::deserialize_i64(
                &mut *__nc_dispatch_value,
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
        /// Acquires one validated f32 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:84:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn deserialize_f32_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::f32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::deserialize_f32(
                &mut *__nc_dispatch_value,
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
        /// Acquires one validated f64 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:86:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn deserialize_f64_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::f64 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::deserialize_f64(
                &mut *__nc_dispatch_value,
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
        /// Admits a complete UTF-8 string and reports its byte length before allocation.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:88:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn string_len_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::string_len(
                &mut *__nc_dispatch_value,
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
        /// Copies exactly the admitted string into fresh caller-owned output.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:90:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn copy_string_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_offset: ::core::primitive::u64,
            __nc_arg_0_len: ::core::primitive::u64,
            __nc_arg_0_ptr: *mut <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
            __nc_arg_0_written: *mut ::core::primitive::u64,
            __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            if __nc_arg_0_written.is_null()
                || __nc_arg_0_written
                    .align_offset(::core::mem::align_of::<::core::primitive::u64>()) != 0
            {
                return ::neocogi_com::NC_E_POINTER;
            }
            unsafe {
                *__nc_arg_0_written = 0;
            }
            let mut __nc_arg_0_values = 0_u64;
            let __nc_arg_0 = match unsafe {
                ::neocogi_com::NcOutputWriter::<
                    ::core::primitive::u8,
                >::from_raw(
                    __nc_arg_0_offset,
                    __nc_arg_0_ptr,
                    __nc_arg_0_len,
                    &mut __nc_arg_0_values,
                )
            } {
                ::core::result::Result::Ok(__nc_writer) => __nc_writer,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::copy_string(
                &mut *__nc_dispatch_value,
                __nc_arg_0,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(__nc_value) => {
                    let __nc_raw = match ::neocogi_com::encode_inline(&__nc_value) {
                        ::core::result::Result::Ok(__nc_raw) => __nc_raw,
                        ::core::result::Result::Err(__nc_status) => return __nc_status,
                    };
                    unsafe {
                        *__nc_arg_0_written = __nc_arg_0_values;
                    }
                    unsafe {
                        *__nc_out = __nc_raw;
                    }
                    ::neocogi_com::NC_OK
                }
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Reads optional presence; a present child follows within this value scope.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:92:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn option_present_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::bool as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::option_present(
                &mut *__nc_dispatch_value,
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
        /// Opens a record and rejects more than maximum fields.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:94:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn begin_record_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_0: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
            __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_0 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::u32>(__nc_arg_0_0)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::begin_record(
                &mut *__nc_dispatch_value,
                __nc_arg_0_0,
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
        /// Selects a named field in schema order; false means genuinely absent.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:96:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn field_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_1_offset: ::core::primitive::u64,
            __nc_arg_0_1_len: ::core::primitive::u64,
            __nc_arg_0_1_ptr: *const <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
            __nc_out: *mut <::core::primitive::bool as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_1_values = match unsafe {
                ::neocogi_com::DecodedInput::<
                    ::core::primitive::u8,
                >::from_raw(__nc_arg_0_1_ptr, __nc_arg_0_1_len)
            } {
                ::core::result::Result::Ok(__nc_values) => __nc_values,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_0_1 = ::neocogi_com::NcCopyFromBufferOffset::new(
                __nc_arg_0_1_offset,
                __nc_arg_0_1_values.as_slice(),
            );
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::field(
                &mut *__nc_dispatch_value,
                __nc_arg_0_1,
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
        /// Completes the record, rejecting unconsumed or unknown fields.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:98:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn end_record_thunk(
            __nc_this: *mut ::core::ffi::c_void,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::end_record(
                &mut *__nc_dispatch_value,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Opens an ordered collection and reports its bounded element count.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:100:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn begin_sequence_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::begin_sequence(
                &mut *__nc_dispatch_value,
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
        /// Completes the collection, rejecting unconsumed elements.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:102:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn end_sequence_thunk(
            __nc_this: *mut ::core::ffi::c_void,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::end_sequence(
                &mut *__nc_dispatch_value,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Opens a map and reports its bounded pair count.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:104:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn begin_map_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::begin_map(
                &mut *__nc_dispatch_value,
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
        /// Completes the map.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:106:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn end_map_thunk(
            __nc_this: *mut ::core::ffi::c_void,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::end_map(
                &mut *__nc_dispatch_value,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Probes cases in declaration order; a match selects the following payload.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:108:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn variant_is_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_2: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
            __nc_arg_1_offset: ::core::primitive::u64,
            __nc_arg_1_len: ::core::primitive::u64,
            __nc_arg_1_ptr: *const <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
            __nc_out: *mut <::core::primitive::bool as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_2 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::u32>(__nc_arg_0_2)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_1_values = match unsafe {
                ::neocogi_com::DecodedInput::<
                    ::core::primitive::u8,
                >::from_raw(__nc_arg_1_ptr, __nc_arg_1_len)
            } {
                ::core::result::Result::Ok(__nc_values) => __nc_values,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_1 = ::neocogi_com::NcCopyFromBufferOffset::new(
                __nc_arg_1_offset,
                __nc_arg_1_values.as_slice(),
            );
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::variant_is(
                &mut *__nc_dispatch_value,
                __nc_arg_0_2,
                __nc_arg_1,
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
        /// Marks a schema failure; no later operation may succeed.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:110:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn fail_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_3_offset: ::core::primitive::u64,
            __nc_arg_0_3_len: ::core::primitive::u64,
            __nc_arg_0_3_ptr: *const <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_3_values = match unsafe {
                ::neocogi_com::DecodedInput::<
                    ::core::primitive::u8,
                >::from_raw(__nc_arg_0_3_ptr, __nc_arg_0_3_len)
            } {
                ::core::result::Result::Ok(__nc_values) => __nc_values,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_0_3 = ::neocogi_com::NcCopyFromBufferOffset::new(
                __nc_arg_0_3_offset,
                __nc_arg_0_3_values.as_slice(),
            );
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::fail(
                &mut *__nc_dispatch_value,
                __nc_arg_0_3,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Requires a complete root with no trailing input and closes the session.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:112:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn finish_thunk(
            __nc_this: *mut ::core::ffi::c_void,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as IDeserializer>::finish(
                &mut *__nc_dispatch_value,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
    }
};
#[doc(hidden)]
/// Statically typed vtable and lifecycle adapters for one provider type.
pub struct ISerializationDiagnosticsVtblFor<__NcProvider>(
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
        identity as __nc_arg_0_written, identity as __nc_arg_0_0,
        identity as __nc_arg_0_0_owner_slot, identity as __nc_arg_0_0_offset,
        identity as __nc_arg_0_0_len, identity as __nc_arg_0_0_ptr,
        identity as __nc_arg_0_0_values, identity as __nc_arg_0_0_written,
        identity as __nc_this, identity as __nc_status, identity as __nc_value,
        identity as __nc_raw, identity as __nc_result, identity as __nc_values,
        identity as __nc_writer, identity as __nc_out, identity as __nc_identity,
        identity as __nc_this_0, identity as __nc_status_0, identity as __nc_result_0,
        identity as __nc_prepared, identity as __nc_out_0, identity as __nc_this_1,
        identity as __nc_status_1, identity as __nc_result_1,
        identity as __nc_prepared_0, identity as __nc_out_1, identity as __nc_this_2,
        identity as __nc_status_2, identity as __nc_result_2,
        identity as __nc_prepared_1, identity as __nc_out_2, identity as __nc_this_3,
        identity as __nc_status_3, identity as __nc_result_3,
        identity as __nc_prepared_2, identity as __nc_out_3, identity as __nc_this_4,
        identity as __nc_status_4, identity as __nc_result_4,
        identity as __nc_prepared_3, identity as __nc_out_4, identity as __nc_this_5,
        identity as __nc_status_5, identity as __nc_result_5,
        identity as __nc_prepared_4, identity as __nc_out_5, identity as __nc_this_6,
        identity as __nc_status_6, identity as __nc_result_6,
        identity as __nc_prepared_5, identity as __nc_out_6, identity as __nc_this_7,
        identity as __nc_status_7, identity as __nc_result_7,
        identity as __nc_prepared_6, identity as __nc_out_7,
    };
    impl<__NcProvider> ISerializationDiagnosticsVtblFor<__NcProvider>
    where
        __NcProvider: ISerializationDiagnostics + ::neocogi_com::ComObject,
    {
        /// Distinct provider bindings in the interface's canonical metadata order.
        /// T fixes the thunk implementation; no object headers are stored here.
        pub const PROJECTIONS: [::neocogi_com::__ComProjection<
            __NcProvider,
        >; ISerializationDiagnosticsAbi::INTERFACE_CHAIN.len()] = ::neocogi_com::__nc_projection_table(
            &ISerializationDiagnosticsAbi::INTERFACE_CHAIN,
            unsafe {
                ::neocogi_com::__ComProjection::new(
                    <ISerializationDiagnosticsAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
                    &Self::VTABLE as *const ISerializationDiagnosticsVTable
                        as *const ::neocogi_com::IUnknownVTable,
                )
            },
            &[],
        );
        /// Table for a standalone projection of this interface.
        pub const VTABLE: ISerializationDiagnosticsVTable = Self::with_identity(
            &<ISerializationDiagnosticsAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
        );
        /// Builds an inherited prefix advertising the actual outer projection.
        pub const fn with_identity(
            __nc_identity: &'static ::neocogi_com::NcInterfaceId,
        ) -> ISerializationDiagnosticsVTable {
            ISerializationDiagnosticsVTable {
                unknown: ::neocogi_com::IUnknownVTable {
                    interface_id: __nc_identity,
                    query_interface: ::neocogi_com::lifetime::nc_query_interface,
                    add_ref: ::neocogi_com::lifetime::nc_strong_retain,
                    release: ::neocogi_com::lifetime::nc_strong_release,
                    downgrade: ::core::option::Option::Some(
                        ::neocogi_com::lifetime::nc_downgrade,
                    ),
                },
                failure_status: Self::failure_status_thunk,
                byte_offset: Self::byte_offset_thunk,
                line: Self::line_thunk,
                column: Self::column_thunk,
                message_len: Self::message_len_thunk,
                copy_message: Self::copy_message_thunk,
                path_len: Self::path_len_thunk,
                copy_path: Self::copy_path_thunk,
            }
        }
        /// Returns the original failed operation's status, as data.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:143:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn failure_status_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::neocogi_com::NcStatus as ::neocogi_com::ComValue>::Wire,
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
            let __nc_result = <__NcProvider as ISerializationDiagnostics>::failure_status(
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
        /// Reports a zero-based byte cursor/offset, or u64::MAX when unavailable.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:145:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn byte_offset_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
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
            let __nc_result = <__NcProvider as ISerializationDiagnostics>::byte_offset(
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
        /// Reports a one-based source line, or zero when unavailable.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:147:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn line_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
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
            let __nc_result = <__NcProvider as ISerializationDiagnostics>::line(
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
        /// Reports a one-based source column, or zero when unavailable.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:149:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn column_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
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
            let __nc_result = <__NcProvider as ISerializationDiagnostics>::column(
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
        /// Reports the diagnostic message's UTF-8 byte length.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:151:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn message_len_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
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
            let __nc_result = <__NcProvider as ISerializationDiagnostics>::message_len(
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
        /// Copies the complete message to fresh output; short capacity writes no prefix.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:153:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn copy_message_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_offset: ::core::primitive::u64,
            __nc_arg_0_len: ::core::primitive::u64,
            __nc_arg_0_ptr: *mut <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
            __nc_arg_0_written: *mut ::core::primitive::u64,
            __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            if __nc_arg_0_written.is_null()
                || __nc_arg_0_written
                    .align_offset(::core::mem::align_of::<::core::primitive::u64>()) != 0
            {
                return ::neocogi_com::NC_E_POINTER;
            }
            unsafe {
                *__nc_arg_0_written = 0;
            }
            let mut __nc_arg_0_values = 0_u64;
            let __nc_arg_0 = match unsafe {
                ::neocogi_com::NcOutputWriter::<
                    ::core::primitive::u8,
                >::from_raw(
                    __nc_arg_0_offset,
                    __nc_arg_0_ptr,
                    __nc_arg_0_len,
                    &mut __nc_arg_0_values,
                )
            } {
                ::core::result::Result::Ok(__nc_writer) => __nc_writer,
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
            let __nc_result = <__NcProvider as ISerializationDiagnostics>::copy_message(
                &*__nc_dispatch_value,
                __nc_arg_0,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(__nc_value) => {
                    let __nc_raw = match ::neocogi_com::encode_inline(&__nc_value) {
                        ::core::result::Result::Ok(__nc_raw) => __nc_raw,
                        ::core::result::Result::Err(__nc_status) => return __nc_status,
                    };
                    unsafe {
                        *__nc_arg_0_written = __nc_arg_0_values;
                    }
                    unsafe {
                        *__nc_out = __nc_raw;
                    }
                    ::neocogi_com::NC_OK
                }
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Reports the logical field/index path's UTF-8 byte length.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:155:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn path_len_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
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
            let __nc_result = <__NcProvider as ISerializationDiagnostics>::path_len(
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
        /// Copies the complete logical path, rooted at $, into fresh output.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:157:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn copy_path_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_0_offset: ::core::primitive::u64,
            __nc_arg_0_0_len: ::core::primitive::u64,
            __nc_arg_0_0_ptr: *mut <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
            __nc_arg_0_0_written: *mut ::core::primitive::u64,
            __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            if __nc_arg_0_0_written.is_null()
                || __nc_arg_0_0_written
                    .align_offset(::core::mem::align_of::<::core::primitive::u64>()) != 0
            {
                return ::neocogi_com::NC_E_POINTER;
            }
            unsafe {
                *__nc_arg_0_0_written = 0;
            }
            let mut __nc_arg_0_0_values = 0_u64;
            let __nc_arg_0_0 = match unsafe {
                ::neocogi_com::NcOutputWriter::<
                    ::core::primitive::u8,
                >::from_raw(
                    __nc_arg_0_0_offset,
                    __nc_arg_0_0_ptr,
                    __nc_arg_0_0_len,
                    &mut __nc_arg_0_0_values,
                )
            } {
                ::core::result::Result::Ok(__nc_writer) => __nc_writer,
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
            let __nc_result = <__NcProvider as ISerializationDiagnostics>::copy_path(
                &*__nc_dispatch_value,
                __nc_arg_0_0,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(__nc_value) => {
                    let __nc_raw = match ::neocogi_com::encode_inline(&__nc_value) {
                        ::core::result::Result::Ok(__nc_raw) => __nc_raw,
                        ::core::result::Result::Err(__nc_status) => return __nc_status,
                    };
                    unsafe {
                        *__nc_arg_0_0_written = __nc_arg_0_0_values;
                    }
                    unsafe {
                        *__nc_out = __nc_raw;
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
pub struct ISerializationFormatVtblFor<__NcProvider>(
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
        identity as __nc_arg_2_written, identity as __nc_arg_3,
        identity as __nc_arg_3_owner_slot, identity as __nc_arg_3_offset,
        identity as __nc_arg_3_len, identity as __nc_arg_3_ptr,
        identity as __nc_arg_3_values, identity as __nc_arg_3_written,
        identity as __nc_arg_0_0, identity as __nc_arg_0_0_owner_slot,
        identity as __nc_arg_0_0_offset, identity as __nc_arg_0_0_len,
        identity as __nc_arg_0_0_ptr, identity as __nc_arg_0_0_values,
        identity as __nc_arg_0_0_written, identity as __nc_arg_1_0,
        identity as __nc_arg_1_0_owner_slot, identity as __nc_arg_1_0_offset,
        identity as __nc_arg_1_0_len, identity as __nc_arg_1_0_ptr,
        identity as __nc_arg_1_0_values, identity as __nc_arg_1_0_written,
        identity as __nc_arg_2_0, identity as __nc_arg_2_0_owner_slot,
        identity as __nc_arg_2_0_offset, identity as __nc_arg_2_0_len,
        identity as __nc_arg_2_0_ptr, identity as __nc_arg_2_0_values,
        identity as __nc_arg_2_0_written, identity as __nc_arg_3_0,
        identity as __nc_arg_3_0_owner_slot, identity as __nc_arg_3_0_offset,
        identity as __nc_arg_3_0_len, identity as __nc_arg_3_0_ptr,
        identity as __nc_arg_3_0_values, identity as __nc_arg_3_0_written,
        identity as __nc_arg_4, identity as __nc_arg_4_owner_slot,
        identity as __nc_arg_4_offset, identity as __nc_arg_4_len,
        identity as __nc_arg_4_ptr, identity as __nc_arg_4_values,
        identity as __nc_arg_4_written, identity as __nc_this, identity as __nc_status,
        identity as __nc_value, identity as __nc_raw, identity as __nc_result,
        identity as __nc_values, identity as __nc_writer, identity as __nc_out,
        identity as __nc_identity, identity as __nc_this_0, identity as __nc_status_0,
        identity as __nc_result_0, identity as __nc_prepared, identity as __nc_out_0,
        identity as __nc_this_1, identity as __nc_status_1, identity as __nc_result_1,
        identity as __nc_prepared_0, identity as __nc_out_1,
    };
    impl<__NcProvider> ISerializationFormatVtblFor<__NcProvider>
    where
        __NcProvider: ISerializationFormat + ::neocogi_com::ComObject,
    {
        /// Distinct provider bindings in the interface's canonical metadata order.
        /// T fixes the thunk implementation; no object headers are stored here.
        pub const PROJECTIONS: [::neocogi_com::__ComProjection<
            __NcProvider,
        >; ISerializationFormatAbi::INTERFACE_CHAIN.len()] = ::neocogi_com::__nc_projection_table(
            &ISerializationFormatAbi::INTERFACE_CHAIN,
            unsafe {
                ::neocogi_com::__ComProjection::new(
                    <ISerializationFormatAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
                    &Self::VTABLE as *const ISerializationFormatVTable
                        as *const ::neocogi_com::IUnknownVTable,
                )
            },
            &[],
        );
        /// Table for a standalone projection of this interface.
        pub const VTABLE: ISerializationFormatVTable = Self::with_identity(
            &<ISerializationFormatAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
        );
        /// Builds an inherited prefix advertising the actual outer projection.
        pub const fn with_identity(
            __nc_identity: &'static ::neocogi_com::NcInterfaceId,
        ) -> ISerializationFormatVTable {
            ISerializationFormatVTable {
                unknown: ::neocogi_com::IUnknownVTable {
                    interface_id: __nc_identity,
                    query_interface: ::neocogi_com::lifetime::nc_query_interface,
                    add_ref: ::neocogi_com::lifetime::nc_strong_retain,
                    release: ::neocogi_com::lifetime::nc_strong_release,
                    downgrade: ::core::option::Option::Some(
                        ::neocogi_com::lifetime::nc_downgrade,
                    ),
                },
                create_serializer: Self::create_serializer_thunk,
                create_deserializer: Self::create_deserializer_thunk,
            }
        }
        /// Creates a bounded serializer with format-owned output storage.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:126:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn create_serializer_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0: <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
            __nc_arg_1: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
            __nc_arg_2: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
            __nc_arg_3: <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
            __nc_out: *mut ::neocogi_com::ComRaw<self::ISerializerAbi>,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::u64>(__nc_arg_0)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_1 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::u32>(__nc_arg_1)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_2 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::u32>(__nc_arg_2)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_3 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::u64>(__nc_arg_3)
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
            let __nc_result = <__NcProvider as ISerializationFormat>::create_serializer(
                &*__nc_dispatch_value,
                __nc_arg_0,
                __nc_arg_1,
                __nc_arg_2,
                __nc_arg_3,
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
        /// Copies the complete borrowed document into a new bounded source session.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:128:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn create_deserializer_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_0_offset: ::core::primitive::u64,
            __nc_arg_0_0_len: ::core::primitive::u64,
            __nc_arg_0_0_ptr: *const <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
            __nc_arg_1_0: <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
            __nc_arg_2_0: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
            __nc_arg_3_0: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
            __nc_arg_4: <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
            __nc_out: *mut ::neocogi_com::ComRaw<self::IDeserializerAbi>,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_0_values = match unsafe {
                ::neocogi_com::DecodedInput::<
                    ::core::primitive::u8,
                >::from_raw(__nc_arg_0_0_ptr, __nc_arg_0_0_len)
            } {
                ::core::result::Result::Ok(__nc_values) => __nc_values,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_0_0 = ::neocogi_com::NcCopyFromBufferOffset::new(
                __nc_arg_0_0_offset,
                __nc_arg_0_0_values.as_slice(),
            );
            let __nc_arg_1_0 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::u64>(__nc_arg_1_0)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_2_0 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::u32>(__nc_arg_2_0)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_3_0 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::u32>(__nc_arg_3_0)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_4 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::u64>(__nc_arg_4)
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
            let __nc_result = <__NcProvider as ISerializationFormat>::create_deserializer(
                &*__nc_dispatch_value,
                __nc_arg_0_0,
                __nc_arg_1_0,
                __nc_arg_2_0,
                __nc_arg_3_0,
                __nc_arg_4,
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
pub struct ISerializationOutputVtblFor<__NcProvider>(
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
        identity as __nc_identity, identity as __nc_this_0, identity as __nc_status_0,
        identity as __nc_result_0, identity as __nc_prepared, identity as __nc_out_0,
        identity as __nc_this_1, identity as __nc_status_1, identity as __nc_result_1,
        identity as __nc_prepared_0, identity as __nc_out_1,
    };
    impl<__NcProvider> ISerializationOutputVtblFor<__NcProvider>
    where
        __NcProvider: ISerializationOutput + ::neocogi_com::ComObject,
    {
        /// Distinct provider bindings in the interface's canonical metadata order.
        /// T fixes the thunk implementation; no object headers are stored here.
        pub const PROJECTIONS: [::neocogi_com::__ComProjection<
            __NcProvider,
        >; ISerializationOutputAbi::INTERFACE_CHAIN.len()] = ::neocogi_com::__nc_projection_table(
            &ISerializationOutputAbi::INTERFACE_CHAIN,
            unsafe {
                ::neocogi_com::__ComProjection::new(
                    <ISerializationOutputAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
                    &Self::VTABLE as *const ISerializationOutputVTable
                        as *const ::neocogi_com::IUnknownVTable,
                )
            },
            &[],
        );
        /// Table for a standalone projection of this interface.
        pub const VTABLE: ISerializationOutputVTable = Self::with_identity(
            &<ISerializationOutputAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
        );
        /// Builds an inherited prefix advertising the actual outer projection.
        pub const fn with_identity(
            __nc_identity: &'static ::neocogi_com::NcInterfaceId,
        ) -> ISerializationOutputVTable {
            ISerializationOutputVTable {
                unknown: ::neocogi_com::IUnknownVTable {
                    interface_id: __nc_identity,
                    query_interface: ::neocogi_com::lifetime::nc_query_interface,
                    add_ref: ::neocogi_com::lifetime::nc_strong_retain,
                    release: ::neocogi_com::lifetime::nc_strong_release,
                    downgrade: ::core::option::Option::Some(
                        ::neocogi_com::lifetime::nc_downgrade,
                    ),
                },
                byte_len: Self::byte_len_thunk,
                copy_to: Self::copy_to_thunk,
            }
        }
        /// Reports the complete document length after successful finish.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:118:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn byte_len_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
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
            let __nc_result = <__NcProvider as ISerializationOutput>::byte_len(
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
        /// Copies the complete document; insufficient capacity publishes no prefix.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:120:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn copy_to_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_offset: ::core::primitive::u64,
            __nc_arg_0_len: ::core::primitive::u64,
            __nc_arg_0_ptr: *mut <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
            __nc_arg_0_written: *mut ::core::primitive::u64,
            __nc_out: *mut <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            if __nc_arg_0_written.is_null()
                || __nc_arg_0_written
                    .align_offset(::core::mem::align_of::<::core::primitive::u64>()) != 0
            {
                return ::neocogi_com::NC_E_POINTER;
            }
            unsafe {
                *__nc_arg_0_written = 0;
            }
            let mut __nc_arg_0_values = 0_u64;
            let __nc_arg_0 = match unsafe {
                ::neocogi_com::NcOutputWriter::<
                    ::core::primitive::u8,
                >::from_raw(
                    __nc_arg_0_offset,
                    __nc_arg_0_ptr,
                    __nc_arg_0_len,
                    &mut __nc_arg_0_values,
                )
            } {
                ::core::result::Result::Ok(__nc_writer) => __nc_writer,
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
            let __nc_result = <__NcProvider as ISerializationOutput>::copy_to(
                &*__nc_dispatch_value,
                __nc_arg_0,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(__nc_value) => {
                    let __nc_raw = match ::neocogi_com::encode_inline(&__nc_value) {
                        ::core::result::Result::Ok(__nc_raw) => __nc_raw,
                        ::core::result::Result::Err(__nc_status) => return __nc_status,
                    };
                    unsafe {
                        *__nc_arg_0_written = __nc_arg_0_values;
                    }
                    unsafe {
                        *__nc_out = __nc_raw;
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
pub struct ISerializationPluginVtblFor<__NcProvider>(
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
        identity as __nc_identity, identity as __nc_this_0, identity as __nc_status_0,
        identity as __nc_result_0, identity as __nc_prepared, identity as __nc_out_0,
        identity as __nc_this_1, identity as __nc_status_1, identity as __nc_result_1,
        identity as __nc_prepared_0, identity as __nc_out_1,
    };
    impl<__NcProvider> ISerializationPluginVtblFor<__NcProvider>
    where
        __NcProvider: ISerializationPlugin + ::neocogi_com::ComObject,
    {
        /// Distinct provider bindings in the interface's canonical metadata order.
        /// T fixes the thunk implementation; no object headers are stored here.
        pub const PROJECTIONS: [::neocogi_com::__ComProjection<
            __NcProvider,
        >; ISerializationPluginAbi::INTERFACE_CHAIN.len()] = ::neocogi_com::__nc_projection_table(
            &ISerializationPluginAbi::INTERFACE_CHAIN,
            unsafe {
                ::neocogi_com::__ComProjection::new(
                    <ISerializationPluginAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
                    &Self::VTABLE as *const ISerializationPluginVTable
                        as *const ::neocogi_com::IUnknownVTable,
                )
            },
            &[],
        );
        /// Table for a standalone projection of this interface.
        pub const VTABLE: ISerializationPluginVTable = Self::with_identity(
            &<ISerializationPluginAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
        );
        /// Builds an inherited prefix advertising the actual outer projection.
        pub const fn with_identity(
            __nc_identity: &'static ::neocogi_com::NcInterfaceId,
        ) -> ISerializationPluginVTable {
            ISerializationPluginVTable {
                unknown: ::neocogi_com::IUnknownVTable {
                    interface_id: __nc_identity,
                    query_interface: ::neocogi_com::lifetime::nc_query_interface,
                    add_ref: ::neocogi_com::lifetime::nc_strong_retain,
                    release: ::neocogi_com::lifetime::nc_strong_release,
                    downgrade: ::core::option::Option::Some(
                        ::neocogi_com::lifetime::nc_downgrade,
                    ),
                },
                default_format: Self::default_format_thunk,
                format: Self::format_thunk,
            }
        }
        /// Returns the unconditional default NCB format factory.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:134:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn default_format_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_out: *mut ::neocogi_com::ComRaw<self::ISerializationFormatAbi>,
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
            let __nc_result = <__NcProvider as ISerializationPlugin>::default_format(
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
        /// Selects ncb or json; disabled and unknown names return NC_E_NOINTERFACE.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:136:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn format_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_offset: ::core::primitive::u64,
            __nc_arg_0_len: ::core::primitive::u64,
            __nc_arg_0_ptr: *const <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
            __nc_out: *mut ::neocogi_com::ComRaw<self::ISerializationFormatAbi>,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() || __nc_out.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_values = match unsafe {
                ::neocogi_com::DecodedInput::<
                    ::core::primitive::u8,
                >::from_raw(__nc_arg_0_ptr, __nc_arg_0_len)
            } {
                ::core::result::Result::Ok(__nc_values) => __nc_values,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_0 = ::neocogi_com::NcCopyFromBufferOffset::new(
                __nc_arg_0_offset,
                __nc_arg_0_values.as_slice(),
            );
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_ref(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializationPlugin>::format(
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
pub struct ISerializerVtblFor<__NcProvider>(
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
        identity as __nc_arg_0_written, identity as __nc_arg_0_0,
        identity as __nc_arg_0_0_owner_slot, identity as __nc_arg_0_0_offset,
        identity as __nc_arg_0_0_len, identity as __nc_arg_0_0_ptr,
        identity as __nc_arg_0_0_values, identity as __nc_arg_0_0_written,
        identity as __nc_arg_0_1, identity as __nc_arg_0_1_owner_slot,
        identity as __nc_arg_0_1_offset, identity as __nc_arg_0_1_len,
        identity as __nc_arg_0_1_ptr, identity as __nc_arg_0_1_values,
        identity as __nc_arg_0_1_written, identity as __nc_arg_0_2,
        identity as __nc_arg_0_2_owner_slot, identity as __nc_arg_0_2_offset,
        identity as __nc_arg_0_2_len, identity as __nc_arg_0_2_ptr,
        identity as __nc_arg_0_2_values, identity as __nc_arg_0_2_written,
        identity as __nc_arg_0_3, identity as __nc_arg_0_3_owner_slot,
        identity as __nc_arg_0_3_offset, identity as __nc_arg_0_3_len,
        identity as __nc_arg_0_3_ptr, identity as __nc_arg_0_3_values,
        identity as __nc_arg_0_3_written, identity as __nc_arg_0_4,
        identity as __nc_arg_0_4_owner_slot, identity as __nc_arg_0_4_offset,
        identity as __nc_arg_0_4_len, identity as __nc_arg_0_4_ptr,
        identity as __nc_arg_0_4_values, identity as __nc_arg_0_4_written,
        identity as __nc_arg_0_5, identity as __nc_arg_0_5_owner_slot,
        identity as __nc_arg_0_5_offset, identity as __nc_arg_0_5_len,
        identity as __nc_arg_0_5_ptr, identity as __nc_arg_0_5_values,
        identity as __nc_arg_0_5_written, identity as __nc_arg_0_6,
        identity as __nc_arg_0_6_owner_slot, identity as __nc_arg_0_6_offset,
        identity as __nc_arg_0_6_len, identity as __nc_arg_0_6_ptr,
        identity as __nc_arg_0_6_values, identity as __nc_arg_0_6_written,
        identity as __nc_arg_0_7, identity as __nc_arg_0_7_owner_slot,
        identity as __nc_arg_0_7_offset, identity as __nc_arg_0_7_len,
        identity as __nc_arg_0_7_ptr, identity as __nc_arg_0_7_values,
        identity as __nc_arg_0_7_written, identity as __nc_arg_0_8,
        identity as __nc_arg_0_8_owner_slot, identity as __nc_arg_0_8_offset,
        identity as __nc_arg_0_8_len, identity as __nc_arg_0_8_ptr,
        identity as __nc_arg_0_8_values, identity as __nc_arg_0_8_written,
        identity as __nc_arg_0_9, identity as __nc_arg_0_9_owner_slot,
        identity as __nc_arg_0_9_offset, identity as __nc_arg_0_9_len,
        identity as __nc_arg_0_9_ptr, identity as __nc_arg_0_9_values,
        identity as __nc_arg_0_9_written, identity as __nc_arg_0_10,
        identity as __nc_arg_0_10_owner_slot, identity as __nc_arg_0_10_offset,
        identity as __nc_arg_0_10_len, identity as __nc_arg_0_10_ptr,
        identity as __nc_arg_0_10_values, identity as __nc_arg_0_10_written,
        identity as __nc_arg_0_11, identity as __nc_arg_0_11_owner_slot,
        identity as __nc_arg_0_11_offset, identity as __nc_arg_0_11_len,
        identity as __nc_arg_0_11_ptr, identity as __nc_arg_0_11_values,
        identity as __nc_arg_0_11_written, identity as __nc_arg_0_12,
        identity as __nc_arg_0_12_owner_slot, identity as __nc_arg_0_12_offset,
        identity as __nc_arg_0_12_len, identity as __nc_arg_0_12_ptr,
        identity as __nc_arg_0_12_values, identity as __nc_arg_0_12_written,
        identity as __nc_arg_0_13, identity as __nc_arg_0_13_owner_slot,
        identity as __nc_arg_0_13_offset, identity as __nc_arg_0_13_len,
        identity as __nc_arg_0_13_ptr, identity as __nc_arg_0_13_values,
        identity as __nc_arg_0_13_written, identity as __nc_arg_0_14,
        identity as __nc_arg_0_14_owner_slot, identity as __nc_arg_0_14_offset,
        identity as __nc_arg_0_14_len, identity as __nc_arg_0_14_ptr,
        identity as __nc_arg_0_14_values, identity as __nc_arg_0_14_written,
        identity as __nc_arg_0_15, identity as __nc_arg_0_15_owner_slot,
        identity as __nc_arg_0_15_offset, identity as __nc_arg_0_15_len,
        identity as __nc_arg_0_15_ptr, identity as __nc_arg_0_15_values,
        identity as __nc_arg_0_15_written, identity as __nc_arg_0_16,
        identity as __nc_arg_0_16_owner_slot, identity as __nc_arg_0_16_offset,
        identity as __nc_arg_0_16_len, identity as __nc_arg_0_16_ptr,
        identity as __nc_arg_0_16_values, identity as __nc_arg_0_16_written,
        identity as __nc_arg_1, identity as __nc_arg_1_owner_slot,
        identity as __nc_arg_1_offset, identity as __nc_arg_1_len,
        identity as __nc_arg_1_ptr, identity as __nc_arg_1_values,
        identity as __nc_arg_1_written, identity as __nc_arg_0_17,
        identity as __nc_arg_0_17_owner_slot, identity as __nc_arg_0_17_offset,
        identity as __nc_arg_0_17_len, identity as __nc_arg_0_17_ptr,
        identity as __nc_arg_0_17_values, identity as __nc_arg_0_17_written,
        identity as __nc_this, identity as __nc_status, identity as __nc_value,
        identity as __nc_raw, identity as __nc_result, identity as __nc_values,
        identity as __nc_writer, identity as __nc_out, identity as __nc_identity,
        identity as __nc_this_0, identity as __nc_status_0, identity as __nc_result_0,
        identity as __nc_prepared, identity as __nc_out_0, identity as __nc_this_1,
        identity as __nc_status_1, identity as __nc_result_1,
        identity as __nc_prepared_0, identity as __nc_out_1, identity as __nc_this_2,
        identity as __nc_status_2, identity as __nc_result_2,
        identity as __nc_prepared_1, identity as __nc_out_2, identity as __nc_this_3,
        identity as __nc_status_3, identity as __nc_result_3,
        identity as __nc_prepared_2, identity as __nc_out_3, identity as __nc_this_4,
        identity as __nc_status_4, identity as __nc_result_4,
        identity as __nc_prepared_3, identity as __nc_out_4, identity as __nc_this_5,
        identity as __nc_status_5, identity as __nc_result_5,
        identity as __nc_prepared_4, identity as __nc_out_5, identity as __nc_this_6,
        identity as __nc_status_6, identity as __nc_result_6,
        identity as __nc_prepared_5, identity as __nc_out_6, identity as __nc_this_7,
        identity as __nc_status_7, identity as __nc_result_7,
        identity as __nc_prepared_6, identity as __nc_out_7, identity as __nc_this_8,
        identity as __nc_status_8, identity as __nc_result_8,
        identity as __nc_prepared_7, identity as __nc_out_8, identity as __nc_this_9,
        identity as __nc_status_9, identity as __nc_result_9,
        identity as __nc_prepared_8, identity as __nc_out_9, identity as __nc_this_10,
        identity as __nc_status_10, identity as __nc_result_10,
        identity as __nc_prepared_9, identity as __nc_out_10, identity as __nc_this_11,
        identity as __nc_status_11, identity as __nc_result_11,
        identity as __nc_prepared_10, identity as __nc_out_11, identity as __nc_this_12,
        identity as __nc_status_12, identity as __nc_result_12,
        identity as __nc_prepared_11, identity as __nc_out_12, identity as __nc_this_13,
        identity as __nc_status_13, identity as __nc_result_13,
        identity as __nc_prepared_12, identity as __nc_out_13, identity as __nc_this_14,
        identity as __nc_status_14, identity as __nc_result_14,
        identity as __nc_prepared_13, identity as __nc_out_14, identity as __nc_this_15,
        identity as __nc_status_15, identity as __nc_result_15,
        identity as __nc_prepared_14, identity as __nc_out_15, identity as __nc_this_16,
        identity as __nc_status_16, identity as __nc_result_16,
        identity as __nc_prepared_15, identity as __nc_out_16, identity as __nc_this_17,
        identity as __nc_status_17, identity as __nc_result_17,
        identity as __nc_prepared_16, identity as __nc_out_17, identity as __nc_this_18,
        identity as __nc_status_18, identity as __nc_result_18,
        identity as __nc_prepared_17, identity as __nc_out_18, identity as __nc_this_19,
        identity as __nc_status_19, identity as __nc_result_19,
        identity as __nc_prepared_18, identity as __nc_out_19, identity as __nc_this_20,
        identity as __nc_status_20, identity as __nc_result_20,
        identity as __nc_prepared_19, identity as __nc_out_20, identity as __nc_this_21,
        identity as __nc_status_21, identity as __nc_result_21,
        identity as __nc_prepared_20, identity as __nc_out_21, identity as __nc_this_22,
        identity as __nc_status_22, identity as __nc_result_22,
        identity as __nc_prepared_21, identity as __nc_out_22, identity as __nc_this_23,
        identity as __nc_status_23, identity as __nc_result_23,
        identity as __nc_prepared_22, identity as __nc_out_23, identity as __nc_this_24,
        identity as __nc_status_24, identity as __nc_result_24,
        identity as __nc_prepared_23, identity as __nc_out_24, identity as __nc_this_25,
        identity as __nc_status_25, identity as __nc_result_25,
        identity as __nc_prepared_24, identity as __nc_out_25,
    };
    impl<__NcProvider> ISerializerVtblFor<__NcProvider>
    where
        __NcProvider: ISerializer + ::neocogi_com::ComObject,
    {
        /// Distinct provider bindings in the interface's canonical metadata order.
        /// T fixes the thunk implementation; no object headers are stored here.
        pub const PROJECTIONS: [::neocogi_com::__ComProjection<
            __NcProvider,
        >; ISerializerAbi::INTERFACE_CHAIN.len()] = ::neocogi_com::__nc_projection_table(
            &ISerializerAbi::INTERFACE_CHAIN,
            unsafe {
                ::neocogi_com::__ComProjection::new(
                    <ISerializerAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
                    &Self::VTABLE as *const ISerializerVTable
                        as *const ::neocogi_com::IUnknownVTable,
                )
            },
            &[],
        );
        /// Table for a standalone projection of this interface.
        pub const VTABLE: ISerializerVTable = Self::with_identity(
            &<ISerializerAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
        );
        /// Builds an inherited prefix advertising the actual outer projection.
        pub const fn with_identity(
            __nc_identity: &'static ::neocogi_com::NcInterfaceId,
        ) -> ISerializerVTable {
            ISerializerVTable {
                unknown: ::neocogi_com::IUnknownVTable {
                    interface_id: __nc_identity,
                    query_interface: ::neocogi_com::lifetime::nc_query_interface,
                    add_ref: ::neocogi_com::lifetime::nc_strong_retain,
                    release: ::neocogi_com::lifetime::nc_strong_release,
                    downgrade: ::core::option::Option::Some(
                        ::neocogi_com::lifetime::nc_downgrade,
                    ),
                },
                serialize_unit: Self::serialize_unit_thunk,
                serialize_bool: Self::serialize_bool_thunk,
                serialize_u8: Self::serialize_u8_thunk,
                serialize_u16: Self::serialize_u16_thunk,
                serialize_u32: Self::serialize_u32_thunk,
                serialize_u64: Self::serialize_u64_thunk,
                serialize_i8: Self::serialize_i8_thunk,
                serialize_i16: Self::serialize_i16_thunk,
                serialize_i32: Self::serialize_i32_thunk,
                serialize_i64: Self::serialize_i64_thunk,
                serialize_f32: Self::serialize_f32_thunk,
                serialize_f64: Self::serialize_f64_thunk,
                serialize_string: Self::serialize_string_thunk,
                begin_option: Self::begin_option_thunk,
                end_option: Self::end_option_thunk,
                begin_record: Self::begin_record_thunk,
                field: Self::field_thunk,
                end_record: Self::end_record_thunk,
                begin_sequence: Self::begin_sequence_thunk,
                end_sequence: Self::end_sequence_thunk,
                begin_map: Self::begin_map_thunk,
                end_map: Self::end_map_thunk,
                begin_variant: Self::begin_variant_thunk,
                end_variant: Self::end_variant_thunk,
                fail: Self::fail_thunk,
                finish: Self::finish_thunk,
            }
        }
        /// Writes a unit value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:4:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn serialize_unit_thunk(
            __nc_this: *mut ::core::ffi::c_void,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::serialize_unit(
                &mut *__nc_dispatch_value,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Writes a bool value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:6:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn serialize_bool_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0: <::core::primitive::bool as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::bool>(__nc_arg_0)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::serialize_bool(
                &mut *__nc_dispatch_value,
                __nc_arg_0,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Writes a u8 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:8:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn serialize_u8_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_0: <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_0 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::u8>(__nc_arg_0_0)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::serialize_u8(
                &mut *__nc_dispatch_value,
                __nc_arg_0_0,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Writes a u16 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:10:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn serialize_u16_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_1: <::core::primitive::u16 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_1 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::u16>(__nc_arg_0_1)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::serialize_u16(
                &mut *__nc_dispatch_value,
                __nc_arg_0_1,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Writes a u32 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:12:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn serialize_u32_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_2: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_2 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::u32>(__nc_arg_0_2)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::serialize_u32(
                &mut *__nc_dispatch_value,
                __nc_arg_0_2,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Writes a u64 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:14:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn serialize_u64_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_3: <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_3 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::u64>(__nc_arg_0_3)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::serialize_u64(
                &mut *__nc_dispatch_value,
                __nc_arg_0_3,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Writes a i8 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:16:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn serialize_i8_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_4: <::core::primitive::i8 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_4 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::i8>(__nc_arg_0_4)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::serialize_i8(
                &mut *__nc_dispatch_value,
                __nc_arg_0_4,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Writes a i16 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:18:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn serialize_i16_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_5: <::core::primitive::i16 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_5 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::i16>(__nc_arg_0_5)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::serialize_i16(
                &mut *__nc_dispatch_value,
                __nc_arg_0_5,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Writes a i32 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:20:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn serialize_i32_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_6: <::core::primitive::i32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_6 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::i32>(__nc_arg_0_6)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::serialize_i32(
                &mut *__nc_dispatch_value,
                __nc_arg_0_6,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Writes a i64 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:22:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn serialize_i64_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_7: <::core::primitive::i64 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_7 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::i64>(__nc_arg_0_7)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::serialize_i64(
                &mut *__nc_dispatch_value,
                __nc_arg_0_7,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Writes a f32 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:24:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn serialize_f32_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_8: <::core::primitive::f32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_8 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::f32>(__nc_arg_0_8)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::serialize_f32(
                &mut *__nc_dispatch_value,
                __nc_arg_0_8,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Writes a f64 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:26:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn serialize_f64_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_9: <::core::primitive::f64 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_9 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::f64>(__nc_arg_0_9)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::serialize_f64(
                &mut *__nc_dispatch_value,
                __nc_arg_0_9,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Copies one UTF-8 string during this call.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:28:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn serialize_string_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_10_offset: ::core::primitive::u64,
            __nc_arg_0_10_len: ::core::primitive::u64,
            __nc_arg_0_10_ptr: *const <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_10_values = match unsafe {
                ::neocogi_com::DecodedInput::<
                    ::core::primitive::u8,
                >::from_raw(__nc_arg_0_10_ptr, __nc_arg_0_10_len)
            } {
                ::core::result::Result::Ok(__nc_values) => __nc_values,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_0_10 = ::neocogi_com::NcCopyFromBufferOffset::new(
                __nc_arg_0_10_offset,
                __nc_arg_0_10_values.as_slice(),
            );
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::serialize_string(
                &mut *__nc_dispatch_value,
                __nc_arg_0_10,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Opens optional presence; the present child follows before end_option.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:30:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn begin_option_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_11: <::core::primitive::bool as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_11 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::bool>(__nc_arg_0_11)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::begin_option(
                &mut *__nc_dispatch_value,
                __nc_arg_0_11,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Completes the optional value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:32:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn end_option_thunk(
            __nc_this: *mut ::core::ffi::c_void,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::end_option(
                &mut *__nc_dispatch_value,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Opens a named record containing exactly fields members.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:34:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn begin_record_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_12: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_12 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::u32>(__nc_arg_0_12)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::begin_record(
                &mut *__nc_dispatch_value,
                __nc_arg_0_12,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Selects the next named field before writing its typed value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:36:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn field_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_13_offset: ::core::primitive::u64,
            __nc_arg_0_13_len: ::core::primitive::u64,
            __nc_arg_0_13_ptr: *const <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_13_values = match unsafe {
                ::neocogi_com::DecodedInput::<
                    ::core::primitive::u8,
                >::from_raw(__nc_arg_0_13_ptr, __nc_arg_0_13_len)
            } {
                ::core::result::Result::Ok(__nc_values) => __nc_values,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_0_13 = ::neocogi_com::NcCopyFromBufferOffset::new(
                __nc_arg_0_13_offset,
                __nc_arg_0_13_values.as_slice(),
            );
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::field(
                &mut *__nc_dispatch_value,
                __nc_arg_0_13,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Completes the record.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:38:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn end_record_thunk(
            __nc_this: *mut ::core::ffi::c_void,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::end_record(
                &mut *__nc_dispatch_value,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Opens an ordered collection with its complete element count.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:40:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn begin_sequence_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_14: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_14 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::u32>(__nc_arg_0_14)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::begin_sequence(
                &mut *__nc_dispatch_value,
                __nc_arg_0_14,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Completes the collection.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:42:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn end_sequence_thunk(
            __nc_this: *mut ::core::ffi::c_void,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::end_sequence(
                &mut *__nc_dispatch_value,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Opens a map; keys and values alternate for exactly pairs entries.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:44:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn begin_map_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_15: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_15 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::u32>(__nc_arg_0_15)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::begin_map(
                &mut *__nc_dispatch_value,
                __nc_arg_0_15,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Completes the map.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:46:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn end_map_thunk(
            __nc_this: *mut ::core::ffi::c_void,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::end_map(
                &mut *__nc_dispatch_value,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Opens a case identified by both declaration ordinal and authored name.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:48:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn begin_variant_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_16: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
            __nc_arg_1_offset: ::core::primitive::u64,
            __nc_arg_1_len: ::core::primitive::u64,
            __nc_arg_1_ptr: *const <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_16 = match unsafe {
                ::neocogi_com::decode_inline::<::core::primitive::u32>(__nc_arg_0_16)
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_1_values = match unsafe {
                ::neocogi_com::DecodedInput::<
                    ::core::primitive::u8,
                >::from_raw(__nc_arg_1_ptr, __nc_arg_1_len)
            } {
                ::core::result::Result::Ok(__nc_values) => __nc_values,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_1 = ::neocogi_com::NcCopyFromBufferOffset::new(
                __nc_arg_1_offset,
                __nc_arg_1_values.as_slice(),
            );
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::begin_variant(
                &mut *__nc_dispatch_value,
                __nc_arg_0_16,
                __nc_arg_1,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Completes the selected case and its optional payload.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:50:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn end_variant_thunk(
            __nc_this: *mut ::core::ffi::c_void,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::end_variant(
                &mut *__nc_dispatch_value,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Marks a schema failure; no later operation may succeed.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:52:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn fail_thunk(
            __nc_this: *mut ::core::ffi::c_void,
            __nc_arg_0_17_offset: ::core::primitive::u64,
            __nc_arg_0_17_len: ::core::primitive::u64,
            __nc_arg_0_17_ptr: *const <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_arg_0_17_values = match unsafe {
                ::neocogi_com::DecodedInput::<
                    ::core::primitive::u8,
                >::from_raw(__nc_arg_0_17_ptr, __nc_arg_0_17_len)
            } {
                ::core::result::Result::Ok(__nc_values) => __nc_values,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_arg_0_17 = ::neocogi_com::NcCopyFromBufferOffset::new(
                __nc_arg_0_17_offset,
                __nc_arg_0_17_values.as_slice(),
            );
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::fail(
                &mut *__nc_dispatch_value,
                __nc_arg_0_17,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
        /// Commits a complete document; repeated finish and further writes fail.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:54:8` (module `neocogi.com.serialization`).
        /// Validates raw pointers, borrows the concrete provider through its
        /// object-wide gate, invokes its typed method, and lowers the result.
        /// The gate is released before transferring an owned result; errors
        /// are normalized so an unwritten output can never look successful.
        ///
        /// # Safety
        /// The interface belongs to T's live generated storage. Non-null ABI
        /// arguments must satisfy their method's readable/writable contracts.
        unsafe extern "C" fn finish_thunk(
            __nc_this: *mut ::core::ffi::c_void,
        ) -> ::neocogi_com::NcStatus {
            if __nc_this.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let __nc_dispatch_object = __nc_this;
            if __nc_dispatch_object.is_null() {
                return ::neocogi_com::NC_E_POINTER;
            }
            let mut __nc_dispatch_value = match unsafe {
                <__NcProvider as ::neocogi_com::ComObject>::borrow_abi_mut(
                    __nc_dispatch_object,
                )
            } {
                ::core::result::Result::Ok(__nc_value) => __nc_value,
                ::core::result::Result::Err(__nc_status) => return __nc_status,
            };
            let __nc_result = <__NcProvider as ISerializer>::finish(
                &mut *__nc_dispatch_value,
            );
            ::core::mem::drop(__nc_dispatch_value);
            match __nc_result {
                ::core::result::Result::Ok(()) => ::neocogi_com::NC_OK,
                ::core::result::Result::Err(__nc_status) => __nc_status.normalize_error(),
            }
        }
    }
};
