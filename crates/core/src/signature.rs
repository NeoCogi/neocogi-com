//! Canonical ABI signatures shared by macros and runtime validation.
//!
//! Compatibility is represented by a versioned sequence of typed
//! [`NcAbiSignaturePart`] records stored as one SHA-256 digest. Type aliases and imports
//! resolve through [`crate::AbiType::ABI_TYPE`], while `size_of`, `align_of`,
//! and `offset_of` values are evaluated by rustc.
//!
//! Signatures use a C-compatible pointer/count view and are compared
//! byte-for-byte. Validation adds no work to update, layout, paint, or other
//! object calls.

use core::hash::{Hash, Hasher};

use crate::guid::NcGuid;
use crate::metadata::NcAbiSignaturePart;
use crate::metadata::{NcAbiAggregateKind, NcAbiPassing, NcAbiThreadingMode, NcAbiType};
use crate::status::{NC_E_POINTER, NcStatus};

mod sha256;

/// Versioned prefix for the canonical byte stream hashed by exact validation.
const NC_ABI_SIGNATURE_DOMAIN: &[u8] = b"ncabi1;";

/// Declares one canonical ABI signature backed by exact-size static storage.
///
/// Procedural macros generate the same pattern directly. This expression macro
/// is provided for the deliberately unsafe handwritten-interface extension
/// point and for tests that need a small explicit contract.
///
/// # Examples
///
/// ```
/// use neocogi_com_core::{NcAbiSignature, NcAbiSignaturePart, NcGuid, abi_signature};
///
/// const IID: NcGuid = NcGuid::from_fields(1, 2, 3, [4, 5, 6, 7, 8, 9, 10, 11]);
/// const SIGNATURE: NcAbiSignature = abi_signature![
///     NcAbiSignaturePart::Interface { iid: IID },
/// ];
/// assert!(!SIGNATURE.is_empty());
/// ```
#[macro_export]
macro_rules! abi_signature {
    (&[$($part:expr),+ $(,)?]) => {
        $crate::abi_signature![$($part),+]
    };
    ($($part:expr),+ $(,)?) => {{
        const __NC_ABI_SIGNATURE_PARTS: &[$crate::NcAbiSignaturePart<'static>] = &[$($part),+];
        static __NC_ABI_SIGNATURE_BYTES: [u8; $crate::NcAbiSignature::__encoded_len(__NC_ABI_SIGNATURE_PARTS)] =
            $crate::NcAbiSignature::__encode::<{
                $crate::NcAbiSignature::__encoded_len(__NC_ABI_SIGNATURE_PARTS)
            }>(__NC_ABI_SIGNATURE_PARTS);
        $crate::NcAbiSignature::from_static(&__NC_ABI_SIGNATURE_BYTES)
    }};
}

/// Maximum number of primitive output tokens required by one logical part.
const MAX_TOKENS_PER_PART: usize = 8;

/// Static canonical ABI signature stored in C-compatible pointer/count form.
///
/// The pointed bytes remain owned by the component that declares the ABI type.
/// Components are pinned while their metadata or objects are visible, so the
/// view stays valid for every comparison that can reach it.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NcAbiSignature {
    /// Pointer to immutable canonical validation bytes, or null for an empty view.
    data: *const u8,
    /// Number of readable bytes at [`Self::data`].
    len: u32,
}

// SAFETY: A valid signature references immutable static bytes. Moving or
// sharing the two-word view cannot mutate or invalidate the backing storage.
unsafe impl Send for NcAbiSignature {}
// SAFETY: The same immutable static storage may be read concurrently.
unsafe impl Sync for NcAbiSignature {}

impl NcAbiSignature {
    /// Empty signature sentinel used when no ABI contract is present.
    pub const EMPTY: Self = Self { data: core::ptr::null(), len: 0 };

