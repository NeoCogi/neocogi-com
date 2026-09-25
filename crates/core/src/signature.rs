//! Precomputed ABI signatures used by runtime contract validation.
//!
//! `neocogi` resolves and hashes IDL before invoking rustc. Generated bindings
//! contain only the final 32-byte digest, so compiling a host or plugin never
//! serializes or hashes its contract schema. This module owns only the stable
//! C-compatible view and byte-wise comparison used at runtime.

use core::hash::{Hash, Hasher};

use crate::status::{NC_E_POINTER, NcStatus};

/// Static precomputed ABI signature stored in C-compatible pointer/count form.
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
    /// Required byte width of every generator-computed SHA-256 ABI signature.
    pub const SHA256_LEN: usize = 32;

    /// Empty signature sentinel used when no ABI contract is present.
    pub const EMPTY: Self = Self { data: core::ptr::null(), len: 0 };

    /// Creates a signature view over immutable static validation bytes.
    ///
    /// Generated code uses this constructor with the exact-size buffer emitted
    /// for an attributed ABI declaration.
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        assert!(
            bytes.is_empty() || bytes.len() == Self::SHA256_LEN,
            "ABI signature must be empty or one SHA-256 digest"
        );
        Self {
            data: if bytes.is_empty() { core::ptr::null() } else { bytes.as_ptr() },
            len: bytes.len() as u32,
        }
    }

    /// Borrows one digest only for an immediate in-process interface query.
    /// The returned view must never outlive `bytes` or be stored in metadata.
    pub(crate) fn from_borrowed(bytes: &[u8; Self::SHA256_LEN]) -> Self {
        Self { data: bytes.as_ptr(), len: Self::SHA256_LEN as u32 }
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
