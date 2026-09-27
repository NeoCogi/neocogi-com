//! Marker traits and layout helpers for values lowered across the C ABI.
//!
//! This module distinguishes ordinary by-value ABI types, durable plain values,
//! and macro-generated structs whose compiled layout remains available as typed
//! metadata. It also owns the opaque tag/payload storage used by generated COM
//! enums so safe wrappers can enforce discriminant and payload validity.

use crate::guid::NcGuid;
use crate::metadata::{NcAbiStructLayout, NcAbiType};
use crate::signature::NcAbiSignature;
use crate::status::NcStatus;

/// Nominal type metadata for copied application values with a stable C layout.
/// Generated methods transport [`crate::ComValue::Wire`], not `Self` directly;
/// this marker alone neither validates foreign contents nor supplies a codec.
///
/// # Safety
///
/// Implementors must have a stable ABI layout, must not contain Rust references,
/// raw pointers, or other lifetime-bound data, must not rely on Rust drop glue
/// across the ABI boundary. Inline arrays are permitted as C-layout fields;
/// calls transport their repr(C) checked wire wrapper, never a native Rust array
/// directly in an extern function signature.
pub unsafe trait AbiType: Copy + 'static {
    /// Canonical ABI type identity stored in method and field descriptors.
    const ABI_TYPE: NcAbiType;
}

/// Marker for copyable ABI values with no hidden pointer lifetime or ownership.
///
/// # Safety
///
/// Implementors must satisfy [`AbiType`], must be safe to copy as durable data,
/// and must not encode borrowed addresses, borrowed references, or ownership
/// obligations that require a separate release/drop protocol.
///
/// This marker does not assert that every bit pattern is a valid `Self`, that
/// arbitrary storage is initialized, or that a tagged union's discriminant and
/// active member agree. APIs that materialize a Rust value must establish those
/// validity requirements separately before reading the value.
pub unsafe trait AbiPlainValue: AbiType {}

/// Marker for macro-generated application structs with physical C-layout metadata.
/// This describes `Self`, not the separate [`crate::ComValue::Wire`] transported
/// by generated methods. Raw field layouts may differ, especially for nested enums.
///
/// # Safety
///
/// Implementors must describe their actual ABI declaration and layout exactly.
/// [`Self::ABI_SIGNATURE`] must be the declaration-scoped fingerprint assigned
/// by the IDL generator. `size`, `align`, and every field offset in
/// [`Self::ABI_STRUCT_LAYOUT`] must match the compiled representation of `Self`,
/// and every field type must be the field's canonical ABI type. Prefer using
/// generated `#[com_struct]` bindings, which combine the tool-owned identity
/// with the compiler's physical layout facts.
///
/// With `#[com_struct(c_struct)]`, external fields remain opaque aggregates in
/// typed metadata. Their Rust compiler layout is checked locally, but the IDL
/// declaration signature—not compiler-side hashing—is the compatibility
/// authority. The contract author must ensure those external fields retain the
/// declared C layout and plain-value semantics.
pub unsafe trait ComStruct: Copy + 'static {
    /// Generator-assigned ABI identity for this structure declaration.
    const ABI_SIGNATURE: NcAbiSignature;

    /// Physical application layout; do not use it to address recursive wire fields.
    const ABI_STRUCT_LAYOUT: NcAbiStructLayout;
}

/// Opaque storage used by macro-generated tagged ABI values.
///
/// The fields intentionally remain private to `neocogi-com-core`. Generated wrappers
/// expose validity-preserving safe constructors and use the unsafe constructor
/// only after establishing the tag/payload relationship.
#[doc(hidden)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct __NcTaggedEnumAbiStorage<Tag, Payload> {
    tag: Tag,
    payload: Payload,
}

impl<Tag, Payload> __NcTaggedEnumAbiStorage<Tag, Payload> {
    /// Byte offset of the tag in this C-compatible storage pair.
    pub const TAG_OFFSET: usize = core::mem::offset_of!(Self, tag);

    /// Byte offset of the payload in this C-compatible storage pair.
    pub const PAYLOAD_OFFSET: usize = core::mem::offset_of!(Self, payload);

    /// Creates storage from a raw tag and payload.
    ///
    /// # Safety
    ///
    /// The caller must guarantee the logical relationship required by the
    /// generated wrapper. If the tag selects a typed union member, that member
    /// must be active, every byte subsequently read must be initialized, and
    /// every selected field must contain a valid Rust value.
    pub const unsafe fn from_raw_parts_unchecked(tag: Tag, payload: Payload) -> Self {
        Self { tag, payload }
    }

    /// Returns the tag without inspecting the payload.
    pub const fn tag(&self) -> Tag
    where
        Tag: Copy,
    {
        self.tag
    }

    /// Borrows the payload without interpreting it.
    pub const fn payload(&self) -> &Payload {
        &self.payload
    }

    /// Consumes the storage and returns both raw parts.
    pub fn into_raw_parts(self) -> (Tag, Payload) {
        (self.tag, self.payload)
    }
}

#[doc(hidden)]
/// Returns the canonical ABI type for a generated by-value parameter.
pub const fn __nc_abi_type<T: AbiType>() -> NcAbiType {
    T::ABI_TYPE
}

