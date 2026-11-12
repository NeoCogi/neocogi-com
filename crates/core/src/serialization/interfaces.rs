// Generated from core/schemas/neocogi/com/serialization/formats.ncidl.
// Regenerate: NC_UPDATE_PERSISTENCE_BINDINGS=1 cargo test -p neocogi-idl serialization_bindings_match_idl

/// Stable logical identity of this generated contract module.
pub const MODULE_ID: ::neocogi_com::NcGuid = ::neocogi_com::NcGuid::from_fields(
    3124728653u32,
    64204u16,
    23363u16,
    [130u8, 201u8, 97u8, 37u8, 55u8, 221u8, 57u8, 31u8],
);
/// Transitive canonical ABI fingerprint of this generated module.
pub const ABI_FINGERPRINT: [::core::primitive::u8; 32] = [
    52u8, 178u8, 151u8, 181u8, 67u8, 79u8, 205u8, 215u8, 57u8, 48u8, 191u8, 186u8, 78u8,
    133u8, 210u8, 219u8, 209u8, 129u8, 152u8, 237u8, 51u8, 63u8, 197u8, 134u8, 189u8,
    120u8, 200u8, 90u8, 217u8, 161u8, 136u8, 213u8,
];
/// Exact dotted module name derived from the normalized IDL path.
pub const MODULE_NAME: &::core::primitive::str = "neocogi.com.serialization";
/// A mutable typed source; each value scope constructs one caller-owned value.
/// IDL source: `neocogi/com/serialization/formats.ncidl:58:11` (module `neocogi.com.serialization`).
pub trait IDeserializer {
    /// Opens one typed value, charging nesting and construction limits.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:60:8` (module `neocogi.com.serialization`).
    fn enter_value(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Completes the current typed value after its children have been consumed.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:62:8` (module `neocogi.com.serialization`).
    fn leave_value(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Reads a unit value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:64:8` (module `neocogi.com.serialization`).
    fn deserialize_unit(
        &mut self,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Acquires one validated bool value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:66:8` (module `neocogi.com.serialization`).
    fn deserialize_bool(
        &mut self,
    ) -> ::core::result::Result<::core::primitive::bool, ::neocogi_com::NcStatus>;
    /// Acquires one validated u8 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:68:8` (module `neocogi.com.serialization`).
    fn deserialize_u8(
        &mut self,
    ) -> ::core::result::Result<::core::primitive::u8, ::neocogi_com::NcStatus>;
    /// Acquires one validated u16 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:70:8` (module `neocogi.com.serialization`).
    fn deserialize_u16(
        &mut self,
    ) -> ::core::result::Result<::core::primitive::u16, ::neocogi_com::NcStatus>;
    /// Acquires one validated u32 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:72:8` (module `neocogi.com.serialization`).
    fn deserialize_u32(
        &mut self,
    ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus>;
    /// Acquires one validated u64 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:74:8` (module `neocogi.com.serialization`).
    fn deserialize_u64(
        &mut self,
    ) -> ::core::result::Result<::core::primitive::u64, ::neocogi_com::NcStatus>;
    /// Acquires one validated i8 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:76:8` (module `neocogi.com.serialization`).
    fn deserialize_i8(
        &mut self,
    ) -> ::core::result::Result<::core::primitive::i8, ::neocogi_com::NcStatus>;
    /// Acquires one validated i16 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:78:8` (module `neocogi.com.serialization`).
    fn deserialize_i16(
        &mut self,
    ) -> ::core::result::Result<::core::primitive::i16, ::neocogi_com::NcStatus>;
    /// Acquires one validated i32 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:80:8` (module `neocogi.com.serialization`).
    fn deserialize_i32(
        &mut self,
    ) -> ::core::result::Result<::core::primitive::i32, ::neocogi_com::NcStatus>;
    /// Acquires one validated i64 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:82:8` (module `neocogi.com.serialization`).
    fn deserialize_i64(
        &mut self,
    ) -> ::core::result::Result<::core::primitive::i64, ::neocogi_com::NcStatus>;
    /// Acquires one validated f32 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:84:8` (module `neocogi.com.serialization`).
    fn deserialize_f32(
        &mut self,
    ) -> ::core::result::Result<::core::primitive::f32, ::neocogi_com::NcStatus>;
    /// Acquires one validated f64 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:86:8` (module `neocogi.com.serialization`).
    fn deserialize_f64(
        &mut self,
    ) -> ::core::result::Result<::core::primitive::f64, ::neocogi_com::NcStatus>;
    /// Admits a complete UTF-8 string and reports its byte length before allocation.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:88:8` (module `neocogi.com.serialization`).
    fn string_len(
        &mut self,
    ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus>;
    /// Copies exactly the admitted string into fresh caller-owned output.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:90:8` (module `neocogi.com.serialization`).
    fn copy_string(
        &mut self,
        output: ::neocogi_com::NcOutputWriter<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus>;
    /// Reads optional presence; a present child follows within this value scope.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:92:8` (module `neocogi.com.serialization`).
    fn option_present(
        &mut self,
    ) -> ::core::result::Result<::core::primitive::bool, ::neocogi_com::NcStatus>;
    /// Opens a record and rejects more than maximum fields.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:94:8` (module `neocogi.com.serialization`).
    fn begin_record(
        &mut self,
        maximum: ::core::primitive::u32,
    ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus>;
    /// Selects a named field in schema order; false means genuinely absent.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:96:8` (module `neocogi.com.serialization`).
    fn field(
        &mut self,
        name: ::neocogi_com::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<::core::primitive::bool, ::neocogi_com::NcStatus>;
    /// Completes the record, rejecting unconsumed or unknown fields.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:98:8` (module `neocogi.com.serialization`).
    fn end_record(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Opens an ordered collection and reports its bounded element count.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:100:8` (module `neocogi.com.serialization`).
    fn begin_sequence(
        &mut self,
    ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus>;
    /// Completes the collection, rejecting unconsumed elements.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:102:8` (module `neocogi.com.serialization`).
    fn end_sequence(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Opens a map and reports its bounded pair count.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:104:8` (module `neocogi.com.serialization`).
    fn begin_map(
        &mut self,
    ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus>;
    /// Completes the map.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:106:8` (module `neocogi.com.serialization`).
    fn end_map(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Probes cases in declaration order; a match selects the following payload.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:108:8` (module `neocogi.com.serialization`).
    fn variant_is(
        &mut self,
        index: ::core::primitive::u32,
        name: ::neocogi_com::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<::core::primitive::bool, ::neocogi_com::NcStatus>;
    /// Marks a schema failure; no later operation may succeed.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:110:8` (module `neocogi.com.serialization`).
    fn fail(
        &mut self,
        message: ::neocogi_com::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Requires a complete root with no trailing input and closes the session.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:112:8` (module `neocogi.com.serialization`).
    fn finish(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
}
///Interface marker for [`IDeserializer`]; handles carry its object/table pair.
pub enum IDeserializerAbi {}
#[repr(C)]
///C-compatible vtable for [`IDeserializer`].
pub struct IDeserializerVTable {
    /// Common identity, query and reference-count slots.
    pub unknown: ::neocogi_com::IUnknownVTable,
    /// Opens one typed value, charging nesting and construction limits.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:60:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::enter_value`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub enter_value: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
    ) -> ::neocogi_com::NcStatus,
    /// Completes the current typed value after its children have been consumed.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:62:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::leave_value`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub leave_value: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
    ) -> ::neocogi_com::NcStatus,
    /// Reads a unit value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:64:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::deserialize_unit`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub deserialize_unit: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
    ) -> ::neocogi_com::NcStatus,
    /// Acquires one validated bool value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:66:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::deserialize_bool`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: bool as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub deserialize_bool: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::bool as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Acquires one validated u8 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:68:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::deserialize_u8`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u8 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub deserialize_u8: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Acquires one validated u16 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:70:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::deserialize_u16`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u16 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub deserialize_u16: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::u16 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Acquires one validated u32 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:72:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::deserialize_u32`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub deserialize_u32: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Acquires one validated u64 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:74:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::deserialize_u64`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u64 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub deserialize_u64: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Acquires one validated i8 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:76:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::deserialize_i8`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: i8 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub deserialize_i8: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::i8 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Acquires one validated i16 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:78:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::deserialize_i16`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: i16 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub deserialize_i16: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::i16 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Acquires one validated i32 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:80:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::deserialize_i32`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: i32 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub deserialize_i32: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::i32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Acquires one validated i64 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:82:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::deserialize_i64`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: i64 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub deserialize_i64: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::i64 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Acquires one validated f32 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:84:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::deserialize_f32`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: f32 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub deserialize_f32: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::f32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Acquires one validated f64 value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:86:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::deserialize_f64`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: f64 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub deserialize_f64: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::f64 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Admits a complete UTF-8 string and reports its byte length before allocation.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:88:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::string_len`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub string_len: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Copies exactly the admitted string into fresh caller-owned output.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:90:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::copy_string`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `output` uses ABI storage `* mut < :: core :: primitive :: u8 as :: neocogi_com :: ComValue > :: Wire` for `:: neocogi_com :: NcOutputWriter < '_ , :: core :: primitive :: u8 >`. The writable buffer pointer is accompanied by `offset: u64`, `capacity: u64`, and a separate `written: *mut u64`. Old contents are not input. Only the successfully reported initialized prefix may be read; count and element storage must not alias.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub copy_string: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_offset: ::core::primitive::u64,
        __nc_arg_0_len: ::core::primitive::u64,
        __nc_arg_0_ptr: *mut <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_0_written: *mut ::core::primitive::u64,
        __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Reads optional presence; a present child follows within this value scope.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:92:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::option_present`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: bool as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub option_present: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::bool as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Opens a record and rejects more than maximum fields.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:94:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::begin_record`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `maximum` uses ABI storage `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: u32`. Passed by value.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub begin_record: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_0: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
        __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Selects a named field in schema order; false means genuinely absent.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:96:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::field`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `name` uses ABI storage `* const < :: core :: primitive :: u8 as :: neocogi_com :: ComValue > :: Wire` for `:: neocogi_com :: NcCopyFromBufferOffset < '_ , :: core :: primitive :: u8 >`. The readable buffer pointer is accompanied by `offset: u64` and `len: u64`.
    ///
    ///On success, `out` receives `< :: core :: primitive :: bool as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub field: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_1_offset: ::core::primitive::u64,
        __nc_arg_0_1_len: ::core::primitive::u64,
        __nc_arg_0_1_ptr: *const <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
        __nc_out: *mut <::core::primitive::bool as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Completes the record, rejecting unconsumed or unknown fields.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:98:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::end_record`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub end_record: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
    ) -> ::neocogi_com::NcStatus,
    /// Opens an ordered collection and reports its bounded element count.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:100:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::begin_sequence`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub begin_sequence: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Completes the collection, rejecting unconsumed elements.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:102:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::end_sequence`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub end_sequence: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
    ) -> ::neocogi_com::NcStatus,
    /// Opens a map and reports its bounded pair count.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:104:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::begin_map`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub begin_map: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Completes the map.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:106:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::end_map`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub end_map: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
    ) -> ::neocogi_com::NcStatus,
    /// Probes cases in declaration order; a match selects the following payload.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:108:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::variant_is`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `index` uses ABI storage `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: u32`. Passed by value.
    ///- `name` uses ABI storage `* const < :: core :: primitive :: u8 as :: neocogi_com :: ComValue > :: Wire` for `:: neocogi_com :: NcCopyFromBufferOffset < '_ , :: core :: primitive :: u8 >`. The readable buffer pointer is accompanied by `offset: u64` and `len: u64`.
    ///
    ///On success, `out` receives `< :: core :: primitive :: bool as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub variant_is: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_2: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_1_offset: ::core::primitive::u64,
        __nc_arg_1_len: ::core::primitive::u64,
        __nc_arg_1_ptr: *const <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
        __nc_out: *mut <::core::primitive::bool as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Marks a schema failure; no later operation may succeed.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:110:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::fail`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `message` uses ABI storage `* const < :: core :: primitive :: u8 as :: neocogi_com :: ComValue > :: Wire` for `:: neocogi_com :: NcCopyFromBufferOffset < '_ , :: core :: primitive :: u8 >`. The readable buffer pointer is accompanied by `offset: u64` and `len: u64`.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub fail: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_3_offset: ::core::primitive::u64,
        __nc_arg_0_3_len: ::core::primitive::u64,
        __nc_arg_0_3_ptr: *const <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Requires a complete root with no trailing input and closes the session.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:112:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`IDeserializer::finish`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub finish: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
    ) -> ::neocogi_com::NcStatus,
}
unsafe impl ::neocogi_com::ComInterface for IDeserializerAbi {
    const IID: ::neocogi_com::NcGuid = ::neocogi_com::NcGuid {
        data1: 974314803u32,
        data2: 29229u16,
        data3: 22088u16,
        data4: [191u8, 77u8, 115u8, 178u8, 221u8, 193u8, 18u8, 230u8],
    };
    const INTERFACE_ID: ::neocogi_com::NcInterfaceId = ::neocogi_com::NcInterfaceId::new(
        ::neocogi_com::NcGuid {
            data1: 974314803u32,
            data2: 29229u16,
            data3: 22088u16,
            data4: [191u8, 77u8, 115u8, 178u8, 221u8, 193u8, 18u8, 230u8],
        },
        IDeserializerAbi::ABI_SIGNATURE,
    );
    type VTable = IDeserializerVTable;
    type Threading = ::neocogi_com::ThreadLocal;
}
#[repr(transparent)]
///Borrowed static method view for [`IDeserializer`].
pub struct IDeserializerClient<
    __NcThreading: ::neocogi_com::ThreadingModel = ::neocogi_com::ThreadLocal,
> {
    /// Certified transparent storage supplied only by a live COM pointer borrow.
    __nc_pointer: ::neocogi_com::NcClientPointer<IDeserializerAbi, __NcThreading>,
}
unsafe impl ::neocogi_com::ComClientInterface for IDeserializerAbi {
    type Client<__NcThreading: ::neocogi_com::ThreadingModel> = IDeserializerClient<
        __NcThreading,
    >;
    fn __nc_client_ref<__NcThreading: ::neocogi_com::ThreadingModel>(
        pointer: &::neocogi_com::NcClientPointer<Self, __NcThreading>,
    ) -> &Self::Client<__NcThreading> {
        unsafe {
            &*(::core::ptr::from_ref(pointer)
                as *const IDeserializerClient<__NcThreading>)
        }
    }
    fn __nc_client_mut<__NcThreading: ::neocogi_com::ThreadingModel>(
        pointer: &mut ::neocogi_com::NcClientPointer<Self, __NcThreading>,
    ) -> &mut Self::Client<__NcThreading> {
        unsafe {
            &mut *(::core::ptr::from_mut(pointer)
                as *mut IDeserializerClient<__NcThreading>)
        }
    }
}
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
    impl<
        __NcThreading: ::neocogi_com::ThreadingModel,
    > IDeserializerClient<__NcThreading> {
        /// Opens one typed value, charging nesting and construction limits.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:60:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::enter_value`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `()`.
        pub fn enter_value(
            &mut self,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_this_0 = self.__nc_pointer.as_raw();
            let __nc_status_0 = unsafe {
                ((*__nc_this_0.vtable).enter_value)(__nc_this_0.object)
            };
            if __nc_status_0.is_err() {
                return ::core::result::Result::Err(__nc_status_0);
            }
            let __nc_result_0 = ();
            ::core::result::Result::Ok(__nc_result_0)
        }
        /// Completes the current typed value after its children have been consumed.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:62:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::leave_value`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `()`.
        pub fn leave_value(
            &mut self,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_this_1 = self.__nc_pointer.as_raw();
            let __nc_status_1 = unsafe {
                ((*__nc_this_1.vtable).leave_value)(__nc_this_1.object)
            };
            if __nc_status_1.is_err() {
                return ::core::result::Result::Err(__nc_status_1);
            }
            let __nc_result_1 = ();
            ::core::result::Result::Ok(__nc_result_1)
        }
        /// Reads a unit value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:64:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::deserialize_unit`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `()`.
        pub fn deserialize_unit(
            &mut self,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_this_2 = self.__nc_pointer.as_raw();
            let __nc_status_2 = unsafe {
                ((*__nc_this_2.vtable).deserialize_unit)(__nc_this_2.object)
            };
            if __nc_status_2.is_err() {
                return ::core::result::Result::Err(__nc_status_2);
            }
            let __nc_result_2 = ();
            ::core::result::Result::Ok(__nc_result_2)
        }
        /// Acquires one validated bool value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:66:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::deserialize_bool`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: bool` from the ABI out slot.
        pub fn deserialize_bool(
            &mut self,
        ) -> ::core::result::Result<::core::primitive::bool, ::neocogi_com::NcStatus> {
            let mut __nc_out_3 = ::core::mem::MaybeUninit::<
                <::core::primitive::bool as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_3 = self.__nc_pointer.as_raw();
            let __nc_status_3 = unsafe {
                ((*__nc_this_3.vtable)
                    .deserialize_bool)(__nc_this_3.object, __nc_out_3.as_mut_ptr())
            };
            if __nc_status_3.is_err() {
                return ::core::result::Result::Err(__nc_status_3);
            }
            let __nc_result_3 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::bool,
                >(__nc_out_3.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_3)
        }
        /// Acquires one validated u8 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:68:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::deserialize_u8`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: u8` from the ABI out slot.
        pub fn deserialize_u8(
            &mut self,
        ) -> ::core::result::Result<::core::primitive::u8, ::neocogi_com::NcStatus> {
            let mut __nc_out_4 = ::core::mem::MaybeUninit::<
                <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_4 = self.__nc_pointer.as_raw();
            let __nc_status_4 = unsafe {
                ((*__nc_this_4.vtable)
                    .deserialize_u8)(__nc_this_4.object, __nc_out_4.as_mut_ptr())
            };
            if __nc_status_4.is_err() {
                return ::core::result::Result::Err(__nc_status_4);
            }
            let __nc_result_4 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::u8,
                >(__nc_out_4.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_4)
        }
        /// Acquires one validated u16 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:70:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::deserialize_u16`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: u16` from the ABI out slot.
        pub fn deserialize_u16(
            &mut self,
        ) -> ::core::result::Result<::core::primitive::u16, ::neocogi_com::NcStatus> {
            let mut __nc_out_5 = ::core::mem::MaybeUninit::<
                <::core::primitive::u16 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_5 = self.__nc_pointer.as_raw();
            let __nc_status_5 = unsafe {
                ((*__nc_this_5.vtable)
                    .deserialize_u16)(__nc_this_5.object, __nc_out_5.as_mut_ptr())
            };
            if __nc_status_5.is_err() {
                return ::core::result::Result::Err(__nc_status_5);
            }
            let __nc_result_5 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::u16,
                >(__nc_out_5.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_5)
        }
        /// Acquires one validated u32 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:72:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::deserialize_u32`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: u32` from the ABI out slot.
        pub fn deserialize_u32(
            &mut self,
        ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus> {
            let mut __nc_out_6 = ::core::mem::MaybeUninit::<
                <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_6 = self.__nc_pointer.as_raw();
            let __nc_status_6 = unsafe {
                ((*__nc_this_6.vtable)
                    .deserialize_u32)(__nc_this_6.object, __nc_out_6.as_mut_ptr())
            };
            if __nc_status_6.is_err() {
                return ::core::result::Result::Err(__nc_status_6);
            }
            let __nc_result_6 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::u32,
                >(__nc_out_6.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_6)
        }
        /// Acquires one validated u64 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:74:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::deserialize_u64`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: u64` from the ABI out slot.
        pub fn deserialize_u64(
            &mut self,
        ) -> ::core::result::Result<::core::primitive::u64, ::neocogi_com::NcStatus> {
            let mut __nc_out_7 = ::core::mem::MaybeUninit::<
                <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_7 = self.__nc_pointer.as_raw();
            let __nc_status_7 = unsafe {
                ((*__nc_this_7.vtable)
                    .deserialize_u64)(__nc_this_7.object, __nc_out_7.as_mut_ptr())
            };
            if __nc_status_7.is_err() {
                return ::core::result::Result::Err(__nc_status_7);
            }
            let __nc_result_7 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::u64,
                >(__nc_out_7.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_7)
        }
        /// Acquires one validated i8 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:76:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::deserialize_i8`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: i8` from the ABI out slot.
        pub fn deserialize_i8(
            &mut self,
        ) -> ::core::result::Result<::core::primitive::i8, ::neocogi_com::NcStatus> {
            let mut __nc_out_8 = ::core::mem::MaybeUninit::<
                <::core::primitive::i8 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_8 = self.__nc_pointer.as_raw();
            let __nc_status_8 = unsafe {
                ((*__nc_this_8.vtable)
                    .deserialize_i8)(__nc_this_8.object, __nc_out_8.as_mut_ptr())
            };
            if __nc_status_8.is_err() {
                return ::core::result::Result::Err(__nc_status_8);
            }
            let __nc_result_8 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::i8,
                >(__nc_out_8.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_8)
        }
        /// Acquires one validated i16 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:78:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::deserialize_i16`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: i16` from the ABI out slot.
        pub fn deserialize_i16(
            &mut self,
        ) -> ::core::result::Result<::core::primitive::i16, ::neocogi_com::NcStatus> {
            let mut __nc_out_9 = ::core::mem::MaybeUninit::<
                <::core::primitive::i16 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_9 = self.__nc_pointer.as_raw();
            let __nc_status_9 = unsafe {
                ((*__nc_this_9.vtable)
                    .deserialize_i16)(__nc_this_9.object, __nc_out_9.as_mut_ptr())
            };
            if __nc_status_9.is_err() {
                return ::core::result::Result::Err(__nc_status_9);
            }
            let __nc_result_9 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::i16,
                >(__nc_out_9.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_9)
        }
        /// Acquires one validated i32 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:80:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::deserialize_i32`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: i32` from the ABI out slot.
        pub fn deserialize_i32(
            &mut self,
        ) -> ::core::result::Result<::core::primitive::i32, ::neocogi_com::NcStatus> {
            let mut __nc_out_10 = ::core::mem::MaybeUninit::<
                <::core::primitive::i32 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_10 = self.__nc_pointer.as_raw();
            let __nc_status_10 = unsafe {
                ((*__nc_this_10.vtable)
                    .deserialize_i32)(__nc_this_10.object, __nc_out_10.as_mut_ptr())
            };
            if __nc_status_10.is_err() {
                return ::core::result::Result::Err(__nc_status_10);
            }
            let __nc_result_10 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::i32,
                >(__nc_out_10.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_10)
        }
        /// Acquires one validated i64 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:82:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::deserialize_i64`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: i64` from the ABI out slot.
        pub fn deserialize_i64(
            &mut self,
        ) -> ::core::result::Result<::core::primitive::i64, ::neocogi_com::NcStatus> {
            let mut __nc_out_11 = ::core::mem::MaybeUninit::<
                <::core::primitive::i64 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_11 = self.__nc_pointer.as_raw();
            let __nc_status_11 = unsafe {
                ((*__nc_this_11.vtable)
                    .deserialize_i64)(__nc_this_11.object, __nc_out_11.as_mut_ptr())
            };
            if __nc_status_11.is_err() {
                return ::core::result::Result::Err(__nc_status_11);
            }
            let __nc_result_11 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::i64,
                >(__nc_out_11.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_11)
        }
        /// Acquires one validated f32 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:84:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::deserialize_f32`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: f32` from the ABI out slot.
        pub fn deserialize_f32(
            &mut self,
        ) -> ::core::result::Result<::core::primitive::f32, ::neocogi_com::NcStatus> {
            let mut __nc_out_12 = ::core::mem::MaybeUninit::<
                <::core::primitive::f32 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_12 = self.__nc_pointer.as_raw();
            let __nc_status_12 = unsafe {
                ((*__nc_this_12.vtable)
                    .deserialize_f32)(__nc_this_12.object, __nc_out_12.as_mut_ptr())
            };
            if __nc_status_12.is_err() {
                return ::core::result::Result::Err(__nc_status_12);
            }
            let __nc_result_12 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::f32,
                >(__nc_out_12.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_12)
        }
        /// Acquires one validated f64 value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:86:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::deserialize_f64`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: f64` from the ABI out slot.
        pub fn deserialize_f64(
            &mut self,
        ) -> ::core::result::Result<::core::primitive::f64, ::neocogi_com::NcStatus> {
            let mut __nc_out_13 = ::core::mem::MaybeUninit::<
                <::core::primitive::f64 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_13 = self.__nc_pointer.as_raw();
            let __nc_status_13 = unsafe {
                ((*__nc_this_13.vtable)
                    .deserialize_f64)(__nc_this_13.object, __nc_out_13.as_mut_ptr())
            };
            if __nc_status_13.is_err() {
                return ::core::result::Result::Err(__nc_status_13);
            }
            let __nc_result_13 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::f64,
                >(__nc_out_13.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_13)
        }
        /// Admits a complete UTF-8 string and reports its byte length before allocation.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:88:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::string_len`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: u32` from the ABI out slot.
        pub fn string_len(
            &mut self,
        ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus> {
            let mut __nc_out_14 = ::core::mem::MaybeUninit::<
                <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_14 = self.__nc_pointer.as_raw();
            let __nc_status_14 = unsafe {
                ((*__nc_this_14.vtable)
                    .string_len)(__nc_this_14.object, __nc_out_14.as_mut_ptr())
            };
            if __nc_status_14.is_err() {
                return ::core::result::Result::Err(__nc_status_14);
            }
            let __nc_result_14 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::u32,
                >(__nc_out_14.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_14)
        }
        /// Copies exactly the admitted string into fresh caller-owned output.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:90:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::copy_string`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `output` supplies output storage for `:: core :: primitive :: u8` through [`::neocogi_com::NcOutput`]. See its lifecycle contract for acquisition, preparation and publication.
        ///
        ///On success, returns `:: core :: primitive :: u32` from the ABI out slot.
        pub fn copy_string(
            &mut self,
            __nc_arg_0: ::neocogi_com::NcOutput<'_, ::core::primitive::u8>,
        ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus> {
            let mut __nc_arg_0 = __nc_arg_0.prepare()?;
            let mut __nc_out_15 = ::core::mem::MaybeUninit::<
                <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_15 = self.__nc_pointer.as_raw();
            let __nc_status_15 = unsafe {
                ((*__nc_this_15.vtable)
                    .copy_string)(
                    __nc_this_15.object,
                    __nc_arg_0.offset(),
                    __nc_arg_0.capacity(),
                    __nc_arg_0.as_mut_ptr(),
                    __nc_arg_0.written_ptr(),
                    __nc_out_15.as_mut_ptr(),
                )
            };
            if __nc_status_15.is_err() {
                return ::core::result::Result::Err(__nc_status_15);
            }
            let __nc_result_15 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::u32,
                >(__nc_out_15.assume_init())
            })?;
            (unsafe { __nc_arg_0.convert_output() })?;
            __nc_arg_0.publish();
            ::core::result::Result::Ok(__nc_result_15)
        }
        /// Reads optional presence; a present child follows within this value scope.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:92:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::option_present`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: bool` from the ABI out slot.
        pub fn option_present(
            &mut self,
        ) -> ::core::result::Result<::core::primitive::bool, ::neocogi_com::NcStatus> {
            let mut __nc_out_16 = ::core::mem::MaybeUninit::<
                <::core::primitive::bool as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_16 = self.__nc_pointer.as_raw();
            let __nc_status_16 = unsafe {
                ((*__nc_this_16.vtable)
                    .option_present)(__nc_this_16.object, __nc_out_16.as_mut_ptr())
            };
            if __nc_status_16.is_err() {
                return ::core::result::Result::Err(__nc_status_16);
            }
            let __nc_result_16 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::bool,
                >(__nc_out_16.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_16)
        }
        /// Opens a record and rejects more than maximum fields.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:94:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::begin_record`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `maximum` is passed by value as `:: core :: primitive :: u32`.
        ///
        ///On success, returns `:: core :: primitive :: u32` from the ABI out slot.
        pub fn begin_record(
            &mut self,
            __nc_arg_0_0: ::core::primitive::u32,
        ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus> {
            let __nc_arg_0_0 = ::neocogi_com::encode_inline(&__nc_arg_0_0)?;
            let mut __nc_out_17 = ::core::mem::MaybeUninit::<
                <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_17 = self.__nc_pointer.as_raw();
            let __nc_status_17 = unsafe {
                ((*__nc_this_17.vtable)
                    .begin_record)(
                    __nc_this_17.object,
                    __nc_arg_0_0,
                    __nc_out_17.as_mut_ptr(),
                )
            };
            if __nc_status_17.is_err() {
                return ::core::result::Result::Err(__nc_status_17);
            }
            let __nc_result_17 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::u32,
                >(__nc_out_17.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_17)
        }
        /// Selects a named field in schema order; false means genuinely absent.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:96:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::field`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `name` is passed as a safe readable `:: core :: primitive :: u8` buffer plus offset.
        ///
        ///On success, returns `:: core :: primitive :: bool` from the ABI out slot.
        pub fn field(
            &mut self,
            __nc_arg_0_1: ::neocogi_com::NcCopyFromBufferOffset<
                '_,
                ::core::primitive::u8,
            >,
        ) -> ::core::result::Result<::core::primitive::bool, ::neocogi_com::NcStatus> {
            let __nc_arg_0_1 = ::neocogi_com::EncodedInput::new(__nc_arg_0_1)?;
            let mut __nc_out_18 = ::core::mem::MaybeUninit::<
                <::core::primitive::bool as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_18 = self.__nc_pointer.as_raw();
            let __nc_status_18 = unsafe {
                ((*__nc_this_18.vtable)
                    .field)(
                    __nc_this_18.object,
                    __nc_arg_0_1.offset(),
                    __nc_arg_0_1.len(),
                    __nc_arg_0_1.as_ptr(),
                    __nc_out_18.as_mut_ptr(),
                )
            };
            if __nc_status_18.is_err() {
                return ::core::result::Result::Err(__nc_status_18);
            }
            let __nc_result_18 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::bool,
                >(__nc_out_18.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_18)
        }
        /// Completes the record, rejecting unconsumed or unknown fields.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:98:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::end_record`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `()`.
        pub fn end_record(
            &mut self,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_this_19 = self.__nc_pointer.as_raw();
            let __nc_status_19 = unsafe {
                ((*__nc_this_19.vtable).end_record)(__nc_this_19.object)
            };
            if __nc_status_19.is_err() {
                return ::core::result::Result::Err(__nc_status_19);
            }
            let __nc_result_19 = ();
            ::core::result::Result::Ok(__nc_result_19)
        }
        /// Opens an ordered collection and reports its bounded element count.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:100:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::begin_sequence`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: u32` from the ABI out slot.
        pub fn begin_sequence(
            &mut self,
        ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus> {
            let mut __nc_out_20 = ::core::mem::MaybeUninit::<
                <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_20 = self.__nc_pointer.as_raw();
            let __nc_status_20 = unsafe {
                ((*__nc_this_20.vtable)
                    .begin_sequence)(__nc_this_20.object, __nc_out_20.as_mut_ptr())
            };
            if __nc_status_20.is_err() {
                return ::core::result::Result::Err(__nc_status_20);
            }
            let __nc_result_20 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::u32,
                >(__nc_out_20.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_20)
        }
        /// Completes the collection, rejecting unconsumed elements.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:102:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::end_sequence`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `()`.
        pub fn end_sequence(
            &mut self,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_this_21 = self.__nc_pointer.as_raw();
            let __nc_status_21 = unsafe {
                ((*__nc_this_21.vtable).end_sequence)(__nc_this_21.object)
            };
            if __nc_status_21.is_err() {
                return ::core::result::Result::Err(__nc_status_21);
            }
            let __nc_result_21 = ();
            ::core::result::Result::Ok(__nc_result_21)
        }
        /// Opens a map and reports its bounded pair count.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:104:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::begin_map`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: u32` from the ABI out slot.
        pub fn begin_map(
            &mut self,
        ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus> {
            let mut __nc_out_22 = ::core::mem::MaybeUninit::<
                <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_22 = self.__nc_pointer.as_raw();
            let __nc_status_22 = unsafe {
                ((*__nc_this_22.vtable)
                    .begin_map)(__nc_this_22.object, __nc_out_22.as_mut_ptr())
            };
            if __nc_status_22.is_err() {
                return ::core::result::Result::Err(__nc_status_22);
            }
            let __nc_result_22 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::u32,
                >(__nc_out_22.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_22)
        }
        /// Completes the map.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:106:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::end_map`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `()`.
        pub fn end_map(
            &mut self,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_this_23 = self.__nc_pointer.as_raw();
            let __nc_status_23 = unsafe {
                ((*__nc_this_23.vtable).end_map)(__nc_this_23.object)
            };
            if __nc_status_23.is_err() {
                return ::core::result::Result::Err(__nc_status_23);
            }
            let __nc_result_23 = ();
            ::core::result::Result::Ok(__nc_result_23)
        }
        /// Probes cases in declaration order; a match selects the following payload.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:108:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::variant_is`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `index` is passed by value as `:: core :: primitive :: u32`.
        ///- `name` is passed as a safe readable `:: core :: primitive :: u8` buffer plus offset.
        ///
        ///On success, returns `:: core :: primitive :: bool` from the ABI out slot.
        pub fn variant_is(
            &mut self,
            __nc_arg_0_2: ::core::primitive::u32,
            __nc_arg_1: ::neocogi_com::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
        ) -> ::core::result::Result<::core::primitive::bool, ::neocogi_com::NcStatus> {
            let __nc_arg_0_2 = ::neocogi_com::encode_inline(&__nc_arg_0_2)?;
            let __nc_arg_1 = ::neocogi_com::EncodedInput::new(__nc_arg_1)?;
            let mut __nc_out_24 = ::core::mem::MaybeUninit::<
                <::core::primitive::bool as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_24 = self.__nc_pointer.as_raw();
            let __nc_status_24 = unsafe {
                ((*__nc_this_24.vtable)
                    .variant_is)(
                    __nc_this_24.object,
                    __nc_arg_0_2,
                    __nc_arg_1.offset(),
                    __nc_arg_1.len(),
                    __nc_arg_1.as_ptr(),
                    __nc_out_24.as_mut_ptr(),
                )
            };
            if __nc_status_24.is_err() {
                return ::core::result::Result::Err(__nc_status_24);
            }
            let __nc_result_24 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::bool,
                >(__nc_out_24.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_24)
        }
        /// Marks a schema failure; no later operation may succeed.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:110:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::fail`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `message` is passed as a safe readable `:: core :: primitive :: u8` buffer plus offset.
        ///
        ///On success, returns `()`.
        pub fn fail(
            &mut self,
            __nc_arg_0_3: ::neocogi_com::NcCopyFromBufferOffset<
                '_,
                ::core::primitive::u8,
            >,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0_3 = ::neocogi_com::EncodedInput::new(__nc_arg_0_3)?;
            let __nc_this_25 = self.__nc_pointer.as_raw();
            let __nc_status_25 = unsafe {
                ((*__nc_this_25.vtable)
                    .fail)(
                    __nc_this_25.object,
                    __nc_arg_0_3.offset(),
                    __nc_arg_0_3.len(),
                    __nc_arg_0_3.as_ptr(),
                )
            };
            if __nc_status_25.is_err() {
                return ::core::result::Result::Err(__nc_status_25);
            }
            let __nc_result_25 = ();
            ::core::result::Result::Ok(__nc_result_25)
        }
        /// Requires a complete root with no trailing input and closes the session.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:112:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`IDeserializer::finish`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `()`.
        pub fn finish(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_this_26 = self.__nc_pointer.as_raw();
            let __nc_status_26 = unsafe {
                ((*__nc_this_26.vtable).finish)(__nc_this_26.object)
            };
            if __nc_status_26.is_err() {
                return ::core::result::Result::Err(__nc_status_26);
            }
            let __nc_result_26 = ();
            ::core::result::Result::Ok(__nc_result_26)
        }
    }
};
impl IDeserializerAbi {
    ///Canonical validation form of this interface's typed ABI contract.
    pub const ABI_SIGNATURE: ::neocogi_com::NcAbiSignature = ::neocogi_com::NcAbiSignature::from_static(
        b"\xA0\\\x87\x8D\rW\xB4]z\x90\x0F\x18\x103\xFD \xFE\x846\x08\xA6&\x90\x81\xE8\x05\xC3\xF1\xC1\x8F[\xFB",
    );
    ///Static descriptors for this interface's ABI methods.
    pub const METHOD_DESCRIPTORS: [::neocogi_com::NcMethodDescriptor; 27usize] = [
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("enter_value"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("leave_value"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("deserialize_unit"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("deserialize_bool"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::bool>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("deserialize_u8"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::u8>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("deserialize_u16"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::u16>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("deserialize_u32"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("deserialize_u64"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::u64>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("deserialize_i8"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::i8>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("deserialize_i16"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::i16>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("deserialize_i32"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::i32>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("deserialize_i64"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::i64>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("deserialize_f32"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::f32>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("deserialize_f64"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::f64>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("string_len"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("copy_string"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("output"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::OutputBuffer,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_plain_type::<::core::primitive::u8>(),
                    ),
                },
            ] as &'static [::neocogi_com::NcMethodParamDescriptor])
                .as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("option_present"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::bool>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("begin_record"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("maximum"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
                    ),
                },
            ] as &'static [::neocogi_com::NcMethodParamDescriptor])
                .as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("field"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("name"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::CopyFromBufferOffset,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_plain_type::<::core::primitive::u8>(),
                    ),
                },
            ] as &'static [::neocogi_com::NcMethodParamDescriptor])
                .as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::bool>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("end_record"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("begin_sequence"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("end_sequence"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("begin_map"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("end_map"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("variant_is"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 2u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("index"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
                    ),
                },
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("name"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::CopyFromBufferOffset,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_plain_type::<::core::primitive::u8>(),
                    ),
                },
            ] as &'static [::neocogi_com::NcMethodParamDescriptor])
                .as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::bool>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("fail"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("message"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::CopyFromBufferOffset,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_plain_type::<::core::primitive::u8>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("finish"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Void,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::NcAbiType::Void,
                ),
            },
        },
    ];
    ///Static ABI descriptor for this interface.
    pub const INTERFACE_DESCRIPTOR: ::neocogi_com::NcInterfaceDescriptor = ::neocogi_com::NcInterfaceDescriptor {
        interface_id: <Self as ::neocogi_com::ComInterface>::IID,
        qualified_name: ::neocogi_com::NcStaticUtf8::from_static(
            "neocogi.com.serialization.IDeserializer",
        ),
        abi_signature: Self::ABI_SIGNATURE,
        base: ::neocogi_com::NcInterfaceId::NONE,
        method_count: 27u32,
        methods: Self::METHOD_DESCRIPTORS.as_ptr(),
    };
    /// Distinct projections in derived-first, declared-parent order.
    /// Shared ancestors occur once; repeated identities must have the same ABI.
    pub const INTERFACE_CHAIN: [&'static ::neocogi_com::NcInterfaceDescriptor; ::neocogi_com::__nc_object_interface_count(
        &[&[&Self::INTERFACE_DESCRIPTOR]],
    )] = ::neocogi_com::__nc_interface_chain(&Self::INTERFACE_DESCRIPTOR, &[]);
}
/// Immutable first-failure details on an owning serializer/deserializer session.
/// Coordinates are optional: u64::MAX means no byte offset, zero means no line/column.
/// IDL source: `neocogi/com/serialization/formats.ncidl:141:11` (module `neocogi.com.serialization`).
pub trait ISerializationDiagnostics {
    /// Returns the original failed operation's status, as data.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:143:8` (module `neocogi.com.serialization`).
    fn failure_status(
        &self,
    ) -> ::core::result::Result<::neocogi_com::NcStatus, ::neocogi_com::NcStatus>;
    /// Reports a zero-based byte cursor/offset, or u64::MAX when unavailable.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:145:8` (module `neocogi.com.serialization`).
    fn byte_offset(
        &self,
    ) -> ::core::result::Result<::core::primitive::u64, ::neocogi_com::NcStatus>;
    /// Reports a one-based source line, or zero when unavailable.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:147:8` (module `neocogi.com.serialization`).
    fn line(
        &self,
    ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus>;
    /// Reports a one-based source column, or zero when unavailable.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:149:8` (module `neocogi.com.serialization`).
    fn column(
        &self,
    ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus>;
    /// Reports the diagnostic message's UTF-8 byte length.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:151:8` (module `neocogi.com.serialization`).
    fn message_len(
        &self,
    ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus>;
    /// Copies the complete message to fresh output; short capacity writes no prefix.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:153:8` (module `neocogi.com.serialization`).
    fn copy_message(
        &self,
        output: ::neocogi_com::NcOutputWriter<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus>;
    /// Reports the logical field/index path's UTF-8 byte length.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:155:8` (module `neocogi.com.serialization`).
    fn path_len(
        &self,
    ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus>;
    /// Copies the complete logical path, rooted at $, into fresh output.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:157:8` (module `neocogi.com.serialization`).
    fn copy_path(
        &self,
        output: ::neocogi_com::NcOutputWriter<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus>;
}
///Interface marker for [`ISerializationDiagnostics`]; handles carry its object/table pair.
pub enum ISerializationDiagnosticsAbi {}
#[repr(C)]
///C-compatible vtable for [`ISerializationDiagnostics`].
pub struct ISerializationDiagnosticsVTable {
    /// Common identity, query and reference-count slots.
    pub unknown: ::neocogi_com::IUnknownVTable,
    /// Returns the original failed operation's status, as data.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:143:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializationDiagnostics::failure_status`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: neocogi_com :: NcStatus as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub failure_status: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::neocogi_com::NcStatus as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Reports a zero-based byte cursor/offset, or u64::MAX when unavailable.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:145:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializationDiagnostics::byte_offset`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u64 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub byte_offset: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Reports a one-based source line, or zero when unavailable.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:147:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializationDiagnostics::line`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub line: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Reports a one-based source column, or zero when unavailable.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:149:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializationDiagnostics::column`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub column: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Reports the diagnostic message's UTF-8 byte length.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:151:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializationDiagnostics::message_len`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub message_len: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Copies the complete message to fresh output; short capacity writes no prefix.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:153:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializationDiagnostics::copy_message`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `output` uses ABI storage `* mut < :: core :: primitive :: u8 as :: neocogi_com :: ComValue > :: Wire` for `:: neocogi_com :: NcOutputWriter < '_ , :: core :: primitive :: u8 >`. The writable buffer pointer is accompanied by `offset: u64`, `capacity: u64`, and a separate `written: *mut u64`. Old contents are not input. Only the successfully reported initialized prefix may be read; count and element storage must not alias.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub copy_message: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_offset: ::core::primitive::u64,
        __nc_arg_0_len: ::core::primitive::u64,
        __nc_arg_0_ptr: *mut <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_0_written: *mut ::core::primitive::u64,
        __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Reports the logical field/index path's UTF-8 byte length.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:155:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializationDiagnostics::path_len`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub path_len: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Copies the complete logical path, rooted at $, into fresh output.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:157:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializationDiagnostics::copy_path`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `output` uses ABI storage `* mut < :: core :: primitive :: u8 as :: neocogi_com :: ComValue > :: Wire` for `:: neocogi_com :: NcOutputWriter < '_ , :: core :: primitive :: u8 >`. The writable buffer pointer is accompanied by `offset: u64`, `capacity: u64`, and a separate `written: *mut u64`. Old contents are not input. Only the successfully reported initialized prefix may be read; count and element storage must not alias.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub copy_path: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_0_offset: ::core::primitive::u64,
        __nc_arg_0_0_len: ::core::primitive::u64,
        __nc_arg_0_0_ptr: *mut <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_0_0_written: *mut ::core::primitive::u64,
        __nc_out: *mut <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
}
unsafe impl ::neocogi_com::ComInterface for ISerializationDiagnosticsAbi {
    const IID: ::neocogi_com::NcGuid = ::neocogi_com::NcGuid {
        data1: 3478217469u32,
        data2: 8526u16,
        data3: 23497u16,
        data4: [162u8, 217u8, 71u8, 39u8, 16u8, 184u8, 184u8, 223u8],
    };
    const INTERFACE_ID: ::neocogi_com::NcInterfaceId = ::neocogi_com::NcInterfaceId::new(
        ::neocogi_com::NcGuid {
            data1: 3478217469u32,
            data2: 8526u16,
            data3: 23497u16,
            data4: [162u8, 217u8, 71u8, 39u8, 16u8, 184u8, 184u8, 223u8],
        },
        ISerializationDiagnosticsAbi::ABI_SIGNATURE,
    );
    type VTable = ISerializationDiagnosticsVTable;
    type Threading = ::neocogi_com::ThreadLocal;
}
#[repr(transparent)]
///Borrowed static method view for [`ISerializationDiagnostics`].
pub struct ISerializationDiagnosticsClient<
    __NcThreading: ::neocogi_com::ThreadingModel = ::neocogi_com::ThreadLocal,
> {
    /// Certified transparent storage supplied only by a live COM pointer borrow.
    __nc_pointer: ::neocogi_com::NcClientPointer<
        ISerializationDiagnosticsAbi,
        __NcThreading,
    >,
}
unsafe impl ::neocogi_com::ComClientInterface for ISerializationDiagnosticsAbi {
    type Client<__NcThreading: ::neocogi_com::ThreadingModel> = ISerializationDiagnosticsClient<
        __NcThreading,
    >;
    fn __nc_client_ref<__NcThreading: ::neocogi_com::ThreadingModel>(
        pointer: &::neocogi_com::NcClientPointer<Self, __NcThreading>,
    ) -> &Self::Client<__NcThreading> {
        unsafe {
            &*(::core::ptr::from_ref(pointer)
                as *const ISerializationDiagnosticsClient<__NcThreading>)
        }
    }
    fn __nc_client_mut<__NcThreading: ::neocogi_com::ThreadingModel>(
        pointer: &mut ::neocogi_com::NcClientPointer<Self, __NcThreading>,
    ) -> &mut Self::Client<__NcThreading> {
        unsafe {
            &mut *(::core::ptr::from_mut(pointer)
                as *mut ISerializationDiagnosticsClient<__NcThreading>)
        }
    }
}
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
    impl<
        __NcThreading: ::neocogi_com::ThreadingModel,
    > ISerializationDiagnosticsClient<__NcThreading> {
        /// Returns the original failed operation's status, as data.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:143:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializationDiagnostics::failure_status`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: neocogi_com :: NcStatus` from the ABI out slot.
        pub fn failure_status(
            &self,
        ) -> ::core::result::Result<::neocogi_com::NcStatus, ::neocogi_com::NcStatus> {
            let mut __nc_out_0 = ::core::mem::MaybeUninit::<
                <::neocogi_com::NcStatus as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_0 = self.__nc_pointer.as_raw();
            let __nc_status_0 = unsafe {
                ((*__nc_this_0.vtable)
                    .failure_status)(__nc_this_0.object, __nc_out_0.as_mut_ptr())
            };
            if __nc_status_0.is_err() {
                return ::core::result::Result::Err(__nc_status_0);
            }
            let __nc_result_0 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::neocogi_com::NcStatus,
                >(__nc_out_0.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_0)
        }
        /// Reports a zero-based byte cursor/offset, or u64::MAX when unavailable.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:145:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializationDiagnostics::byte_offset`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: u64` from the ABI out slot.
        pub fn byte_offset(
            &self,
        ) -> ::core::result::Result<::core::primitive::u64, ::neocogi_com::NcStatus> {
            let mut __nc_out_1 = ::core::mem::MaybeUninit::<
                <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_1 = self.__nc_pointer.as_raw();
            let __nc_status_1 = unsafe {
                ((*__nc_this_1.vtable)
                    .byte_offset)(__nc_this_1.object, __nc_out_1.as_mut_ptr())
            };
            if __nc_status_1.is_err() {
                return ::core::result::Result::Err(__nc_status_1);
            }
            let __nc_result_1 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::u64,
                >(__nc_out_1.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_1)
        }
        /// Reports a one-based source line, or zero when unavailable.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:147:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializationDiagnostics::line`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: u32` from the ABI out slot.
        pub fn line(
            &self,
        ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus> {
            let mut __nc_out_2 = ::core::mem::MaybeUninit::<
                <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_2 = self.__nc_pointer.as_raw();
            let __nc_status_2 = unsafe {
                ((*__nc_this_2.vtable).line)(__nc_this_2.object, __nc_out_2.as_mut_ptr())
            };
            if __nc_status_2.is_err() {
                return ::core::result::Result::Err(__nc_status_2);
            }
            let __nc_result_2 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::u32,
                >(__nc_out_2.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_2)
        }
        /// Reports a one-based source column, or zero when unavailable.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:149:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializationDiagnostics::column`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: u32` from the ABI out slot.
        pub fn column(
            &self,
        ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus> {
            let mut __nc_out_3 = ::core::mem::MaybeUninit::<
                <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_3 = self.__nc_pointer.as_raw();
            let __nc_status_3 = unsafe {
                ((*__nc_this_3.vtable)
                    .column)(__nc_this_3.object, __nc_out_3.as_mut_ptr())
            };
            if __nc_status_3.is_err() {
                return ::core::result::Result::Err(__nc_status_3);
            }
            let __nc_result_3 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::u32,
                >(__nc_out_3.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_3)
        }
        /// Reports the diagnostic message's UTF-8 byte length.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:151:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializationDiagnostics::message_len`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: u32` from the ABI out slot.
        pub fn message_len(
            &self,
        ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus> {
            let mut __nc_out_4 = ::core::mem::MaybeUninit::<
                <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_4 = self.__nc_pointer.as_raw();
            let __nc_status_4 = unsafe {
                ((*__nc_this_4.vtable)
                    .message_len)(__nc_this_4.object, __nc_out_4.as_mut_ptr())
            };
            if __nc_status_4.is_err() {
                return ::core::result::Result::Err(__nc_status_4);
            }
            let __nc_result_4 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::u32,
                >(__nc_out_4.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_4)
        }
        /// Copies the complete message to fresh output; short capacity writes no prefix.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:153:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializationDiagnostics::copy_message`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `output` supplies output storage for `:: core :: primitive :: u8` through [`::neocogi_com::NcOutput`]. See its lifecycle contract for acquisition, preparation and publication.
        ///
        ///On success, returns `:: core :: primitive :: u32` from the ABI out slot.
        pub fn copy_message(
            &self,
            __nc_arg_0: ::neocogi_com::NcOutput<'_, ::core::primitive::u8>,
        ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus> {
            let mut __nc_arg_0 = __nc_arg_0.prepare()?;
            let mut __nc_out_5 = ::core::mem::MaybeUninit::<
                <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_5 = self.__nc_pointer.as_raw();
            let __nc_status_5 = unsafe {
                ((*__nc_this_5.vtable)
                    .copy_message)(
                    __nc_this_5.object,
                    __nc_arg_0.offset(),
                    __nc_arg_0.capacity(),
                    __nc_arg_0.as_mut_ptr(),
                    __nc_arg_0.written_ptr(),
                    __nc_out_5.as_mut_ptr(),
                )
            };
            if __nc_status_5.is_err() {
                return ::core::result::Result::Err(__nc_status_5);
            }
            let __nc_result_5 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::u32,
                >(__nc_out_5.assume_init())
            })?;
            (unsafe { __nc_arg_0.convert_output() })?;
            __nc_arg_0.publish();
            ::core::result::Result::Ok(__nc_result_5)
        }
        /// Reports the logical field/index path's UTF-8 byte length.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:155:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializationDiagnostics::path_len`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: u32` from the ABI out slot.
        pub fn path_len(
            &self,
        ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus> {
            let mut __nc_out_6 = ::core::mem::MaybeUninit::<
                <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_6 = self.__nc_pointer.as_raw();
            let __nc_status_6 = unsafe {
                ((*__nc_this_6.vtable)
                    .path_len)(__nc_this_6.object, __nc_out_6.as_mut_ptr())
            };
            if __nc_status_6.is_err() {
                return ::core::result::Result::Err(__nc_status_6);
            }
            let __nc_result_6 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::u32,
                >(__nc_out_6.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_6)
        }
        /// Copies the complete logical path, rooted at $, into fresh output.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:157:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializationDiagnostics::copy_path`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `output` supplies output storage for `:: core :: primitive :: u8` through [`::neocogi_com::NcOutput`]. See its lifecycle contract for acquisition, preparation and publication.
        ///
        ///On success, returns `:: core :: primitive :: u32` from the ABI out slot.
        pub fn copy_path(
            &self,
            __nc_arg_0_0: ::neocogi_com::NcOutput<'_, ::core::primitive::u8>,
        ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus> {
            let mut __nc_arg_0_0 = __nc_arg_0_0.prepare()?;
            let mut __nc_out_7 = ::core::mem::MaybeUninit::<
                <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_7 = self.__nc_pointer.as_raw();
            let __nc_status_7 = unsafe {
                ((*__nc_this_7.vtable)
                    .copy_path)(
                    __nc_this_7.object,
                    __nc_arg_0_0.offset(),
                    __nc_arg_0_0.capacity(),
                    __nc_arg_0_0.as_mut_ptr(),
                    __nc_arg_0_0.written_ptr(),
                    __nc_out_7.as_mut_ptr(),
                )
            };
            if __nc_status_7.is_err() {
                return ::core::result::Result::Err(__nc_status_7);
            }
            let __nc_result_7 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::u32,
                >(__nc_out_7.assume_init())
            })?;
            (unsafe { __nc_arg_0_0.convert_output() })?;
            __nc_arg_0_0.publish();
            ::core::result::Result::Ok(__nc_result_7)
        }
    }
};
impl ISerializationDiagnosticsAbi {
    ///Canonical validation form of this interface's typed ABI contract.
    pub const ABI_SIGNATURE: ::neocogi_com::NcAbiSignature = ::neocogi_com::NcAbiSignature::from_static(
        b"\x13\x1E\xEF\xE7;\xA1\x9F\xEB\xB1\x9B\xB2'\x81\xE1\xB2\xFAe\x07\xEA\x12\xFCX\xF01\x98\x01\x89H\x1By\xEF\x15",
    );
    ///Static descriptors for this interface's ABI methods.
    pub const METHOD_DESCRIPTORS: [::neocogi_com::NcMethodDescriptor; 8usize] = [
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("failure_status"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::neocogi_com::NcStatus>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("byte_offset"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::u64>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("line"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("column"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("message_len"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("copy_message"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("output"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::OutputBuffer,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_plain_type::<::core::primitive::u8>(),
                    ),
                },
            ] as &'static [::neocogi_com::NcMethodParamDescriptor])
                .as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("path_len"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("copy_path"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("output"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::OutputBuffer,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_plain_type::<::core::primitive::u8>(),
                    ),
                },
            ] as &'static [::neocogi_com::NcMethodParamDescriptor])
                .as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
                ),
            },
        },
    ];
    ///Static ABI descriptor for this interface.
    pub const INTERFACE_DESCRIPTOR: ::neocogi_com::NcInterfaceDescriptor = ::neocogi_com::NcInterfaceDescriptor {
        interface_id: <Self as ::neocogi_com::ComInterface>::IID,
        qualified_name: ::neocogi_com::NcStaticUtf8::from_static(
            "neocogi.com.serialization.ISerializationDiagnostics",
        ),
        abi_signature: Self::ABI_SIGNATURE,
        base: ::neocogi_com::NcInterfaceId::NONE,
        method_count: 8u32,
        methods: Self::METHOD_DESCRIPTORS.as_ptr(),
    };
    /// Distinct projections in derived-first, declared-parent order.
    /// Shared ancestors occur once; repeated identities must have the same ABI.
    pub const INTERFACE_CHAIN: [&'static ::neocogi_com::NcInterfaceDescriptor; ::neocogi_com::__nc_object_interface_count(
        &[&[&Self::INTERFACE_DESCRIPTOR]],
    )] = ::neocogi_com::__nc_interface_chain(&Self::INTERFACE_DESCRIPTOR, &[]);
}
/// A format plugin's shared root; each document receives its own exclusive session.
/// IDL source: `neocogi/com/serialization/formats.ncidl:124:11` (module `neocogi.com.serialization`).
pub trait ISerializationFormat {
    /// Creates a bounded serializer with format-owned output storage.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:126:8` (module `neocogi.com.serialization`).
    fn create_serializer(
        &self,
        max_bytes: ::core::primitive::u64,
        max_elements: ::core::primitive::u32,
        max_depth: ::core::primitive::u32,
        max_values: ::core::primitive::u64,
    ) -> ::core::result::Result<
        ::neocogi_com::ComBox<self::ISerializerAbi>,
        ::neocogi_com::NcStatus,
    >;
    /// Copies the complete borrowed document into a new bounded source session.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:128:8` (module `neocogi.com.serialization`).
    fn create_deserializer(
        &self,
        input: ::neocogi_com::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
        max_bytes: ::core::primitive::u64,
        max_elements: ::core::primitive::u32,
        max_depth: ::core::primitive::u32,
        max_values: ::core::primitive::u64,
    ) -> ::core::result::Result<
        ::neocogi_com::ComBox<self::IDeserializerAbi>,
        ::neocogi_com::NcStatus,
    >;
}
///Interface marker for [`ISerializationFormat`]; handles carry its object/table pair.
pub enum ISerializationFormatAbi {}
#[repr(C)]
///C-compatible vtable for [`ISerializationFormat`].
pub struct ISerializationFormatVTable {
    /// Common identity, query and reference-count slots.
    pub unknown: ::neocogi_com::IUnknownVTable,
    /// Creates a bounded serializer with format-owned output storage.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:126:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializationFormat::create_serializer`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `max_bytes` uses ABI storage `< :: core :: primitive :: u64 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: u64`. Passed by value.
    ///- `max_elements` uses ABI storage `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: u32`. Passed by value.
    ///- `max_depth` uses ABI storage `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: u32`. Passed by value.
    ///- `max_values` uses ABI storage `< :: core :: primitive :: u64 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: u64`. Passed by value.
    ///
    ///On success, `out` receives `:: neocogi_com :: ComRaw < self :: ISerializerAbi >` for `:: neocogi_com :: ComBox < self :: ISerializerAbi >`. The caller receives the sole exclusive owner; interface queries cannot duplicate it.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub create_serializer: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0: <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_1: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_2: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_3: <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
        __nc_out: *mut ::neocogi_com::ComRaw<self::ISerializerAbi>,
    ) -> ::neocogi_com::NcStatus,
    /// Copies the complete borrowed document into a new bounded source session.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:128:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializationFormat::create_deserializer`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `input` uses ABI storage `* const < :: core :: primitive :: u8 as :: neocogi_com :: ComValue > :: Wire` for `:: neocogi_com :: NcCopyFromBufferOffset < '_ , :: core :: primitive :: u8 >`. The readable buffer pointer is accompanied by `offset: u64` and `len: u64`.
    ///- `max_bytes` uses ABI storage `< :: core :: primitive :: u64 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: u64`. Passed by value.
    ///- `max_elements` uses ABI storage `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: u32`. Passed by value.
    ///- `max_depth` uses ABI storage `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: u32`. Passed by value.
    ///- `max_values` uses ABI storage `< :: core :: primitive :: u64 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: u64`. Passed by value.
    ///
    ///On success, `out` receives `:: neocogi_com :: ComRaw < self :: IDeserializerAbi >` for `:: neocogi_com :: ComBox < self :: IDeserializerAbi >`. The caller receives the sole exclusive owner; interface queries cannot duplicate it.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub create_deserializer: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_0_offset: ::core::primitive::u64,
        __nc_arg_0_0_len: ::core::primitive::u64,
        __nc_arg_0_0_ptr: *const <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_1_0: <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_2_0: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_3_0: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_4: <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
        __nc_out: *mut ::neocogi_com::ComRaw<self::IDeserializerAbi>,
    ) -> ::neocogi_com::NcStatus,
}
unsafe impl ::neocogi_com::ComInterface for ISerializationFormatAbi {
    const IID: ::neocogi_com::NcGuid = ::neocogi_com::NcGuid {
        data1: 2495874208u32,
        data2: 33281u16,
        data3: 23583u16,
        data4: [151u8, 222u8, 148u8, 217u8, 21u8, 175u8, 146u8, 255u8],
    };
    const INTERFACE_ID: ::neocogi_com::NcInterfaceId = ::neocogi_com::NcInterfaceId::new(
        ::neocogi_com::NcGuid {
            data1: 2495874208u32,
            data2: 33281u16,
            data3: 23583u16,
            data4: [151u8, 222u8, 148u8, 217u8, 21u8, 175u8, 146u8, 255u8],
        },
        ISerializationFormatAbi::ABI_SIGNATURE,
    );
    type VTable = ISerializationFormatVTable;
    type Threading = ::neocogi_com::ThreadLocal;
}
#[repr(transparent)]
///Borrowed static method view for [`ISerializationFormat`].
pub struct ISerializationFormatClient<
    __NcThreading: ::neocogi_com::ThreadingModel = ::neocogi_com::ThreadLocal,
> {
    /// Certified transparent storage supplied only by a live COM pointer borrow.
    __nc_pointer: ::neocogi_com::NcClientPointer<ISerializationFormatAbi, __NcThreading>,
}
unsafe impl ::neocogi_com::ComClientInterface for ISerializationFormatAbi {
    type Client<__NcThreading: ::neocogi_com::ThreadingModel> = ISerializationFormatClient<
        __NcThreading,
    >;
    fn __nc_client_ref<__NcThreading: ::neocogi_com::ThreadingModel>(
        pointer: &::neocogi_com::NcClientPointer<Self, __NcThreading>,
    ) -> &Self::Client<__NcThreading> {
        unsafe {
            &*(::core::ptr::from_ref(pointer)
                as *const ISerializationFormatClient<__NcThreading>)
        }
    }
    fn __nc_client_mut<__NcThreading: ::neocogi_com::ThreadingModel>(
        pointer: &mut ::neocogi_com::NcClientPointer<Self, __NcThreading>,
    ) -> &mut Self::Client<__NcThreading> {
        unsafe {
            &mut *(::core::ptr::from_mut(pointer)
                as *mut ISerializationFormatClient<__NcThreading>)
        }
    }
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
    impl<
        __NcThreading: ::neocogi_com::ThreadingModel,
    > ISerializationFormatClient<__NcThreading> {
        /// Creates a bounded serializer with format-owned output storage.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:126:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializationFormat::create_serializer`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `max_bytes` is passed by value as `:: core :: primitive :: u64`.
        ///- `max_elements` is passed by value as `:: core :: primitive :: u32`.
        ///- `max_depth` is passed by value as `:: core :: primitive :: u32`.
        ///- `max_values` is passed by value as `:: core :: primitive :: u64`.
        ///
        ///On success, returns `:: neocogi_com :: ComBox < self :: ISerializerAbi >`. The caller receives the sole exclusive owner; interface queries cannot duplicate it.
        pub fn create_serializer(
            &self,
            __nc_arg_0: ::core::primitive::u64,
            __nc_arg_1: ::core::primitive::u32,
            __nc_arg_2: ::core::primitive::u32,
            __nc_arg_3: ::core::primitive::u64,
        ) -> ::core::result::Result<
            ::neocogi_com::ComBox<self::ISerializerAbi>,
            ::neocogi_com::NcStatus,
        > {
            let __nc_arg_0 = ::neocogi_com::encode_inline(&__nc_arg_0)?;
            let __nc_arg_1 = ::neocogi_com::encode_inline(&__nc_arg_1)?;
            let __nc_arg_2 = ::neocogi_com::encode_inline(&__nc_arg_2)?;
            let __nc_arg_3 = ::neocogi_com::encode_inline(&__nc_arg_3)?;
            let mut __nc_out_0 = ::core::mem::MaybeUninit::<
                ::neocogi_com::ComRaw<self::ISerializerAbi>,
            >::uninit();
            let __nc_this_0 = self.__nc_pointer.as_raw();
            let __nc_status_0 = unsafe {
                ((*__nc_this_0.vtable)
                    .create_serializer)(
                    __nc_this_0.object,
                    __nc_arg_0,
                    __nc_arg_1,
                    __nc_arg_2,
                    __nc_arg_3,
                    __nc_out_0.as_mut_ptr(),
                )
            };
            if __nc_status_0.is_err() {
                return ::core::result::Result::Err(__nc_status_0);
            }
            let __nc_result_0 = (unsafe {
                <::neocogi_com::ComBox<
                    self::ISerializerAbi,
                >>::__nc_from_raw_owned_checked(__nc_out_0.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_0)
        }
        /// Copies the complete borrowed document into a new bounded source session.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:128:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializationFormat::create_deserializer`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `input` is passed as a safe readable `:: core :: primitive :: u8` buffer plus offset.
        ///- `max_bytes` is passed by value as `:: core :: primitive :: u64`.
        ///- `max_elements` is passed by value as `:: core :: primitive :: u32`.
        ///- `max_depth` is passed by value as `:: core :: primitive :: u32`.
        ///- `max_values` is passed by value as `:: core :: primitive :: u64`.
        ///
        ///On success, returns `:: neocogi_com :: ComBox < self :: IDeserializerAbi >`. The caller receives the sole exclusive owner; interface queries cannot duplicate it.
        pub fn create_deserializer(
            &self,
            __nc_arg_0_0: ::neocogi_com::NcCopyFromBufferOffset<
                '_,
                ::core::primitive::u8,
            >,
            __nc_arg_1_0: ::core::primitive::u64,
            __nc_arg_2_0: ::core::primitive::u32,
            __nc_arg_3_0: ::core::primitive::u32,
            __nc_arg_4: ::core::primitive::u64,
        ) -> ::core::result::Result<
            ::neocogi_com::ComBox<self::IDeserializerAbi>,
            ::neocogi_com::NcStatus,
        > {
            let __nc_arg_0_0 = ::neocogi_com::EncodedInput::new(__nc_arg_0_0)?;
            let __nc_arg_1_0 = ::neocogi_com::encode_inline(&__nc_arg_1_0)?;
            let __nc_arg_2_0 = ::neocogi_com::encode_inline(&__nc_arg_2_0)?;
            let __nc_arg_3_0 = ::neocogi_com::encode_inline(&__nc_arg_3_0)?;
            let __nc_arg_4 = ::neocogi_com::encode_inline(&__nc_arg_4)?;
            let mut __nc_out_1 = ::core::mem::MaybeUninit::<
                ::neocogi_com::ComRaw<self::IDeserializerAbi>,
            >::uninit();
            let __nc_this_1 = self.__nc_pointer.as_raw();
            let __nc_status_1 = unsafe {
                ((*__nc_this_1.vtable)
                    .create_deserializer)(
                    __nc_this_1.object,
                    __nc_arg_0_0.offset(),
                    __nc_arg_0_0.len(),
                    __nc_arg_0_0.as_ptr(),
                    __nc_arg_1_0,
                    __nc_arg_2_0,
                    __nc_arg_3_0,
                    __nc_arg_4,
                    __nc_out_1.as_mut_ptr(),
                )
            };
            if __nc_status_1.is_err() {
                return ::core::result::Result::Err(__nc_status_1);
            }
            let __nc_result_1 = (unsafe {
                <::neocogi_com::ComBox<
                    self::IDeserializerAbi,
                >>::__nc_from_raw_owned_checked(__nc_out_1.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_1)
        }
    }
};
impl ISerializationFormatAbi {
    ///Canonical validation form of this interface's typed ABI contract.
    pub const ABI_SIGNATURE: ::neocogi_com::NcAbiSignature = ::neocogi_com::NcAbiSignature::from_static(
        b"Q\x94\x92\xAB> \x82\x10'\xF7\xB2`\x945\xB2\xED:\x9A\xEFf\xBC\xB8o\xE1|b\x8F\xFD\\\xF5\xF2\xAA",
    );
    ///Static descriptors for this interface's ABI methods.
    pub const METHOD_DESCRIPTORS: [::neocogi_com::NcMethodDescriptor; 2usize] = [
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("create_serializer"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 4u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("max_bytes"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::u64>(),
                    ),
                },
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("max_elements"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
                    ),
                },
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("max_depth"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
                    ),
                },
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("max_values"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::u64>(),
                    ),
                },
            ] as &'static [::neocogi_com::NcMethodParamDescriptor])
                .as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(::neocogi_com::NcAbiPassing::ComBoxOut {
                    mode: <::neocogi_com::ThreadLocal as ::neocogi_com::ThreadingModel>::ABI_THREADING_MODE,
                }),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_interface(
                    <self::ISerializerAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("create_deserializer"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 5u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("input"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::CopyFromBufferOffset,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_plain_type::<::core::primitive::u8>(),
                    ),
                },
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("max_bytes"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::u64>(),
                    ),
                },
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("max_elements"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
                    ),
                },
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("max_depth"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
                    ),
                },
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("max_values"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::u64>(),
                    ),
                },
            ] as &'static [::neocogi_com::NcMethodParamDescriptor])
                .as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(::neocogi_com::NcAbiPassing::ComBoxOut {
                    mode: <::neocogi_com::ThreadLocal as ::neocogi_com::ThreadingModel>::ABI_THREADING_MODE,
                }),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_interface(
                    <self::IDeserializerAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
                ),
            },
        },
    ];
    ///Static ABI descriptor for this interface.
    pub const INTERFACE_DESCRIPTOR: ::neocogi_com::NcInterfaceDescriptor = ::neocogi_com::NcInterfaceDescriptor {
        interface_id: <Self as ::neocogi_com::ComInterface>::IID,
        qualified_name: ::neocogi_com::NcStaticUtf8::from_static(
            "neocogi.com.serialization.ISerializationFormat",
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
/// Byte output of a completed serializer, independent of its document format.
/// IDL source: `neocogi/com/serialization/formats.ncidl:116:11` (module `neocogi.com.serialization`).
pub trait ISerializationOutput {
    /// Reports the complete document length after successful finish.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:118:8` (module `neocogi.com.serialization`).
    fn byte_len(
        &self,
    ) -> ::core::result::Result<::core::primitive::u64, ::neocogi_com::NcStatus>;
    /// Copies the complete document; insufficient capacity publishes no prefix.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:120:8` (module `neocogi.com.serialization`).
    fn copy_to(
        &self,
        output: ::neocogi_com::NcOutputWriter<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<::core::primitive::u64, ::neocogi_com::NcStatus>;
}
///Interface marker for [`ISerializationOutput`]; handles carry its object/table pair.
pub enum ISerializationOutputAbi {}
#[repr(C)]
///C-compatible vtable for [`ISerializationOutput`].
pub struct ISerializationOutputVTable {
    /// Common identity, query and reference-count slots.
    pub unknown: ::neocogi_com::IUnknownVTable,
    /// Reports the complete document length after successful finish.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:118:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializationOutput::byte_len`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u64 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub byte_len: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Copies the complete document; insufficient capacity publishes no prefix.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:120:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializationOutput::copy_to`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `output` uses ABI storage `* mut < :: core :: primitive :: u8 as :: neocogi_com :: ComValue > :: Wire` for `:: neocogi_com :: NcOutputWriter < '_ , :: core :: primitive :: u8 >`. The writable buffer pointer is accompanied by `offset: u64`, `capacity: u64`, and a separate `written: *mut u64`. Old contents are not input. Only the successfully reported initialized prefix may be read; count and element storage must not alias.
    ///
    ///On success, `out` receives `< :: core :: primitive :: u64 as :: neocogi_com :: ComValue > :: Wire`.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub copy_to: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_offset: ::core::primitive::u64,
        __nc_arg_0_len: ::core::primitive::u64,
        __nc_arg_0_ptr: *mut <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_0_written: *mut ::core::primitive::u64,
        __nc_out: *mut <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
}
unsafe impl ::neocogi_com::ComInterface for ISerializationOutputAbi {
    const IID: ::neocogi_com::NcGuid = ::neocogi_com::NcGuid {
        data1: 2336143355u32,
        data2: 4644u16,
        data3: 21403u16,
        data4: [188u8, 180u8, 231u8, 2u8, 75u8, 94u8, 81u8, 217u8],
    };
    const INTERFACE_ID: ::neocogi_com::NcInterfaceId = ::neocogi_com::NcInterfaceId::new(
        ::neocogi_com::NcGuid {
            data1: 2336143355u32,
            data2: 4644u16,
            data3: 21403u16,
            data4: [188u8, 180u8, 231u8, 2u8, 75u8, 94u8, 81u8, 217u8],
        },
        ISerializationOutputAbi::ABI_SIGNATURE,
    );
    type VTable = ISerializationOutputVTable;
    type Threading = ::neocogi_com::ThreadLocal;
}
#[repr(transparent)]
///Borrowed static method view for [`ISerializationOutput`].
pub struct ISerializationOutputClient<
    __NcThreading: ::neocogi_com::ThreadingModel = ::neocogi_com::ThreadLocal,
> {
    /// Certified transparent storage supplied only by a live COM pointer borrow.
    __nc_pointer: ::neocogi_com::NcClientPointer<ISerializationOutputAbi, __NcThreading>,
}
unsafe impl ::neocogi_com::ComClientInterface for ISerializationOutputAbi {
    type Client<__NcThreading: ::neocogi_com::ThreadingModel> = ISerializationOutputClient<
        __NcThreading,
    >;
    fn __nc_client_ref<__NcThreading: ::neocogi_com::ThreadingModel>(
        pointer: &::neocogi_com::NcClientPointer<Self, __NcThreading>,
    ) -> &Self::Client<__NcThreading> {
        unsafe {
            &*(::core::ptr::from_ref(pointer)
                as *const ISerializationOutputClient<__NcThreading>)
        }
    }
    fn __nc_client_mut<__NcThreading: ::neocogi_com::ThreadingModel>(
        pointer: &mut ::neocogi_com::NcClientPointer<Self, __NcThreading>,
    ) -> &mut Self::Client<__NcThreading> {
        unsafe {
            &mut *(::core::ptr::from_mut(pointer)
                as *mut ISerializationOutputClient<__NcThreading>)
        }
    }
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
    > ISerializationOutputClient<__NcThreading> {
        /// Reports the complete document length after successful finish.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:118:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializationOutput::byte_len`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: core :: primitive :: u64` from the ABI out slot.
        pub fn byte_len(
            &self,
        ) -> ::core::result::Result<::core::primitive::u64, ::neocogi_com::NcStatus> {
            let mut __nc_out_0 = ::core::mem::MaybeUninit::<
                <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_0 = self.__nc_pointer.as_raw();
            let __nc_status_0 = unsafe {
                ((*__nc_this_0.vtable)
                    .byte_len)(__nc_this_0.object, __nc_out_0.as_mut_ptr())
            };
            if __nc_status_0.is_err() {
                return ::core::result::Result::Err(__nc_status_0);
            }
            let __nc_result_0 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::u64,
                >(__nc_out_0.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_0)
        }
        /// Copies the complete document; insufficient capacity publishes no prefix.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:120:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializationOutput::copy_to`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `output` supplies output storage for `:: core :: primitive :: u8` through [`::neocogi_com::NcOutput`]. See its lifecycle contract for acquisition, preparation and publication.
        ///
        ///On success, returns `:: core :: primitive :: u64` from the ABI out slot.
        pub fn copy_to(
            &self,
            __nc_arg_0: ::neocogi_com::NcOutput<'_, ::core::primitive::u8>,
        ) -> ::core::result::Result<::core::primitive::u64, ::neocogi_com::NcStatus> {
            let mut __nc_arg_0 = __nc_arg_0.prepare()?;
            let mut __nc_out_1 = ::core::mem::MaybeUninit::<
                <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
            >::uninit();
            let __nc_this_1 = self.__nc_pointer.as_raw();
            let __nc_status_1 = unsafe {
                ((*__nc_this_1.vtable)
                    .copy_to)(
                    __nc_this_1.object,
                    __nc_arg_0.offset(),
                    __nc_arg_0.capacity(),
                    __nc_arg_0.as_mut_ptr(),
                    __nc_arg_0.written_ptr(),
                    __nc_out_1.as_mut_ptr(),
                )
            };
            if __nc_status_1.is_err() {
                return ::core::result::Result::Err(__nc_status_1);
            }
            let __nc_result_1 = (unsafe {
                ::neocogi_com::decode_inline::<
                    ::core::primitive::u64,
                >(__nc_out_1.assume_init())
            })?;
            (unsafe { __nc_arg_0.convert_output() })?;
            __nc_arg_0.publish();
            ::core::result::Result::Ok(__nc_result_1)
        }
    }
};
impl ISerializationOutputAbi {
    ///Canonical validation form of this interface's typed ABI contract.
    pub const ABI_SIGNATURE: ::neocogi_com::NcAbiSignature = ::neocogi_com::NcAbiSignature::from_static(
        b"\xD5\xA2\xA4a\xE0\xAE\xB7\xC2\x90\xCD4Gl\x8F\x0B(\xE6\xEC\xE4\xDD\x0FK\xDC\xD1x[W\xA1h\x9B\xB3j",
    );
    ///Static descriptors for this interface's ABI methods.
    pub const METHOD_DESCRIPTORS: [::neocogi_com::NcMethodDescriptor; 2usize] = [
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("byte_len"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::u64>(),
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("copy_to"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("output"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::OutputBuffer,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_plain_type::<::core::primitive::u8>(),
                    ),
                },
            ] as &'static [::neocogi_com::NcMethodParamDescriptor])
                .as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Out,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::__nc_abi_type::<::core::primitive::u64>(),
                ),
            },
        },
    ];
    ///Static ABI descriptor for this interface.
    pub const INTERFACE_DESCRIPTOR: ::neocogi_com::NcInterfaceDescriptor = ::neocogi_com::NcInterfaceDescriptor {
        interface_id: <Self as ::neocogi_com::ComInterface>::IID,
        qualified_name: ::neocogi_com::NcStaticUtf8::from_static(
            "neocogi.com.serialization.ISerializationOutput",
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
/// One serialization plugin bundles its enabled codecs, with NCB always available.
/// IDL source: `neocogi/com/serialization/formats.ncidl:132:11` (module `neocogi.com.serialization`).
pub trait ISerializationPlugin {
    /// Returns the unconditional default NCB format factory.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:134:8` (module `neocogi.com.serialization`).
    fn default_format(
        &self,
    ) -> ::core::result::Result<
        ::neocogi_com::ComPtr<self::ISerializationFormatAbi>,
        ::neocogi_com::NcStatus,
    >;
    /// Selects ncb or json; disabled and unknown names return NC_E_NOINTERFACE.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:136:8` (module `neocogi.com.serialization`).
    fn format(
        &self,
        name: ::neocogi_com::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<
        ::neocogi_com::ComPtr<self::ISerializationFormatAbi>,
        ::neocogi_com::NcStatus,
    >;
}
///Interface marker for [`ISerializationPlugin`]; handles carry its object/table pair.
pub enum ISerializationPluginAbi {}
#[repr(C)]
///C-compatible vtable for [`ISerializationPlugin`].
pub struct ISerializationPluginVTable {
    /// Common identity, query and reference-count slots.
    pub unknown: ::neocogi_com::IUnknownVTable,
    /// Returns the unconditional default NCB format factory.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:134:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializationPlugin::default_format`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, `out` receives `:: neocogi_com :: ComRaw < self :: ISerializationFormatAbi >` for `:: neocogi_com :: ComPtr < self :: ISerializationFormatAbi >`. The caller receives one shared owner and releases it when finished.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub default_format: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_out: *mut ::neocogi_com::ComRaw<self::ISerializationFormatAbi>,
    ) -> ::neocogi_com::NcStatus,
    /// Selects ncb or json; disabled and unknown names return NC_E_NOINTERFACE.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:136:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializationPlugin::format`].
    ///
    ///Returns `NcStatus`; the `out` pointer is initialized only when the returned status is successful.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table; `out` must be valid writable storage. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `name` uses ABI storage `* const < :: core :: primitive :: u8 as :: neocogi_com :: ComValue > :: Wire` for `:: neocogi_com :: NcCopyFromBufferOffset < '_ , :: core :: primitive :: u8 >`. The readable buffer pointer is accompanied by `offset: u64` and `len: u64`.
    ///
    ///On success, `out` receives `:: neocogi_com :: ComRaw < self :: ISerializationFormatAbi >` for `:: neocogi_com :: ComPtr < self :: ISerializationFormatAbi >`. The caller receives one shared owner and releases it when finished.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub format: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_offset: ::core::primitive::u64,
        __nc_arg_0_len: ::core::primitive::u64,
        __nc_arg_0_ptr: *const <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
        __nc_out: *mut ::neocogi_com::ComRaw<self::ISerializationFormatAbi>,
    ) -> ::neocogi_com::NcStatus,
}
unsafe impl ::neocogi_com::ComInterface for ISerializationPluginAbi {
    const IID: ::neocogi_com::NcGuid = ::neocogi_com::NcGuid {
        data1: 2947607689u32,
        data2: 56026u16,
        data3: 22309u16,
        data4: [151u8, 196u8, 39u8, 112u8, 37u8, 75u8, 55u8, 139u8],
    };
    const INTERFACE_ID: ::neocogi_com::NcInterfaceId = ::neocogi_com::NcInterfaceId::new(
        ::neocogi_com::NcGuid {
            data1: 2947607689u32,
            data2: 56026u16,
            data3: 22309u16,
            data4: [151u8, 196u8, 39u8, 112u8, 37u8, 75u8, 55u8, 139u8],
        },
        ISerializationPluginAbi::ABI_SIGNATURE,
    );
    type VTable = ISerializationPluginVTable;
    type Threading = ::neocogi_com::ThreadLocal;
}
#[repr(transparent)]
///Borrowed static method view for [`ISerializationPlugin`].
pub struct ISerializationPluginClient<
    __NcThreading: ::neocogi_com::ThreadingModel = ::neocogi_com::ThreadLocal,
> {
    /// Certified transparent storage supplied only by a live COM pointer borrow.
    __nc_pointer: ::neocogi_com::NcClientPointer<ISerializationPluginAbi, __NcThreading>,
}
unsafe impl ::neocogi_com::ComClientInterface for ISerializationPluginAbi {
    type Client<__NcThreading: ::neocogi_com::ThreadingModel> = ISerializationPluginClient<
        __NcThreading,
    >;
    fn __nc_client_ref<__NcThreading: ::neocogi_com::ThreadingModel>(
        pointer: &::neocogi_com::NcClientPointer<Self, __NcThreading>,
    ) -> &Self::Client<__NcThreading> {
        unsafe {
            &*(::core::ptr::from_ref(pointer)
                as *const ISerializationPluginClient<__NcThreading>)
        }
    }
    fn __nc_client_mut<__NcThreading: ::neocogi_com::ThreadingModel>(
        pointer: &mut ::neocogi_com::NcClientPointer<Self, __NcThreading>,
    ) -> &mut Self::Client<__NcThreading> {
        unsafe {
            &mut *(::core::ptr::from_mut(pointer)
                as *mut ISerializationPluginClient<__NcThreading>)
        }
    }
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
    > ISerializationPluginClient<__NcThreading> {
        /// Returns the unconditional default NCB format factory.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:134:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializationPlugin::default_format`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `:: neocogi_com :: ComPtr < self :: ISerializationFormatAbi >`. The caller receives one shared owner and releases it when finished.
        pub fn default_format(
            &self,
        ) -> ::core::result::Result<
            ::neocogi_com::ComPtr<self::ISerializationFormatAbi>,
            ::neocogi_com::NcStatus,
        > {
            let mut __nc_out_0 = ::core::mem::MaybeUninit::<
                ::neocogi_com::ComRaw<self::ISerializationFormatAbi>,
            >::uninit();
            let __nc_this_0 = self.__nc_pointer.as_raw();
            let __nc_status_0 = unsafe {
                ((*__nc_this_0.vtable)
                    .default_format)(__nc_this_0.object, __nc_out_0.as_mut_ptr())
            };
            if __nc_status_0.is_err() {
                return ::core::result::Result::Err(__nc_status_0);
            }
            let __nc_result_0 = (unsafe {
                <::neocogi_com::ComPtr<
                    self::ISerializationFormatAbi,
                >>::__nc_from_raw_owned_checked(__nc_out_0.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_0)
        }
        /// Selects ncb or json; disabled and unknown names return NC_E_NOINTERFACE.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:136:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializationPlugin::format`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper creates an uninitialized ABI out slot, invokes the raw vtable function, and reads the out value only when the returned status is successful.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `name` is passed as a safe readable `:: core :: primitive :: u8` buffer plus offset.
        ///
        ///On success, returns `:: neocogi_com :: ComPtr < self :: ISerializationFormatAbi >`. The caller receives one shared owner and releases it when finished.
        pub fn format(
            &self,
            __nc_arg_0: ::neocogi_com::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
        ) -> ::core::result::Result<
            ::neocogi_com::ComPtr<self::ISerializationFormatAbi>,
            ::neocogi_com::NcStatus,
        > {
            let __nc_arg_0 = ::neocogi_com::EncodedInput::new(__nc_arg_0)?;
            let mut __nc_out_1 = ::core::mem::MaybeUninit::<
                ::neocogi_com::ComRaw<self::ISerializationFormatAbi>,
            >::uninit();
            let __nc_this_1 = self.__nc_pointer.as_raw();
            let __nc_status_1 = unsafe {
                ((*__nc_this_1.vtable)
                    .format)(
                    __nc_this_1.object,
                    __nc_arg_0.offset(),
                    __nc_arg_0.len(),
                    __nc_arg_0.as_ptr(),
                    __nc_out_1.as_mut_ptr(),
                )
            };
            if __nc_status_1.is_err() {
                return ::core::result::Result::Err(__nc_status_1);
            }
            let __nc_result_1 = (unsafe {
                <::neocogi_com::ComPtr<
                    self::ISerializationFormatAbi,
                >>::__nc_from_raw_owned_checked(__nc_out_1.assume_init())
            })?;
            ::core::result::Result::Ok(__nc_result_1)
        }
    }
};
impl ISerializationPluginAbi {
    ///Canonical validation form of this interface's typed ABI contract.
    pub const ABI_SIGNATURE: ::neocogi_com::NcAbiSignature = ::neocogi_com::NcAbiSignature::from_static(
        b"^v3\x01\xBA\xFB;\x9D|\xE5\xAA\x02\x02ih\xA3\xEF\x94\x9Dm5\x17|k\xF7?J\xB5\xC8\"?\x14",
    );
    ///Static descriptors for this interface's ABI methods.
    pub const METHOD_DESCRIPTORS: [::neocogi_com::NcMethodDescriptor; 2usize] = [
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("default_format"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(::neocogi_com::NcAbiPassing::ComPtrOut {
                    mode: <::neocogi_com::ThreadLocal as ::neocogi_com::ThreadingModel>::ABI_THREADING_MODE,
                }),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_interface(
                    <self::ISerializationFormatAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
                ),
            },
        },
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("format"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_SHARED,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("name"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::CopyFromBufferOffset,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_plain_type::<::core::primitive::u8>(),
                    ),
                },
            ] as &'static [::neocogi_com::NcMethodParamDescriptor])
                .as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(::neocogi_com::NcAbiPassing::ComPtrOut {
                    mode: <::neocogi_com::ThreadLocal as ::neocogi_com::ThreadingModel>::ABI_THREADING_MODE,
                }),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_interface(
                    <self::ISerializationFormatAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
                ),
            },
        },
    ];
    ///Static ABI descriptor for this interface.
    pub const INTERFACE_DESCRIPTOR: ::neocogi_com::NcInterfaceDescriptor = ::neocogi_com::NcInterfaceDescriptor {
        interface_id: <Self as ::neocogi_com::ComInterface>::IID,
        qualified_name: ::neocogi_com::NcStaticUtf8::from_static(
            "neocogi.com.serialization.ISerializationPlugin",
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
/// A mutable format session. Failures poison it; finish commits one complete document.
/// IDL source: `neocogi/com/serialization/formats.ncidl:2:11` (module `neocogi.com.serialization`).
pub trait ISerializer {
    /// Writes a unit value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:4:8` (module `neocogi.com.serialization`).
    fn serialize_unit(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Writes a bool value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:6:8` (module `neocogi.com.serialization`).
    fn serialize_bool(
        &mut self,
        value: ::core::primitive::bool,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Writes a u8 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:8:8` (module `neocogi.com.serialization`).
    fn serialize_u8(
        &mut self,
        value: ::core::primitive::u8,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Writes a u16 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:10:8` (module `neocogi.com.serialization`).
    fn serialize_u16(
        &mut self,
        value: ::core::primitive::u16,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Writes a u32 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:12:8` (module `neocogi.com.serialization`).
    fn serialize_u32(
        &mut self,
        value: ::core::primitive::u32,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Writes a u64 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:14:8` (module `neocogi.com.serialization`).
    fn serialize_u64(
        &mut self,
        value: ::core::primitive::u64,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Writes a i8 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:16:8` (module `neocogi.com.serialization`).
    fn serialize_i8(
        &mut self,
        value: ::core::primitive::i8,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Writes a i16 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:18:8` (module `neocogi.com.serialization`).
    fn serialize_i16(
        &mut self,
        value: ::core::primitive::i16,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Writes a i32 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:20:8` (module `neocogi.com.serialization`).
    fn serialize_i32(
        &mut self,
        value: ::core::primitive::i32,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Writes a i64 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:22:8` (module `neocogi.com.serialization`).
    fn serialize_i64(
        &mut self,
        value: ::core::primitive::i64,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Writes a f32 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:24:8` (module `neocogi.com.serialization`).
    fn serialize_f32(
        &mut self,
        value: ::core::primitive::f32,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Writes a f64 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:26:8` (module `neocogi.com.serialization`).
    fn serialize_f64(
        &mut self,
        value: ::core::primitive::f64,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Copies one UTF-8 string during this call.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:28:8` (module `neocogi.com.serialization`).
    fn serialize_string(
        &mut self,
        value: ::neocogi_com::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Opens optional presence; the present child follows before end_option.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:30:8` (module `neocogi.com.serialization`).
    fn begin_option(
        &mut self,
        present: ::core::primitive::bool,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Completes the optional value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:32:8` (module `neocogi.com.serialization`).
    fn end_option(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Opens a named record containing exactly fields members.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:34:8` (module `neocogi.com.serialization`).
    fn begin_record(
        &mut self,
        fields: ::core::primitive::u32,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Selects the next named field before writing its typed value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:36:8` (module `neocogi.com.serialization`).
    fn field(
        &mut self,
        name: ::neocogi_com::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Completes the record.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:38:8` (module `neocogi.com.serialization`).
    fn end_record(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Opens an ordered collection with its complete element count.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:40:8` (module `neocogi.com.serialization`).
    fn begin_sequence(
        &mut self,
        length: ::core::primitive::u32,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Completes the collection.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:42:8` (module `neocogi.com.serialization`).
    fn end_sequence(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Opens a map; keys and values alternate for exactly pairs entries.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:44:8` (module `neocogi.com.serialization`).
    fn begin_map(
        &mut self,
        pairs: ::core::primitive::u32,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Completes the map.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:46:8` (module `neocogi.com.serialization`).
    fn end_map(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Opens a case identified by both declaration ordinal and authored name.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:48:8` (module `neocogi.com.serialization`).
    fn begin_variant(
        &mut self,
        index: ::core::primitive::u32,
        name: ::neocogi_com::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Completes the selected case and its optional payload.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:50:8` (module `neocogi.com.serialization`).
    fn end_variant(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Marks a schema failure; no later operation may succeed.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:52:8` (module `neocogi.com.serialization`).
    fn fail(
        &mut self,
        message: ::neocogi_com::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
    /// Commits a complete document; repeated finish and further writes fail.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:54:8` (module `neocogi.com.serialization`).
    fn finish(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus>;
}
///Interface marker for [`ISerializer`]; handles carry its object/table pair.
pub enum ISerializerAbi {}
#[repr(C)]
///C-compatible vtable for [`ISerializer`].
pub struct ISerializerVTable {
    /// Common identity, query and reference-count slots.
    pub unknown: ::neocogi_com::IUnknownVTable,
    /// Writes a unit value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:4:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::serialize_unit`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub serialize_unit: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
    ) -> ::neocogi_com::NcStatus,
    /// Writes a bool value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:6:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::serialize_bool`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `value` uses ABI storage `< :: core :: primitive :: bool as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: bool`. Passed by value.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub serialize_bool: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0: <::core::primitive::bool as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Writes a u8 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:8:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::serialize_u8`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `value` uses ABI storage `< :: core :: primitive :: u8 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: u8`. Passed by value.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub serialize_u8: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_0: <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Writes a u16 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:10:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::serialize_u16`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `value` uses ABI storage `< :: core :: primitive :: u16 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: u16`. Passed by value.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub serialize_u16: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_1: <::core::primitive::u16 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Writes a u32 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:12:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::serialize_u32`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `value` uses ABI storage `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: u32`. Passed by value.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub serialize_u32: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_2: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Writes a u64 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:14:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::serialize_u64`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `value` uses ABI storage `< :: core :: primitive :: u64 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: u64`. Passed by value.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub serialize_u64: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_3: <::core::primitive::u64 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Writes a i8 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:16:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::serialize_i8`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `value` uses ABI storage `< :: core :: primitive :: i8 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: i8`. Passed by value.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub serialize_i8: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_4: <::core::primitive::i8 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Writes a i16 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:18:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::serialize_i16`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `value` uses ABI storage `< :: core :: primitive :: i16 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: i16`. Passed by value.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub serialize_i16: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_5: <::core::primitive::i16 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Writes a i32 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:20:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::serialize_i32`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `value` uses ABI storage `< :: core :: primitive :: i32 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: i32`. Passed by value.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub serialize_i32: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_6: <::core::primitive::i32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Writes a i64 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:22:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::serialize_i64`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `value` uses ABI storage `< :: core :: primitive :: i64 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: i64`. Passed by value.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub serialize_i64: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_7: <::core::primitive::i64 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Writes a f32 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:24:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::serialize_f32`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `value` uses ABI storage `< :: core :: primitive :: f32 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: f32`. Passed by value.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub serialize_f32: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_8: <::core::primitive::f32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Writes a f64 value with its declared width.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:26:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::serialize_f64`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `value` uses ABI storage `< :: core :: primitive :: f64 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: f64`. Passed by value.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub serialize_f64: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_9: <::core::primitive::f64 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Copies one UTF-8 string during this call.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:28:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::serialize_string`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `value` uses ABI storage `* const < :: core :: primitive :: u8 as :: neocogi_com :: ComValue > :: Wire` for `:: neocogi_com :: NcCopyFromBufferOffset < '_ , :: core :: primitive :: u8 >`. The readable buffer pointer is accompanied by `offset: u64` and `len: u64`.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub serialize_string: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_10_offset: ::core::primitive::u64,
        __nc_arg_0_10_len: ::core::primitive::u64,
        __nc_arg_0_10_ptr: *const <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Opens optional presence; the present child follows before end_option.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:30:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::begin_option`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `present` uses ABI storage `< :: core :: primitive :: bool as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: bool`. Passed by value.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub begin_option: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_11: <::core::primitive::bool as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Completes the optional value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:32:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::end_option`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub end_option: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
    ) -> ::neocogi_com::NcStatus,
    /// Opens a named record containing exactly fields members.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:34:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::begin_record`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `fields` uses ABI storage `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: u32`. Passed by value.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub begin_record: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_12: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Selects the next named field before writing its typed value.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:36:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::field`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `name` uses ABI storage `* const < :: core :: primitive :: u8 as :: neocogi_com :: ComValue > :: Wire` for `:: neocogi_com :: NcCopyFromBufferOffset < '_ , :: core :: primitive :: u8 >`. The readable buffer pointer is accompanied by `offset: u64` and `len: u64`.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub field: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_13_offset: ::core::primitive::u64,
        __nc_arg_0_13_len: ::core::primitive::u64,
        __nc_arg_0_13_ptr: *const <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Completes the record.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:38:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::end_record`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub end_record: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
    ) -> ::neocogi_com::NcStatus,
    /// Opens an ordered collection with its complete element count.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:40:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::begin_sequence`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `length` uses ABI storage `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: u32`. Passed by value.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub begin_sequence: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_14: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Completes the collection.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:42:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::end_sequence`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub end_sequence: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
    ) -> ::neocogi_com::NcStatus,
    /// Opens a map; keys and values alternate for exactly pairs entries.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:44:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::begin_map`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `pairs` uses ABI storage `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: u32`. Passed by value.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub begin_map: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_15: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Completes the map.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:46:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::end_map`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub end_map: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
    ) -> ::neocogi_com::NcStatus,
    /// Opens a case identified by both declaration ordinal and authored name.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:48:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::begin_variant`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `index` uses ABI storage `< :: core :: primitive :: u32 as :: neocogi_com :: ComValue > :: Wire` for `:: core :: primitive :: u32`. Passed by value.
    ///- `name` uses ABI storage `* const < :: core :: primitive :: u8 as :: neocogi_com :: ComValue > :: Wire` for `:: neocogi_com :: NcCopyFromBufferOffset < '_ , :: core :: primitive :: u8 >`. The readable buffer pointer is accompanied by `offset: u64` and `len: u64`.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub begin_variant: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_16: <::core::primitive::u32 as ::neocogi_com::ComValue>::Wire,
        __nc_arg_1_offset: ::core::primitive::u64,
        __nc_arg_1_len: ::core::primitive::u64,
        __nc_arg_1_ptr: *const <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Completes the selected case and its optional payload.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:50:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::end_variant`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub end_variant: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
    ) -> ::neocogi_com::NcStatus,
    /// Marks a schema failure; no later operation may succeed.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:52:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::fail`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///- `message` uses ABI storage `* const < :: core :: primitive :: u8 as :: neocogi_com :: ComValue > :: Wire` for `:: neocogi_com :: NcCopyFromBufferOffset < '_ , :: core :: primitive :: u8 >`. The readable buffer pointer is accompanied by `offset: u64` and `len: u64`.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub fail: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
        __nc_arg_0_17_offset: ::core::primitive::u64,
        __nc_arg_0_17_len: ::core::primitive::u64,
        __nc_arg_0_17_ptr: *const <::core::primitive::u8 as ::neocogi_com::ComValue>::Wire,
    ) -> ::neocogi_com::NcStatus,
    /// Commits a complete document; repeated finish and further writes fail.
    /// IDL source: `neocogi/com/serialization/formats.ncidl:54:8` (module `neocogi.com.serialization`).
    ///Raw COM ABI vtable slot for [`ISerializer::finish`].
    ///
    ///Returns `NcStatus`; success is represented by the returned status and there is no `out` pointer.
    ///
    ///# Safety
    ///
    ///`this` must be the live allocation address paired with this table. Arguments lowered from `ComRef` must be valid borrowed object/table pairs for the duration of the call. Both fields must be non-null.
    ///
    ///On success, no output value is written.
    #[allow(
        clippy::type_complexity,
        reason = "Raw vtable slots retain the exact flattened IDL ABI."
    )]
    pub finish: unsafe extern "C" fn(
        __nc_this: *mut ::core::ffi::c_void,
    ) -> ::neocogi_com::NcStatus,
}
unsafe impl ::neocogi_com::ComInterface for ISerializerAbi {
    const IID: ::neocogi_com::NcGuid = ::neocogi_com::NcGuid {
        data1: 1280412527u32,
        data2: 27648u16,
        data3: 22052u16,
        data4: [170u8, 83u8, 45u8, 140u8, 69u8, 140u8, 91u8, 193u8],
    };
    const INTERFACE_ID: ::neocogi_com::NcInterfaceId = ::neocogi_com::NcInterfaceId::new(
        ::neocogi_com::NcGuid {
            data1: 1280412527u32,
            data2: 27648u16,
            data3: 22052u16,
            data4: [170u8, 83u8, 45u8, 140u8, 69u8, 140u8, 91u8, 193u8],
        },
        ISerializerAbi::ABI_SIGNATURE,
    );
    type VTable = ISerializerVTable;
    type Threading = ::neocogi_com::ThreadLocal;
}
#[repr(transparent)]
///Borrowed static method view for [`ISerializer`].
pub struct ISerializerClient<
    __NcThreading: ::neocogi_com::ThreadingModel = ::neocogi_com::ThreadLocal,
> {
    /// Certified transparent storage supplied only by a live COM pointer borrow.
    __nc_pointer: ::neocogi_com::NcClientPointer<ISerializerAbi, __NcThreading>,
}
unsafe impl ::neocogi_com::ComClientInterface for ISerializerAbi {
    type Client<__NcThreading: ::neocogi_com::ThreadingModel> = ISerializerClient<
        __NcThreading,
    >;
    fn __nc_client_ref<__NcThreading: ::neocogi_com::ThreadingModel>(
        pointer: &::neocogi_com::NcClientPointer<Self, __NcThreading>,
    ) -> &Self::Client<__NcThreading> {
        unsafe {
            &*(::core::ptr::from_ref(pointer) as *const ISerializerClient<__NcThreading>)
        }
    }
    fn __nc_client_mut<__NcThreading: ::neocogi_com::ThreadingModel>(
        pointer: &mut ::neocogi_com::NcClientPointer<Self, __NcThreading>,
    ) -> &mut Self::Client<__NcThreading> {
        unsafe {
            &mut *(::core::ptr::from_mut(pointer)
                as *mut ISerializerClient<__NcThreading>)
        }
    }
}
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
    impl<__NcThreading: ::neocogi_com::ThreadingModel> ISerializerClient<__NcThreading> {
        /// Writes a unit value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:4:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::serialize_unit`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `()`.
        pub fn serialize_unit(
            &mut self,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_this_0 = self.__nc_pointer.as_raw();
            let __nc_status_0 = unsafe {
                ((*__nc_this_0.vtable).serialize_unit)(__nc_this_0.object)
            };
            if __nc_status_0.is_err() {
                return ::core::result::Result::Err(__nc_status_0);
            }
            let __nc_result_0 = ();
            ::core::result::Result::Ok(__nc_result_0)
        }
        /// Writes a bool value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:6:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::serialize_bool`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `value` is passed by value as `:: core :: primitive :: bool`.
        ///
        ///On success, returns `()`.
        pub fn serialize_bool(
            &mut self,
            __nc_arg_0: ::core::primitive::bool,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0 = ::neocogi_com::encode_inline(&__nc_arg_0)?;
            let __nc_this_1 = self.__nc_pointer.as_raw();
            let __nc_status_1 = unsafe {
                ((*__nc_this_1.vtable).serialize_bool)(__nc_this_1.object, __nc_arg_0)
            };
            if __nc_status_1.is_err() {
                return ::core::result::Result::Err(__nc_status_1);
            }
            let __nc_result_1 = ();
            ::core::result::Result::Ok(__nc_result_1)
        }
        /// Writes a u8 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:8:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::serialize_u8`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `value` is passed by value as `:: core :: primitive :: u8`.
        ///
        ///On success, returns `()`.
        pub fn serialize_u8(
            &mut self,
            __nc_arg_0_0: ::core::primitive::u8,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0_0 = ::neocogi_com::encode_inline(&__nc_arg_0_0)?;
            let __nc_this_2 = self.__nc_pointer.as_raw();
            let __nc_status_2 = unsafe {
                ((*__nc_this_2.vtable).serialize_u8)(__nc_this_2.object, __nc_arg_0_0)
            };
            if __nc_status_2.is_err() {
                return ::core::result::Result::Err(__nc_status_2);
            }
            let __nc_result_2 = ();
            ::core::result::Result::Ok(__nc_result_2)
        }
        /// Writes a u16 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:10:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::serialize_u16`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `value` is passed by value as `:: core :: primitive :: u16`.
        ///
        ///On success, returns `()`.
        pub fn serialize_u16(
            &mut self,
            __nc_arg_0_1: ::core::primitive::u16,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0_1 = ::neocogi_com::encode_inline(&__nc_arg_0_1)?;
            let __nc_this_3 = self.__nc_pointer.as_raw();
            let __nc_status_3 = unsafe {
                ((*__nc_this_3.vtable).serialize_u16)(__nc_this_3.object, __nc_arg_0_1)
            };
            if __nc_status_3.is_err() {
                return ::core::result::Result::Err(__nc_status_3);
            }
            let __nc_result_3 = ();
            ::core::result::Result::Ok(__nc_result_3)
        }
        /// Writes a u32 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:12:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::serialize_u32`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `value` is passed by value as `:: core :: primitive :: u32`.
        ///
        ///On success, returns `()`.
        pub fn serialize_u32(
            &mut self,
            __nc_arg_0_2: ::core::primitive::u32,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0_2 = ::neocogi_com::encode_inline(&__nc_arg_0_2)?;
            let __nc_this_4 = self.__nc_pointer.as_raw();
            let __nc_status_4 = unsafe {
                ((*__nc_this_4.vtable).serialize_u32)(__nc_this_4.object, __nc_arg_0_2)
            };
            if __nc_status_4.is_err() {
                return ::core::result::Result::Err(__nc_status_4);
            }
            let __nc_result_4 = ();
            ::core::result::Result::Ok(__nc_result_4)
        }
        /// Writes a u64 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:14:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::serialize_u64`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `value` is passed by value as `:: core :: primitive :: u64`.
        ///
        ///On success, returns `()`.
        pub fn serialize_u64(
            &mut self,
            __nc_arg_0_3: ::core::primitive::u64,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0_3 = ::neocogi_com::encode_inline(&__nc_arg_0_3)?;
            let __nc_this_5 = self.__nc_pointer.as_raw();
            let __nc_status_5 = unsafe {
                ((*__nc_this_5.vtable).serialize_u64)(__nc_this_5.object, __nc_arg_0_3)
            };
            if __nc_status_5.is_err() {
                return ::core::result::Result::Err(__nc_status_5);
            }
            let __nc_result_5 = ();
            ::core::result::Result::Ok(__nc_result_5)
        }
        /// Writes a i8 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:16:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::serialize_i8`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `value` is passed by value as `:: core :: primitive :: i8`.
        ///
        ///On success, returns `()`.
        pub fn serialize_i8(
            &mut self,
            __nc_arg_0_4: ::core::primitive::i8,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0_4 = ::neocogi_com::encode_inline(&__nc_arg_0_4)?;
            let __nc_this_6 = self.__nc_pointer.as_raw();
            let __nc_status_6 = unsafe {
                ((*__nc_this_6.vtable).serialize_i8)(__nc_this_6.object, __nc_arg_0_4)
            };
            if __nc_status_6.is_err() {
                return ::core::result::Result::Err(__nc_status_6);
            }
            let __nc_result_6 = ();
            ::core::result::Result::Ok(__nc_result_6)
        }
        /// Writes a i16 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:18:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::serialize_i16`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `value` is passed by value as `:: core :: primitive :: i16`.
        ///
        ///On success, returns `()`.
        pub fn serialize_i16(
            &mut self,
            __nc_arg_0_5: ::core::primitive::i16,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0_5 = ::neocogi_com::encode_inline(&__nc_arg_0_5)?;
            let __nc_this_7 = self.__nc_pointer.as_raw();
            let __nc_status_7 = unsafe {
                ((*__nc_this_7.vtable).serialize_i16)(__nc_this_7.object, __nc_arg_0_5)
            };
            if __nc_status_7.is_err() {
                return ::core::result::Result::Err(__nc_status_7);
            }
            let __nc_result_7 = ();
            ::core::result::Result::Ok(__nc_result_7)
        }
        /// Writes a i32 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:20:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::serialize_i32`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `value` is passed by value as `:: core :: primitive :: i32`.
        ///
        ///On success, returns `()`.
        pub fn serialize_i32(
            &mut self,
            __nc_arg_0_6: ::core::primitive::i32,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0_6 = ::neocogi_com::encode_inline(&__nc_arg_0_6)?;
            let __nc_this_8 = self.__nc_pointer.as_raw();
            let __nc_status_8 = unsafe {
                ((*__nc_this_8.vtable).serialize_i32)(__nc_this_8.object, __nc_arg_0_6)
            };
            if __nc_status_8.is_err() {
                return ::core::result::Result::Err(__nc_status_8);
            }
            let __nc_result_8 = ();
            ::core::result::Result::Ok(__nc_result_8)
        }
        /// Writes a i64 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:22:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::serialize_i64`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `value` is passed by value as `:: core :: primitive :: i64`.
        ///
        ///On success, returns `()`.
        pub fn serialize_i64(
            &mut self,
            __nc_arg_0_7: ::core::primitive::i64,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0_7 = ::neocogi_com::encode_inline(&__nc_arg_0_7)?;
            let __nc_this_9 = self.__nc_pointer.as_raw();
            let __nc_status_9 = unsafe {
                ((*__nc_this_9.vtable).serialize_i64)(__nc_this_9.object, __nc_arg_0_7)
            };
            if __nc_status_9.is_err() {
                return ::core::result::Result::Err(__nc_status_9);
            }
            let __nc_result_9 = ();
            ::core::result::Result::Ok(__nc_result_9)
        }
        /// Writes a f32 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:24:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::serialize_f32`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `value` is passed by value as `:: core :: primitive :: f32`.
        ///
        ///On success, returns `()`.
        pub fn serialize_f32(
            &mut self,
            __nc_arg_0_8: ::core::primitive::f32,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0_8 = ::neocogi_com::encode_inline(&__nc_arg_0_8)?;
            let __nc_this_10 = self.__nc_pointer.as_raw();
            let __nc_status_10 = unsafe {
                ((*__nc_this_10.vtable).serialize_f32)(__nc_this_10.object, __nc_arg_0_8)
            };
            if __nc_status_10.is_err() {
                return ::core::result::Result::Err(__nc_status_10);
            }
            let __nc_result_10 = ();
            ::core::result::Result::Ok(__nc_result_10)
        }
        /// Writes a f64 value with its declared width.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:26:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::serialize_f64`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `value` is passed by value as `:: core :: primitive :: f64`.
        ///
        ///On success, returns `()`.
        pub fn serialize_f64(
            &mut self,
            __nc_arg_0_9: ::core::primitive::f64,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0_9 = ::neocogi_com::encode_inline(&__nc_arg_0_9)?;
            let __nc_this_11 = self.__nc_pointer.as_raw();
            let __nc_status_11 = unsafe {
                ((*__nc_this_11.vtable).serialize_f64)(__nc_this_11.object, __nc_arg_0_9)
            };
            if __nc_status_11.is_err() {
                return ::core::result::Result::Err(__nc_status_11);
            }
            let __nc_result_11 = ();
            ::core::result::Result::Ok(__nc_result_11)
        }
        /// Copies one UTF-8 string during this call.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:28:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::serialize_string`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `value` is passed as a safe readable `:: core :: primitive :: u8` buffer plus offset.
        ///
        ///On success, returns `()`.
        pub fn serialize_string(
            &mut self,
            __nc_arg_0_10: ::neocogi_com::NcCopyFromBufferOffset<
                '_,
                ::core::primitive::u8,
            >,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0_10 = ::neocogi_com::EncodedInput::new(__nc_arg_0_10)?;
            let __nc_this_12 = self.__nc_pointer.as_raw();
            let __nc_status_12 = unsafe {
                ((*__nc_this_12.vtable)
                    .serialize_string)(
                    __nc_this_12.object,
                    __nc_arg_0_10.offset(),
                    __nc_arg_0_10.len(),
                    __nc_arg_0_10.as_ptr(),
                )
            };
            if __nc_status_12.is_err() {
                return ::core::result::Result::Err(__nc_status_12);
            }
            let __nc_result_12 = ();
            ::core::result::Result::Ok(__nc_result_12)
        }
        /// Opens optional presence; the present child follows before end_option.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:30:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::begin_option`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `present` is passed by value as `:: core :: primitive :: bool`.
        ///
        ///On success, returns `()`.
        pub fn begin_option(
            &mut self,
            __nc_arg_0_11: ::core::primitive::bool,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0_11 = ::neocogi_com::encode_inline(&__nc_arg_0_11)?;
            let __nc_this_13 = self.__nc_pointer.as_raw();
            let __nc_status_13 = unsafe {
                ((*__nc_this_13.vtable).begin_option)(__nc_this_13.object, __nc_arg_0_11)
            };
            if __nc_status_13.is_err() {
                return ::core::result::Result::Err(__nc_status_13);
            }
            let __nc_result_13 = ();
            ::core::result::Result::Ok(__nc_result_13)
        }
        /// Completes the optional value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:32:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::end_option`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `()`.
        pub fn end_option(
            &mut self,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_this_14 = self.__nc_pointer.as_raw();
            let __nc_status_14 = unsafe {
                ((*__nc_this_14.vtable).end_option)(__nc_this_14.object)
            };
            if __nc_status_14.is_err() {
                return ::core::result::Result::Err(__nc_status_14);
            }
            let __nc_result_14 = ();
            ::core::result::Result::Ok(__nc_result_14)
        }
        /// Opens a named record containing exactly fields members.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:34:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::begin_record`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `fields` is passed by value as `:: core :: primitive :: u32`.
        ///
        ///On success, returns `()`.
        pub fn begin_record(
            &mut self,
            __nc_arg_0_12: ::core::primitive::u32,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0_12 = ::neocogi_com::encode_inline(&__nc_arg_0_12)?;
            let __nc_this_15 = self.__nc_pointer.as_raw();
            let __nc_status_15 = unsafe {
                ((*__nc_this_15.vtable).begin_record)(__nc_this_15.object, __nc_arg_0_12)
            };
            if __nc_status_15.is_err() {
                return ::core::result::Result::Err(__nc_status_15);
            }
            let __nc_result_15 = ();
            ::core::result::Result::Ok(__nc_result_15)
        }
        /// Selects the next named field before writing its typed value.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:36:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::field`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `name` is passed as a safe readable `:: core :: primitive :: u8` buffer plus offset.
        ///
        ///On success, returns `()`.
        pub fn field(
            &mut self,
            __nc_arg_0_13: ::neocogi_com::NcCopyFromBufferOffset<
                '_,
                ::core::primitive::u8,
            >,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0_13 = ::neocogi_com::EncodedInput::new(__nc_arg_0_13)?;
            let __nc_this_16 = self.__nc_pointer.as_raw();
            let __nc_status_16 = unsafe {
                ((*__nc_this_16.vtable)
                    .field)(
                    __nc_this_16.object,
                    __nc_arg_0_13.offset(),
                    __nc_arg_0_13.len(),
                    __nc_arg_0_13.as_ptr(),
                )
            };
            if __nc_status_16.is_err() {
                return ::core::result::Result::Err(__nc_status_16);
            }
            let __nc_result_16 = ();
            ::core::result::Result::Ok(__nc_result_16)
        }
        /// Completes the record.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:38:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::end_record`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `()`.
        pub fn end_record(
            &mut self,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_this_17 = self.__nc_pointer.as_raw();
            let __nc_status_17 = unsafe {
                ((*__nc_this_17.vtable).end_record)(__nc_this_17.object)
            };
            if __nc_status_17.is_err() {
                return ::core::result::Result::Err(__nc_status_17);
            }
            let __nc_result_17 = ();
            ::core::result::Result::Ok(__nc_result_17)
        }
        /// Opens an ordered collection with its complete element count.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:40:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::begin_sequence`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `length` is passed by value as `:: core :: primitive :: u32`.
        ///
        ///On success, returns `()`.
        pub fn begin_sequence(
            &mut self,
            __nc_arg_0_14: ::core::primitive::u32,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0_14 = ::neocogi_com::encode_inline(&__nc_arg_0_14)?;
            let __nc_this_18 = self.__nc_pointer.as_raw();
            let __nc_status_18 = unsafe {
                ((*__nc_this_18.vtable)
                    .begin_sequence)(__nc_this_18.object, __nc_arg_0_14)
            };
            if __nc_status_18.is_err() {
                return ::core::result::Result::Err(__nc_status_18);
            }
            let __nc_result_18 = ();
            ::core::result::Result::Ok(__nc_result_18)
        }
        /// Completes the collection.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:42:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::end_sequence`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `()`.
        pub fn end_sequence(
            &mut self,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_this_19 = self.__nc_pointer.as_raw();
            let __nc_status_19 = unsafe {
                ((*__nc_this_19.vtable).end_sequence)(__nc_this_19.object)
            };
            if __nc_status_19.is_err() {
                return ::core::result::Result::Err(__nc_status_19);
            }
            let __nc_result_19 = ();
            ::core::result::Result::Ok(__nc_result_19)
        }
        /// Opens a map; keys and values alternate for exactly pairs entries.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:44:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::begin_map`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `pairs` is passed by value as `:: core :: primitive :: u32`.
        ///
        ///On success, returns `()`.
        pub fn begin_map(
            &mut self,
            __nc_arg_0_15: ::core::primitive::u32,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0_15 = ::neocogi_com::encode_inline(&__nc_arg_0_15)?;
            let __nc_this_20 = self.__nc_pointer.as_raw();
            let __nc_status_20 = unsafe {
                ((*__nc_this_20.vtable).begin_map)(__nc_this_20.object, __nc_arg_0_15)
            };
            if __nc_status_20.is_err() {
                return ::core::result::Result::Err(__nc_status_20);
            }
            let __nc_result_20 = ();
            ::core::result::Result::Ok(__nc_result_20)
        }
        /// Completes the map.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:46:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::end_map`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `()`.
        pub fn end_map(
            &mut self,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_this_21 = self.__nc_pointer.as_raw();
            let __nc_status_21 = unsafe {
                ((*__nc_this_21.vtable).end_map)(__nc_this_21.object)
            };
            if __nc_status_21.is_err() {
                return ::core::result::Result::Err(__nc_status_21);
            }
            let __nc_result_21 = ();
            ::core::result::Result::Ok(__nc_result_21)
        }
        /// Opens a case identified by both declaration ordinal and authored name.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:48:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::begin_variant`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `index` is passed by value as `:: core :: primitive :: u32`.
        ///- `name` is passed as a safe readable `:: core :: primitive :: u8` buffer plus offset.
        ///
        ///On success, returns `()`.
        pub fn begin_variant(
            &mut self,
            __nc_arg_0_16: ::core::primitive::u32,
            __nc_arg_1: ::neocogi_com::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0_16 = ::neocogi_com::encode_inline(&__nc_arg_0_16)?;
            let __nc_arg_1 = ::neocogi_com::EncodedInput::new(__nc_arg_1)?;
            let __nc_this_22 = self.__nc_pointer.as_raw();
            let __nc_status_22 = unsafe {
                ((*__nc_this_22.vtable)
                    .begin_variant)(
                    __nc_this_22.object,
                    __nc_arg_0_16,
                    __nc_arg_1.offset(),
                    __nc_arg_1.len(),
                    __nc_arg_1.as_ptr(),
                )
            };
            if __nc_status_22.is_err() {
                return ::core::result::Result::Err(__nc_status_22);
            }
            let __nc_result_22 = ();
            ::core::result::Result::Ok(__nc_result_22)
        }
        /// Completes the selected case and its optional payload.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:50:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::end_variant`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `()`.
        pub fn end_variant(
            &mut self,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_this_23 = self.__nc_pointer.as_raw();
            let __nc_status_23 = unsafe {
                ((*__nc_this_23.vtable).end_variant)(__nc_this_23.object)
            };
            if __nc_status_23.is_err() {
                return ::core::result::Result::Err(__nc_status_23);
            }
            let __nc_result_23 = ();
            ::core::result::Result::Ok(__nc_result_23)
        }
        /// Marks a schema failure; no later operation may succeed.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:52:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::fail`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///- `message` is passed as a safe readable `:: core :: primitive :: u8` buffer plus offset.
        ///
        ///On success, returns `()`.
        pub fn fail(
            &mut self,
            __nc_arg_0_17: ::neocogi_com::NcCopyFromBufferOffset<
                '_,
                ::core::primitive::u8,
            >,
        ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_arg_0_17 = ::neocogi_com::EncodedInput::new(__nc_arg_0_17)?;
            let __nc_this_24 = self.__nc_pointer.as_raw();
            let __nc_status_24 = unsafe {
                ((*__nc_this_24.vtable)
                    .fail)(
                    __nc_this_24.object,
                    __nc_arg_0_17.offset(),
                    __nc_arg_0_17.len(),
                    __nc_arg_0_17.as_ptr(),
                )
            };
            if __nc_status_24.is_err() {
                return ::core::result::Result::Err(__nc_status_24);
            }
            let __nc_result_24 = ();
            ::core::result::Result::Ok(__nc_result_24)
        }
        /// Commits a complete document; repeated finish and further writes fail.
        /// IDL source: `neocogi/com/serialization/formats.ncidl:54:8` (module `neocogi.com.serialization`).
        ///
        ///Calls [`ISerializer::finish`] through the generated COM ABI vtable.
        ///
        ///This safe wrapper invokes the raw vtable function and maps a successful status to `Ok(())`.
        ///Generated Rust thunks preserve negative `Result::Err` statuses and normalize non-negative error values to `NC_E_INVALID_ARG`.
        ///
        ///On success, returns `()`.
        pub fn finish(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
            let __nc_this_25 = self.__nc_pointer.as_raw();
            let __nc_status_25 = unsafe {
                ((*__nc_this_25.vtable).finish)(__nc_this_25.object)
            };
            if __nc_status_25.is_err() {
                return ::core::result::Result::Err(__nc_status_25);
            }
            let __nc_result_25 = ();
            ::core::result::Result::Ok(__nc_result_25)
        }
    }
};
impl ISerializerAbi {
    ///Canonical validation form of this interface's typed ABI contract.
    pub const ABI_SIGNATURE: ::neocogi_com::NcAbiSignature = ::neocogi_com::NcAbiSignature::from_static(
        b"\xCC\x0C\xBA\xF7\xB68\xE1\xE1\xF3\xFFBEI%\x85\0\xA8\xF9\xDB%\xFD\xF5\xC9\xFD'\x1AQ?\xF1\xBC\xBB\xD9",
    );
    ///Static descriptors for this interface's ABI methods.
    pub const METHOD_DESCRIPTORS: [::neocogi_com::NcMethodDescriptor; 26usize] = [
        ::neocogi_com::NcMethodDescriptor {
            name: ::neocogi_com::NcStaticUtf8::from_static("serialize_unit"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("serialize_bool"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("value"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::bool>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("serialize_u8"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("value"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::u8>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("serialize_u16"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("value"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::u16>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("serialize_u32"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("value"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("serialize_u64"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("value"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::u64>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("serialize_i8"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("value"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::i8>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("serialize_i16"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("value"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::i16>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("serialize_i32"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("value"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::i32>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("serialize_i64"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("value"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::i64>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("serialize_f32"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("value"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::f32>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("serialize_f64"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("value"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::f64>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("serialize_string"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("value"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::CopyFromBufferOffset,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_plain_type::<::core::primitive::u8>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("begin_option"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("present"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::bool>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("end_option"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("begin_record"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("fields"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("field"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("name"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::CopyFromBufferOffset,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_plain_type::<::core::primitive::u8>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("end_record"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("begin_sequence"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("length"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("end_sequence"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("begin_map"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("pairs"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("end_map"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("begin_variant"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 2u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("index"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::Value,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_type::<::core::primitive::u32>(),
                    ),
                },
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("name"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::CopyFromBufferOffset,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_plain_type::<::core::primitive::u8>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("end_variant"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("fail"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 1u32,
            params: (&[
                ::neocogi_com::NcMethodParamDescriptor {
                    name: ::neocogi_com::NcStaticUtf8::from_static("message"),
                    passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                        ::neocogi_com::NcAbiPassing::CopyFromBufferOffset,
                    ),
                    ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                        ::neocogi_com::__nc_abi_plain_type::<::core::primitive::u8>(),
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
            name: ::neocogi_com::NcStaticUtf8::from_static("finish"),
            receiver: ::neocogi_com::NC_ABI_RECEIVER_MUTABLE,
            param_count: 0u32,
            params: (&[] as &'static [::neocogi_com::NcMethodParamDescriptor]).as_ptr(),
            returns: ::neocogi_com::NcMethodReturnDescriptor {
                passing: ::neocogi_com::NcAbiPassingDescriptor::from_abi_passing(
                    ::neocogi_com::NcAbiPassing::Void,
                ),
                ty: ::neocogi_com::NcAbiTypeDescriptor::from_abi_type(
                    ::neocogi_com::NcAbiType::Void,
                ),
            },
        },
    ];
    ///Static ABI descriptor for this interface.
    pub const INTERFACE_DESCRIPTOR: ::neocogi_com::NcInterfaceDescriptor = ::neocogi_com::NcInterfaceDescriptor {
        interface_id: <Self as ::neocogi_com::ComInterface>::IID,
        qualified_name: ::neocogi_com::NcStaticUtf8::from_static(
            "neocogi.com.serialization.ISerializer",
        ),
        abi_signature: Self::ABI_SIGNATURE,
        base: ::neocogi_com::NcInterfaceId::NONE,
        method_count: 26u32,
        methods: Self::METHOD_DESCRIPTORS.as_ptr(),
    };
    /// Distinct projections in derived-first, declared-parent order.
    /// Shared ancestors occur once; repeated identities must have the same ABI.
    pub const INTERFACE_CHAIN: [&'static ::neocogi_com::NcInterfaceDescriptor; ::neocogi_com::__nc_object_interface_count(
        &[&[&Self::INTERFACE_DESCRIPTOR]],
    )] = ::neocogi_com::__nc_interface_chain(&Self::INTERFACE_DESCRIPTOR, &[]);
}
