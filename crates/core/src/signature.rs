//! Precomputed, fixed-width ABI signatures used by runtime contract validation.
//!
//! IDL emits final SHA-256 bytes; runtime code neither serializes nor hashes a
//! declaration graph. Structural arrays derive a const digest from their element
//! identity and length. A signature view has exactly one pointer: null for an
//! absent contract, otherwise a pointer to 32 immutable bytes. See core ABI v16.

use core::hash::{Hash, Hasher};

/// One nullable pointer to an immutable SHA-256 digest, with C-compatible layout.
///
/// Generated views borrow provider-owned static storage; loaded providers remain
/// pinned while metadata or objects can reach it. The private query constructor
/// permits only synchronous borrows that cannot escape the query call.
///
/// Earlier signatures supported variable mangles and selectable hash algorithms.
/// SHA-256 became mandatory, leaving their pointer/count representation redundant.
/// A fixed-array pointer removes that count while sharing each provider's digest
/// across descriptors. Inline digests would copy 32 bytes into every view. Equality
/// must compare contents because independent providers have different addresses.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NcAbiSignature {
    /// Null means absence, never unchecked compatibility or an all-zero digest.
    /// Non-null points to one initialized immutable array for the view's lifetime.
    digest: *const [u8; Self::SHA256_LEN],
}

// SAFETY: Public safe construction uses immutable static storage; moving a view
// cannot invalidate it. Private call-scoped views must remain inside their query.
unsafe impl Send for NcAbiSignature {}
// SAFETY: Sharing immutable signature bytes cannot introduce concurrent mutation.
unsafe impl Sync for NcAbiSignature {}

impl NcAbiSignature {
    /// Exact width of every present ABI digest; no alternate encoding is supported.
    pub const SHA256_LEN: usize = 32;

    /// Explicit absence, permitted only where the surrounding descriptor allows it.
    pub const EMPTY: Self = Self { digest: core::ptr::null() };

    /// Borrows a complete static digest. Incorrect widths fail type checking.
    ///
    /// ```compile_fail
    /// use neocogi_com_core::NcAbiSignature;
    /// let _ = NcAbiSignature::from_static(&[0; 17]);
    /// ```
    ///
    /// Absence is explicit rather than an empty slice accepted by this constructor:
    /// ```compile_fail
    /// use neocogi_com_core::NcAbiSignature;
    /// let _ = NcAbiSignature::from_static(&[]);
    /// ```
    pub const fn from_static(bytes: &'static [u8; Self::SHA256_LEN]) -> Self {
        Self { digest: bytes }
    }

    /// Borrows a digest for an immediate checked interface query only.
    /// This view must not escape, enter metadata, or outlive the supplied array.
    pub(crate) fn from_borrowed(bytes: &[u8; Self::SHA256_LEN]) -> Self {
        Self { digest: bytes }
    }

    /// Tests explicit absence without dereferencing foreign storage.
    pub const fn is_empty(self) -> bool {
        self.digest.is_null()
    }

    /// Borrows the complete digest, or returns `None` for explicit absence.
    ///
    /// # Safety
    /// For a non-null pointer, all 32 bytes must be readable and immutable for
    /// the returned lifetime. Pointer shape cannot prove foreign memory validity.
    /// Generated static storage and pinned provider descriptors satisfy this;
    /// a synchronous query borrow lasts only for that call. Absence is handled
    /// by the consumer's contract, not converted into an empty byte slice.
    pub const unsafe fn as_digest<'a>(self) -> Option<&'a [u8; Self::SHA256_LEN]> {
        match self.digest.is_null() {
            true => None,
            // SAFETY: The caller establishes extent, readability and lifetime.
            false => Some(unsafe { &*self.digest }),
        }
    }