    /// Creates a signature view over immutable static validation bytes.
    ///
    /// Generated code uses this constructor with the exact-size buffer emitted
    /// for an attributed ABI declaration.
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        assert!(
            bytes.is_empty() || bytes.len() == sha256::SIGNATURE_LEN,
            "ABI signature must be empty or one SHA-256 digest"
        );
        Self {
            data: if bytes.is_empty() { core::ptr::null() } else { bytes.as_ptr() },
            len: bytes.len() as u32,
        }
    }

    /// Returns the signature length in bytes.
    pub const fn len(self) -> u32 {
        self.len
    }

    /// Returns whether this is the no-contract sentinel.
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Borrows the SHA-256 validation bytes.
    ///
    /// # Safety
    ///
    /// For a non-empty signature, `data` must point to `len` immutable bytes
    /// that remain valid for the returned lifetime. Generated signatures and
    /// validated descriptors from pinned components satisfy this contract.
    pub unsafe fn as_bytes<'a>(self) -> Result<&'a [u8], NcStatus> {
        if self.len == 0 {
            return Ok(&[]);
        }
        if self.data.is_null() {
            return Err(NC_E_POINTER);
        }
        // SAFETY: The caller establishes the pointer, extent, and lifetime.
        Ok(unsafe { core::slice::from_raw_parts(self.data, self.len as usize) })
    }

    /// Returns the exact encoded length for an ordered logical signature.
    ///
    /// This operation is public only for code emitted by `neocogi-com-macros`.
    #[doc(hidden)]
    pub const fn __encoded_len(parts: &[NcAbiSignaturePart<'_>]) -> usize {
        let _ = parts;
        sha256::SIGNATURE_LEN
    }

    /// Encodes ordered logical parts into an exact-size static byte array.
    ///
    /// `N` must equal [`Self::__encoded_len`] for `parts`; generated code uses
    /// the same const expression for both the array length and this argument.
    #[doc(hidden)]
    pub const fn __encode<const N: usize>(parts: &[NcAbiSignaturePart<'_>]) -> [u8; N] {
        sha256::encode(parts)
    }

    /// Compares two generated signatures during constant evaluation.
    ///
    /// This operation is public only for macro-generated ancestry tables.
    #[doc(hidden)]
    pub const fn __const_eq(self, other: Self) -> bool {
        if self.len != other.len {
            return false;
        }
        if self.len == 0 {
            return true;
        }

        // SAFETY: Safe constructors accept only static storage. Foreign values
        // reach const evaluation only through unsafe producer implementations.
        let left = unsafe { core::slice::from_raw_parts(self.data, self.len as usize) };
        let right = unsafe { core::slice::from_raw_parts(other.data, other.len as usize) };
        let mut index = 0;
        while index < left.len() {
            if left[index] != right[index] {
                return false;
            }
            index += 1;
        }
        true
    }

    /// Encodes arbitrary test records into leaked process-lifetime storage.
    ///
    /// Production signatures are always exact-size compile-time statics. This
    /// helper exists only for core unit tests that compare many runtime-built
    /// variations without obscuring those tests with static declarations.
    #[cfg(test)]
    pub(crate) fn from_abi_parts(parts: &[NcAbiSignaturePart<'_>]) -> Self {
        let bytes = encode_owned_for_tests(parts).into_boxed_slice();
        Self::from_static(std::boxed::Box::leak(bytes))
    }

    /// Encodes aggregate layout records for runtime-built unit-test cases.
    #[cfg(test)]
    pub(crate) fn from_aggregate_layout(kind: NcAbiAggregateKind, size: usize, align: usize, fields: &[crate::NcAbiFieldLayout]) -> Self {
        let mut parts = std::vec![NcAbiSignaturePart::Aggregate { kind }, NcAbiSignaturePart::Layout { size, align }];
        parts.extend(fields.iter().map(|field| NcAbiSignaturePart::Field {
            index: field.index,
            offset: field.offset,
            ty: field.ty,
        }));
        Self::from_abi_parts(&parts)
    }
}

impl PartialEq for NcAbiSignature {
    /// Compares canonical content rather than component-local pointer values.
    fn eq(&self, other: &Self) -> bool {
        Self::__const_eq(*self, *other)
    }
}

impl Eq for NcAbiSignature {}

impl Hash for NcAbiSignature {
    /// Hashes canonical content so hash maps agree with byte-wise equality.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.len.hash(state);
        if self.len != 0 {
            // SAFETY: Every safe value points to static immutable storage. Raw
            // FFI producers must uphold the same invariant before registration.
            unsafe { core::slice::from_raw_parts(self.data, self.len as usize) }.hash(state);
        }
    }
}

/// Primitive output operation shared by length calculation and encoding.
#[derive(Clone, Copy)]
enum SignatureToken<'a> {
    /// Unused fixed-array slot.
    Empty,
    /// One literal ASCII byte.
    Byte(u8),
    /// An arbitrary UTF-8 fragment whose length is encoded by its caller.
    Bytes(&'a [u8]),
    /// A complete nested canonical signature.
    Signature(NcAbiSignature),
    /// One GUID rendered as 32 lowercase hexadecimal digits.
    Guid(NcGuid),
    /// An integer rendered as a fixed number of lowercase hexadecimal digits.
    Hex {
        /// Unsigned bit pattern to render.
        value: u128,
        /// Exact output width in hexadecimal digits.
        digits: u8,
    },
}

/// Fixed-capacity token sequence for one logical signature record.
#[derive(Clone, Copy)]
struct SignatureTokens<'a> {
    /// Primitive tokens in canonical output order.
    values: [SignatureToken<'a>; MAX_TOKENS_PER_PART],
    /// Number of initialized entries in [`Self::values`].
    len: usize,
}

impl<'a> SignatureTokens<'a> {
    /// Creates an empty token sequence.
    const fn new() -> Self {
        Self {
            values: [SignatureToken::Empty; MAX_TOKENS_PER_PART],
            len: 0,
        }
    }

    /// Appends one token, rejecting an internal grammar-capacity mistake.
    const fn push(&mut self, token: SignatureToken<'a>) {
        assert!(self.len < self.values.len(), "ABI signature part requires too many tokens");
        self.values[self.len] = token;
        self.len += 1;
    }
}

/// Converts one logical ABI record into canonical primitive output tokens.
const fn tokens_for_part(part: NcAbiSignaturePart<'_>) -> SignatureTokens<'_> {
    let mut tokens = SignatureTokens::new();
    match part {
        NcAbiSignaturePart::Interface { iid } => {
            tokens.push(SignatureToken::Byte(b'I'));
            tokens.push(SignatureToken::Guid(iid));
        }
        NcAbiSignaturePart::BaseInterface { interface } => {
            tokens.push(SignatureToken::Byte(b'B'));
            tokens.push(SignatureToken::Guid(interface.iid));
            tokens.push(SignatureToken::Hex {
                value: interface.abi_signature.len as u128,
                digits: 8,
            });
            tokens.push(SignatureToken::Signature(interface.abi_signature));
        }
        NcAbiSignaturePart::Method { index, name } => {
            tokens.push(SignatureToken::Byte(b'M'));
            tokens.push(SignatureToken::Hex { value: index as u128, digits: 8 });
            tokens.push(SignatureToken::Hex { value: name.len() as u128, digits: 8 });
            tokens.push(SignatureToken::Bytes(name.as_bytes()));
        }
        NcAbiSignaturePart::Receiver { receiver } => {
            tokens.push(SignatureToken::Byte(b'R'));
            append_receiver(&mut tokens, receiver);
        }
        NcAbiSignaturePart::Arg { index, passing, ty } => {
            tokens.push(SignatureToken::Byte(b'A'));
            tokens.push(SignatureToken::Hex { value: index as u128, digits: 8 });
            append_passing(&mut tokens, passing);
            append_type(&mut tokens, ty);
        }
        NcAbiSignaturePart::Return { passing, ty } => {
            tokens.push(SignatureToken::Byte(b'O'));
            append_passing(&mut tokens, passing);
            append_type(&mut tokens, ty);
        }
        NcAbiSignaturePart::Status => tokens.push(SignatureToken::Byte(b'S')),
        NcAbiSignaturePart::Layout { size, align } => {
            tokens.push(SignatureToken::Byte(b'L'));
            tokens.push(SignatureToken::Hex { value: size as u128, digits: 16 });
            tokens.push(SignatureToken::Hex { value: align as u128, digits: 16 });
        }
        NcAbiSignaturePart::Field { index, offset, ty } => {
            tokens.push(SignatureToken::Byte(b'F'));
            tokens.push(SignatureToken::Hex { value: index as u128, digits: 8 });
            tokens.push(SignatureToken::Hex { value: offset as u128, digits: 16 });
            append_type(&mut tokens, ty);
        }
        NcAbiSignaturePart::Variant { index, tag, ty } => {
            tokens.push(SignatureToken::Byte(b'V'));
            tokens.push(SignatureToken::Hex { value: index as u128, digits: 8 });
            tokens.push(SignatureToken::Hex { value: tag as u128, digits: 32 });
            append_type(&mut tokens, ty);
        }
        NcAbiSignaturePart::Aggregate { kind } => {
            tokens.push(SignatureToken::Byte(b'G'));
            append_aggregate_kind(&mut tokens, kind);
        }
    }
    tokens
}

/// Appends the canonical token for one ABI type.
const fn append_type<'a>(tokens: &mut SignatureTokens<'a>, ty: NcAbiType) {
    let tag = match ty {
        NcAbiType::Void => 0x00,
        NcAbiType::Bool => 0x01,
        NcAbiType::U8 => 0x02,
        NcAbiType::U16 => 0x03,
        NcAbiType::U32 => 0x04,
        NcAbiType::U64 => 0x05,
        NcAbiType::USize => 0x06,
        NcAbiType::I8 => 0x07,
        NcAbiType::I16 => 0x08,
        NcAbiType::I32 => 0x09,
        NcAbiType::I64 => 0x0a,
        NcAbiType::ISize => 0x0b,
        NcAbiType::F32 => 0x0c,
        NcAbiType::F64 => 0x0d,
        NcAbiType::NcGuid => 0x0e,
        NcAbiType::NcStatus => 0x0f,
        NcAbiType::Aggregate(_) => 0x11,
        NcAbiType::Interface { .. } => 0x12,
    };
    tokens.push(SignatureToken::Hex { value: tag, digits: 2 });
    match ty {
        NcAbiType::Aggregate(signature) => {
            tokens.push(SignatureToken::Hex { value: signature.len as u128, digits: 8 });
            tokens.push(SignatureToken::Signature(signature));
        }
        NcAbiType::Interface { iid } => tokens.push(SignatureToken::Guid(iid)),
        _ => {}
    }
}

/// Appends the canonical tokens for one passing mode.
const fn append_passing<'a>(tokens: &mut SignatureTokens<'a>, passing: NcAbiPassing) {
    let (tag, threading) = match passing {
        NcAbiPassing::Void => (0x00, None),
        NcAbiPassing::Value => (0x01, None),
        NcAbiPassing::Out => (0x02, None),
        NcAbiPassing::CopyFromBufferOffset => (0x03, None),
        NcAbiPassing::CopyToBufferOffset => (0x04, None),
        NcAbiPassing::ComRef { mode } => (0x05, Some(mode)),
        NcAbiPassing::ComPtrOut { mode } => (0x06, Some(mode)),
    };
    tokens.push(SignatureToken::Hex { value: tag, digits: 2 });
    if let Some(mode) = threading {
        append_threading_mode(tokens, mode);
    }
}