#[doc(hidden)]
/// Returns the canonical ABI type while requiring durable plain-value semantics.
pub const fn __nc_abi_plain_type<T: AbiPlainValue>() -> NcAbiType {
    T::ABI_TYPE
}

#[doc(hidden)]
/// Returns the aggregate ABI identity generated for a COM struct.
pub const fn __nc_abi_com_struct_type<T: ComStruct>() -> NcAbiType {
    NcAbiType::Aggregate(T::ABI_SIGNATURE)
}

macro_rules! impl_abi_plain_value {
    ($($ty:ty => $abi_type:expr),* $(,)?) => {
        $(
            // SAFETY: These primitive and core ABI value types are `Copy`, have
            // stable C-shaped representations in this library's ABI contract,
            // and carry no Rust-owned resources or borrowed lifetimes.
            unsafe impl AbiType for $ty {
                const ABI_TYPE: NcAbiType = $abi_type;
            }
            // SAFETY: The same types are durable plain values with no release
            // protocol or hidden ownership.
            unsafe impl AbiPlainValue for $ty {}
        )*
    };
}

impl_abi_plain_value!(
    bool => NcAbiType::Bool,
    u8 => NcAbiType::U8,
    u16 => NcAbiType::U16,
    u32 => NcAbiType::U32,
    u64 => NcAbiType::U64,
    usize => NcAbiType::USize,
    i8 => NcAbiType::I8,
    i16 => NcAbiType::I16,
    i32 => NcAbiType::I32,
    i64 => NcAbiType::I64,
    isize => NcAbiType::ISize,
    f32 => NcAbiType::F32,
    f64 => NcAbiType::F64,
    NcGuid => NcAbiType::NcGuid,
    NcStatus => NcAbiType::NcStatus,
);

/// Derives an intrinsic array identity from its element contract and fixed length.
/// The domain, little-endian tag/count and zero-filled digest slot are normative.
/// This constant-size calculation never hashes a declaration graph or its fields.
const fn array_signature(element: NcAbiType, len: usize) -> [u8; 32] {
    use crate::metadata::NcAbiTypeDescriptor;
    let descriptor = NcAbiTypeDescriptor::from_abi_type(element);
    let mut input = [0u8; 60];
    let domain = b"neocogi.array:v1";
    let mut index = 0;
    while index < domain.len() {
        input[index] = domain[index];
        index += 1;
    }
    let tag = descriptor.tag.to_le_bytes();
    index = 0;
    while index < tag.len() {
        input[16 + index] = tag[index];
        index += 1;
    }
    let count = (len as u64).to_le_bytes();
    index = 0;
    while index < count.len() {
        input[20 + index] = count[index];
        index += 1;
    }
    match element {
        NcAbiType::Aggregate(signature) => {
            let digest = signature.const_bytes();
            index = 0;
            while index < digest.len() {
                input[28 + index] = digest[index];
                index += 1;
            }
        }
        NcAbiType::Interface { .. } | NcAbiType::Void => panic!("an array element must be a plain value"),
        _ => {}
    }
    crate::__nc_abi_digest(&input)
}

// SAFETY: A fixed array of plain values has contiguous stable field layout,
// contains no ownership or call-scoped addresses, and is Copy. Its actual calls
// use ValueArray<T::Wire, N>, whose decoder checks each element before construction.
unsafe impl<T: AbiPlainValue, const N: usize> AbiType for [T; N] {
    const ABI_TYPE: NcAbiType = NcAbiType::Aggregate(NcAbiSignature::from_static(&array_signature(T::ABI_TYPE, N)));
}
// SAFETY: Arrays introduce no resources or lifetime obligations beyond T.
unsafe impl<T: AbiPlainValue, const N: usize> AbiPlainValue for [T; N] {}

#[cfg(test)]
mod array_tests {
    use super::*;
    /// Pins the intrinsic's domain and byte encoding independently of Rust layout.
    #[test]
    fn array_signature_matches_known_sha256_and_distinguishes_shapes() {
        let bools = <[bool; 3] as AbiType>::ABI_TYPE;
        const EXPECTED: [u8; 32] = [0x34, 0xa4, 0x31, 0xc9, 0xb2, 0xe9, 0xf3, 0x54, 0xf8, 0xce, 0x68, 0x9b, 0xbc, 0xe6, 0xc2, 0x2b, 0xd4, 0x85, 0x69, 0xb4, 0x95, 0xa3, 0x9d, 0x52, 0xd0, 0xa3, 0x79, 0x94, 0x79, 0x77, 0x5b, 0x80];
        assert_eq!(bools, NcAbiType::Aggregate(NcAbiSignature::from_static(&EXPECTED)));
        assert_ne!(bools, <[u8; 3] as AbiType>::ABI_TYPE);
        assert_ne!(bools, <[bool; 4] as AbiType>::ABI_TYPE);
        assert_ne!(<[bool; 4] as AbiType>::ABI_TYPE, <[[bool; 2]; 2] as AbiType>::ABI_TYPE);
        assert_ne!(<[u8; 0] as AbiType>::ABI_TYPE, <[u16; 0] as AbiType>::ABI_TYPE);
    }
}