    /// Borrows a required static digest when deriving intrinsic array identities.
    /// ABI_TYPE storage is static under the unsafe AbiType producer contract.
    pub(crate) const fn const_bytes(self) -> &'static [u8; Self::SHA256_LEN] {
        // SAFETY: Only static ABI_TYPE metadata reaches intrinsic identity derivation.
        match unsafe { self.as_digest() } {
            Some(bytes) => bytes,
            None => panic!("an aggregate element needs a complete signature"),
        }
    }

    /// Compares canonical contents during macro-generated ancestry construction.
    #[doc(hidden)]
    pub const fn __const_eq(self, other: Self) -> bool {
        // SAFETY: Public constructors use static bytes; foreign producers and
        // private synchronous query callers uphold the same readable-view contract.
        match unsafe { (self.as_digest(), other.as_digest()) } {
            (None, None) => true,
            (None, Some(_)) | (Some(_), None) => false,
            (Some(left), Some(right)) => {
                let mut index = 0;
                while index < Self::SHA256_LEN {
                    if left[index] != right[index] {
                        return false;
                    }
                    index += 1;
                }
                true
            }
        }
    }
}
impl PartialEq for NcAbiSignature {
    /// Independent components may publish equal bytes at different addresses.
    fn eq(&self, other: &Self) -> bool {
        Self::__const_eq(*self, *other)
    }
}
impl Eq for NcAbiSignature {}
impl Hash for NcAbiSignature {
    /// Hashes presence and complete contents, consistently with bytewise equality.
    fn hash<H: Hasher>(&self, state: &mut H) {
        // SAFETY: Producers establish immutable readable storage for every live view.
        unsafe { self.as_digest() }.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hashes the complete logical view, using no provider-address information.
    fn content_hash(signature: NcAbiSignature) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        signature.hash(&mut hasher);
        hasher.finish()
    }

    /// Separate caller storage must negotiate like equal provider-local statics.
    #[test]
    fn independent_storage_compares_and_hashes_by_contents() {
        let left = [0x5a; 32];
        let right = [0x5a; 32];
        assert_ne!(left.as_ptr(), right.as_ptr());
        let left = NcAbiSignature::from_borrowed(&left);
        let right = NcAbiSignature::from_borrowed(&right);
        assert_eq!(left, right);
        assert_eq!(content_hash(left), content_hash(right));
        assert!(left.__const_eq(right));
    }

    /// No byte, including the final byte, is omitted from exact comparison.
    #[test]
    fn every_digest_byte_participates_in_equality() {
        let expected = NcAbiSignature::from_static(&[0x5a; 32]);
        for index in 0..32 {
            let mut bytes = [0x5a; 32];
            bytes[index] ^= 1;
            let changed = NcAbiSignature::from_borrowed(&bytes);
            assert_ne!(expected, changed, "byte {index}");
            assert!(!expected.__const_eq(changed));
        }
    }

    /// Absence differs from every present digest, including all-zero bytes.
    #[test]
    fn absence_and_zero_digest_are_distinct() {
        let zero = NcAbiSignature::from_static(&[0; 32]);
        assert!(NcAbiSignature::EMPTY.is_empty());
        assert!(!zero.is_empty());
        assert_ne!(NcAbiSignature::EMPTY, zero);
        assert_ne!(content_hash(NcAbiSignature::EMPTY), content_hash(zero));
        assert_eq!(NcAbiSignature::EMPTY, NcAbiSignature::EMPTY);
        // SAFETY: Both views are absent or refer to immutable static arrays.
        unsafe {
            assert_eq!(NcAbiSignature::EMPTY.as_digest(), None);
            assert_eq!(zero.as_digest(), Some(&[0; 32]));
        }
    }

    /// The signature view is thin on every pointer width supported by the core.
    #[test]
    fn fixed_digest_view_uses_one_pointer() {
        assert_eq!(core::mem::size_of::<NcAbiSignature>(), core::mem::size_of::<*const u8>());
        assert_eq!(core::mem::align_of::<NcAbiSignature>(), core::mem::align_of::<*const u8>());
        assert_eq!(core::mem::size_of::<crate::NcInterfaceId>(), 16 + core::mem::size_of::<*const u8>());
    }
}