/// Appends the canonical token for one threading capability.
const fn append_threading_mode<'a>(tokens: &mut SignatureTokens<'a>, mode: NcAbiThreadingMode) {
    let tag = match mode {
        NcAbiThreadingMode::ThreadLocal => 0x00,
        NcAbiThreadingMode::ThreadSafe => 0x01,
    };
    tokens.push(SignatureToken::Hex { value: tag, digits: 2 });
}

/// Appends the canonical token for one method receiver mode.
const fn append_receiver<'a>(tokens: &mut SignatureTokens<'a>, receiver: crate::NcAbiReceiver) {
    let tag = match receiver {
        crate::NcAbiReceiver::Shared => 0x00,
        crate::NcAbiReceiver::Mutable => 0x01,
    };
    tokens.push(SignatureToken::Hex { value: tag, digits: 2 });
}

/// Appends the canonical token for one aggregate category.
const fn append_aggregate_kind<'a>(tokens: &mut SignatureTokens<'a>, kind: NcAbiAggregateKind) {
    let tag = match kind {
        NcAbiAggregateKind::Struct => 0x00,
        NcAbiAggregateKind::Union => 0x01,
        NcAbiAggregateKind::TaggedEnum => 0x02,
    };
    tokens.push(SignatureToken::Hex { value: tag, digits: 2 });
}

