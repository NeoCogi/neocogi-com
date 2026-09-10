//! Canonical mangled ABI signatures shared by macros and runtime validation.
//!
//! Compatibility is represented by a versioned sequence of typed
//! [`NcAbiSignaturePart`] records. Procedural macros generate an exactly sized
//! static UTF-8 buffer for each aggregate, constructor, and interface. Type
//! aliases and imports resolve through [`crate::AbiType::ABI_TYPE`], while
//! `size_of`, `align_of`, and `offset_of` values are evaluated by rustc.
//!
//! Signatures are compared byte-for-byte. The format is a compact mangle rather
//! than a source-level Rust type spelling, so equivalent Rust paths cannot
//! create false ABI differences and no probabilistic hash collision is possible.

use core::hash::{Hash, Hasher};

use crate::guid::NcGuid;
use crate::metadata::{NcAbiAggregateKind, NcAbiPassing, NcAbiSignaturePart, NcAbiThreadingMode, NcAbiType};
use crate::status::{NC_E_POINTER, NcStatus};

/// Versioned prefix for every complete mangled ABI signature.
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
    /// Pointer to immutable canonical UTF-8 bytes, or null for an empty view.
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

    /// Creates a signature view over immutable static mangled bytes.
    ///
    /// Generated code uses this constructor with the exact-size buffer emitted
    /// for an attributed ABI declaration.
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        assert!(bytes.len() <= u32::MAX as usize, "ABI signature exceeds the u32 byte-length limit");
        Self {
            data: if bytes.is_empty() { core::ptr::null() } else { bytes.as_ptr() },
            len: bytes.len() as u32,
        }
    }

    /// Creates a signature view from a static mangled string.
    pub const fn from_static_str(value: &'static str) -> Self {
        Self::from_static(value.as_bytes())
    }

    /// Returns the signature length in bytes.
    pub const fn len(self) -> u32 {
        self.len
    }

    /// Returns whether this is the no-contract sentinel.
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Checks the null/length invariant without dereferencing the view.
    pub(crate) const fn has_valid_shape(self) -> bool {
        self.data.is_null() == (self.len == 0)
    }

    /// Borrows the mangled signature bytes.
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
        let mut len = NC_ABI_SIGNATURE_DOMAIN.len();
        let mut part_index = 0;
        while part_index < parts.len() {
            let tokens = tokens_for_part(parts[part_index]);
            let mut token_index = 0;
            while token_index < tokens.len {
                len = len.checked_add(token_len(tokens.values[token_index])).expect("ABI signature length overflow");
                token_index += 1;
            }
            part_index += 1;
        }
        assert!(len <= u32::MAX as usize, "ABI signature exceeds the u32 byte-length limit");
        len
    }

    /// Encodes ordered logical parts into an exact-size static byte array.
    ///
    /// `N` must equal [`Self::__encoded_len`] for `parts`; generated code uses
    /// the same const expression for both the array length and this argument.
    #[doc(hidden)]
    pub const fn __encode<const N: usize>(parts: &[NcAbiSignaturePart<'_>]) -> [u8; N] {
        assert!(N == Self::__encoded_len(parts), "incorrect ABI signature buffer length");
        let mut writer = SignatureWriter::<N>::new();
        writer.write_bytes(NC_ABI_SIGNATURE_DOMAIN);

        let mut part_index = 0;
        while part_index < parts.len() {
            let tokens = tokens_for_part(parts[part_index]);
            let mut token_index = 0;
            while token_index < tokens.len {
                writer.write_token(tokens.values[token_index]);
                token_index += 1;
            }
            part_index += 1;
        }
        writer.finish()
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
        let bytes = encode_owned(parts).into_boxed_slice();
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

/// Streaming verifier used to check descriptor metadata without allocation.
pub(crate) struct NcAbiSignatureVerifier<'a> {
    /// Advertised bytes borrowed from a descriptor owned by a pinned component.
    actual: &'a [u8],
    /// Number of advertised bytes consumed by successful comparisons.
    offset: usize,
    /// Whether every expected byte encountered so far matched.
    matches: bool,
}

impl<'a> NcAbiSignatureVerifier<'a> {
    /// Starts verification and consumes the mandatory mangle version prefix.
    ///
    /// # Safety
    ///
    /// `signature` must satisfy [`NcAbiSignature::as_bytes`]'s pointer and
    /// lifetime contract for `'a`.
    pub(crate) unsafe fn new(signature: NcAbiSignature) -> Result<Self, NcStatus> {
        // SAFETY: The caller establishes the backing storage contract.
        let actual = unsafe { signature.as_bytes()? };
        let mut verifier = Self { actual, offset: 0, matches: true };
        verifier.verify_bytes(NC_ABI_SIGNATURE_DOMAIN);
        Ok(verifier)
    }

    /// Compares one logical record using the canonical tokenizer.
    pub(crate) fn verify_part(&mut self, part: NcAbiSignaturePart<'_>) {
        let tokens = tokens_for_part(part);
        let mut index = 0;
        while index < tokens.len {
            self.verify_token(tokens.values[index]);
            index += 1;
        }
    }

    /// Returns whether the full advertised signature matched exactly.
    pub(crate) fn finish(self) -> bool {
        self.matches && self.offset == self.actual.len()
    }

    /// Compares one primitive output token.
    fn verify_token(&mut self, token: SignatureToken<'_>) {
        match token {
            SignatureToken::Empty => {}
            SignatureToken::Byte(byte) => self.verify_byte(byte),
            SignatureToken::Bytes(bytes) => self.verify_bytes(bytes),
            SignatureToken::Signature(signature) => {
                if signature.len != 0 {
                    // SAFETY: Nested signatures originate from ABI type
                    // constants whose backing components remain pinned.
                    self.verify_bytes(unsafe { core::slice::from_raw_parts(signature.data, signature.len as usize) });
                }
            }
            SignatureToken::Guid(guid) => {
                let bytes = guid.to_uuid_bytes();
                let mut index = 0;
                while index < bytes.len() {
                    self.verify_hex(bytes[index] as u128, 2);
                    index += 1;
                }
            }
            SignatureToken::Hex { value, digits } => self.verify_hex(value, digits),
        }
    }

    /// Compares fixed-width lowercase hexadecimal output.
    fn verify_hex(&mut self, value: u128, digits: u8) {
        let mut remaining = digits;
        while remaining != 0 {
            remaining -= 1;
            let shift = remaining as u32 * 4;
            self.verify_byte(hex_digit(((value >> shift) & 0x0f) as u8));
        }
    }

    /// Compares one byte and advances without reading past the advertised view.
    fn verify_byte(&mut self, expected: u8) {
        if self.offset >= self.actual.len() {
            self.matches = false;
            return;
        }
        if self.actual[self.offset] != expected {
            self.matches = false;
        }
        self.offset += 1;
    }

    /// Compares one byte slice in encounter order.
    fn verify_bytes(&mut self, expected: &[u8]) {
        let mut index = 0;
        while index < expected.len() {
            self.verify_byte(expected[index]);
            index += 1;
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

/// Exact-size const writer used only while initializing generated statics.
struct SignatureWriter<const N: usize> {
    /// Output bytes initialized from left to right.
    bytes: [u8; N],
    /// Next output position.
    len: usize,
}

impl<const N: usize> SignatureWriter<N> {
    /// Creates an empty writer for an exact-size result.
    const fn new() -> Self {
        Self { bytes: [0; N], len: 0 }
    }

    /// Writes one primitive output token.
    const fn write_token(&mut self, token: SignatureToken<'_>) {
        match token {
            SignatureToken::Empty => {}
            SignatureToken::Byte(byte) => self.write_byte(byte),
            SignatureToken::Bytes(bytes) => self.write_bytes(bytes),
            SignatureToken::Signature(signature) => {
                if signature.len != 0 {
                    // SAFETY: Nested ABI type signatures point to generated
                    // static buffers whose lengths were checked at creation.
                    self.write_bytes(unsafe { core::slice::from_raw_parts(signature.data, signature.len as usize) });
                }
            }
            SignatureToken::Guid(guid) => {
                let bytes = guid.to_uuid_bytes();
                let mut index = 0;
                while index < bytes.len() {
                    self.write_hex(bytes[index] as u128, 2);
                    index += 1;
                }
            }
            SignatureToken::Hex { value, digits } => self.write_hex(value, digits),
        }
    }

    /// Writes fixed-width lowercase hexadecimal output.
    const fn write_hex(&mut self, value: u128, digits: u8) {
        let mut remaining = digits;
        while remaining != 0 {
            remaining -= 1;
            let shift = remaining as u32 * 4;
            self.write_byte(hex_digit(((value >> shift) & 0x0f) as u8));
        }
    }

    /// Writes one byte into the exact-size buffer.
    const fn write_byte(&mut self, byte: u8) {
        assert!(self.len < N, "ABI signature writer exceeded its exact buffer");
        self.bytes[self.len] = byte;
        self.len += 1;
    }

    /// Writes one byte slice in encounter order.
    const fn write_bytes(&mut self, bytes: &[u8]) {
        let mut index = 0;
        while index < bytes.len() {
            self.write_byte(bytes[index]);
            index += 1;
        }
    }

    /// Returns the completed exact-size buffer.
    const fn finish(self) -> [u8; N] {
        assert!(self.len == N, "ABI signature writer did not fill its exact buffer");
        self.bytes
    }
}

/// Converts one hexadecimal nibble to canonical lowercase ASCII.
const fn hex_digit(value: u8) -> u8 {
    if value < 10 { b'0' + value } else { b'a' + (value - 10) }
}

/// Runtime encoder used only by tests that generate many signature variations.
#[cfg(test)]
fn encode_owned(parts: &[NcAbiSignaturePart<'_>]) -> std::vec::Vec<u8> {
    let mut bytes = std::vec::Vec::with_capacity(NcAbiSignature::__encoded_len(parts));
    bytes.extend_from_slice(NC_ABI_SIGNATURE_DOMAIN);
    for part in parts {
        let tokens = tokens_for_part(*part);
        for token in &tokens.values[..tokens.len] {
            append_owned_token(&mut bytes, *token);
        }
    }
    bytes
}

/// Appends one canonical primitive token to a test-owned byte vector.
#[cfg(test)]
fn append_owned_token(bytes: &mut std::vec::Vec<u8>, token: SignatureToken<'_>) {
    match token {
        SignatureToken::Empty => {}
        SignatureToken::Byte(byte) => bytes.push(byte),
        SignatureToken::Bytes(value) => bytes.extend_from_slice(value),
        SignatureToken::Signature(signature) => {
            // SAFETY: Unit-test signatures are generated statics or leaked
            // process-lifetime buffers created by `from_abi_parts` above.
            bytes.extend_from_slice(unsafe { signature.as_bytes().expect("valid test signature") });
        }
        SignatureToken::Guid(guid) => {
            for byte in guid.to_uuid_bytes() {
                append_owned_hex(bytes, byte as u128, 2);
            }
        }
        SignatureToken::Hex { value, digits } => append_owned_hex(bytes, value, digits),
    }
}

/// Appends one fixed-width lowercase hexadecimal value for the test encoder.
#[cfg(test)]
fn append_owned_hex(bytes: &mut std::vec::Vec<u8>, value: u128, digits: u8) {
    let mut remaining = digits;
    while remaining != 0 {
        remaining -= 1;
        let shift = remaining as u32 * 4;
        bytes.push(hex_digit(((value >> shift) & 0x0f) as u8));
    }
}

#[cfg(test)]
mod tests {
    use super::NcAbiSignature;
    use crate::{NcAbiSignaturePart, NcAbiType, NcGuid};

    const PARTS: [NcAbiSignaturePart<'static>; 3] = [
        NcAbiSignaturePart::Interface {
            iid: NcGuid::from_fields(0x1234_5678, 0x9abc, 0xdef0, [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0]),
        },
        NcAbiSignaturePart::Method { index: 7, name: "value" },
        NcAbiSignaturePart::Return {
            passing: crate::NcAbiPassing::Out,
            ty: NcAbiType::U32,
        },
    ];
    static BYTES: [u8; NcAbiSignature::__encoded_len(&PARTS)] = NcAbiSignature::__encode::<{ NcAbiSignature::__encoded_len(&PARTS) }>(&PARTS);
    const SIGNATURE: NcAbiSignature = NcAbiSignature::from_static(&BYTES);

    #[test]
    fn mangle_is_exact_and_human_readable() {
        // SAFETY: `SIGNATURE` references the generated static buffer above.
        let text = unsafe { core::str::from_utf8_unchecked(SIGNATURE.as_bytes().unwrap()) };
        assert_eq!(text, "ncabi1;I123456789abcdef0123456789abcdef0M0000000700000005valueO0204");
    }

    #[test]
    fn equality_compares_bytes_instead_of_static_addresses() {
        static COPY: [u8; BYTES.len()] = BYTES;
        assert_eq!(SIGNATURE, NcAbiSignature::from_static(&COPY));
        assert_ne!(SIGNATURE, NcAbiSignature::from_static_str("ncabi1;different"));
    }
}