/// Returns the number of output bytes produced by one primitive token.
const fn token_len(token: SignatureToken<'_>) -> usize {
    match token {
        SignatureToken::Empty => 0,
        SignatureToken::Byte(_) => 1,
        SignatureToken::Bytes(bytes) => bytes.len(),
        SignatureToken::Signature(signature) => signature.len as usize,
        SignatureToken::Guid(_) => 32,
        SignatureToken::Hex { digits, .. } => digits as usize,
    }
}

/// Allocation-free cursor over the canonical ABI byte stream.
///
/// SHA-256 consumes this cursor directly without materializing an intermediate
/// buffer or duplicating type serialization rules.
struct SignatureByteCursor<'slice, 'value> {
    /// Logical records that remain to be serialized.
    parts: &'slice [NcAbiSignaturePart<'value>],
    /// Index of the next logical record to tokenize.
    part_index: usize,
    /// Primitive tokens for the current logical record.
    tokens: SignatureTokens<'value>,
    /// Index of the current primitive token.
    token_index: usize,
    /// Byte offset within the current primitive token.
    token_offset: usize,
    /// Byte offset within the shared signature-domain prefix.
    domain_offset: usize,
}

impl<'slice, 'value> SignatureByteCursor<'slice, 'value> {
    /// Creates a cursor for a complete signature or one incremental part.
    const fn new(parts: &'slice [NcAbiSignaturePart<'value>], include_domain: bool) -> Self {
        Self {
            parts,
            part_index: 0,
            tokens: SignatureTokens::new(),
            token_index: 0,
            token_offset: 0,
            domain_offset: if include_domain { 0 } else { NC_ABI_SIGNATURE_DOMAIN.len() },
        }
    }

    /// Returns the next canonical byte, or `None` after the complete stream.
    const fn next(&mut self) -> Option<u8> {
        if self.domain_offset < NC_ABI_SIGNATURE_DOMAIN.len() {
            let byte = NC_ABI_SIGNATURE_DOMAIN[self.domain_offset];
            self.domain_offset += 1;
            return Some(byte);
        }

        loop {
            if self.token_index < self.tokens.len {
                let token = self.tokens.values[self.token_index];
                if self.token_offset < token_len(token) {
                    let byte = token_byte(token, self.token_offset);
                    self.token_offset += 1;
                    return Some(byte);
                }
                self.token_index += 1;
                self.token_offset = 0;
                continue;
            }
            if self.part_index == self.parts.len() {
                return None;
            }
            self.tokens = tokens_for_part(self.parts[self.part_index]);
            self.part_index += 1;
            self.token_index = 0;
            self.token_offset = 0;
        }
    }
}

/// Returns one byte from a primitive signature token.
const fn token_byte(token: SignatureToken<'_>, index: usize) -> u8 {
    match token {
        SignatureToken::Empty => panic!("empty signature token has no bytes"),
        SignatureToken::Byte(byte) => {
            assert!(index == 0, "single-byte signature token index is out of bounds");
            byte
        }
        SignatureToken::Bytes(bytes) => bytes[index],
        SignatureToken::Signature(signature) => {
            // SAFETY: Nested signature constants point to immutable static
            // buffers whose lengths were checked by `from_static`.
            (unsafe { core::slice::from_raw_parts(signature.data, signature.len as usize) })[index]
        }
        SignatureToken::Guid(guid) => {
            let bytes = guid.to_uuid_bytes();
            let byte = bytes[index / 2];
            let shift = if index & 1 == 0 { 4 } else { 0 };
            hex_digit((byte >> shift) & 0x0f)
        }
        SignatureToken::Hex { value, digits } => {
            let shift = (digits as usize - index - 1) as u32 * 4;
            hex_digit(((value >> shift) & 0x0f) as u8)
        }
    }
}

/// Converts one hexadecimal nibble to canonical lowercase ASCII.
const fn hex_digit(value: u8) -> u8 {
    if value < 10 { b'0' + value } else { b'a' + (value - 10) }
}

/// Runtime encoder used only by tests that generate many signature variations.
#[cfg(test)]
fn encode_owned_for_tests(parts: &[NcAbiSignaturePart<'_>]) -> std::vec::Vec<u8> {
    sha256::encode::<{ sha256::SIGNATURE_LEN }>(parts).to_vec()
}

#[cfg(test)]
mod tests {
    use super::NcAbiSignature;

    /// Confirms safe handwritten metadata cannot introduce another signature width.
    #[test]
    #[should_panic(expected = "ABI signature must be empty or one SHA-256 digest")]
    fn static_signatures_reject_non_sha256_widths() {
        // Sixteen bytes model the removed compact-hash representation.
        static INVALID_SIGNATURE: [u8; 16] = [0; 16];

        let _ = NcAbiSignature::from_static(&INVALID_SIGNATURE);
    }
}
