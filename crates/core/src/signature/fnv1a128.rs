//! FNV-1a-128 encoding for canonical COM ABI signature bytes.

use super::{NcAbiSignature, NcAbiSignaturePart, SignatureByteCursor};
use crate::{NC_E_INVALID_ARG, NcStatus};

/// Number of bytes in one FNV-1a-128 signature.
pub(super) const SIGNATURE_LEN: usize = 16;

/// Domain separator that keeps ABI fingerprints distinct from other FNV uses.
const DOMAIN: &[u8] = b"neocogi-com:abi-fnv1a128:v1\0";

/// Standard FNV-1a-128 offset basis.
const OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;

/// Standard FNV-1a-128 prime.
const PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;

/// Encodes one logical ABI contract into an exact-size FNV fingerprint.
pub(super) const fn encode<const N: usize>(parts: &[NcAbiSignaturePart<'_>]) -> [u8; N] {
    assert!(N == SIGNATURE_LEN, "incorrect FNV-1a-128 signature buffer length");
    let mut hasher = Fnv1a128::new();
    hasher.update_parts(parts, true);
    let digest = hasher.finish();
    let mut output = [0; N];
    let mut index = 0;
    while index < SIGNATURE_LEN {
        output[index] = digest[index];
        index += 1;
    }
    output
}

/// Incremental verifier used while reconstructing descriptor contracts.
pub(crate) struct Verifier<'a> {
    /// Advertised component bytes retained until the comparison is complete.
    actual: &'a [u8],
    /// FNV state receiving reconstructed canonical bytes.
    hasher: Fnv1a128,
}

impl<'a> Verifier<'a> {
    /// Starts a verifier for one fixed-width advertised signature.
    ///
    /// # Safety
    ///
    /// `signature` must point to readable immutable storage for `'a`.
    pub(crate) unsafe fn new(signature: NcAbiSignature) -> Result<Self, NcStatus> {
        // SAFETY: The caller establishes the signature storage contract.
        let actual = unsafe { signature.as_bytes()? };
        if actual.len() != SIGNATURE_LEN {
            return Err(NC_E_INVALID_ARG);
        }
        let mut hasher = Fnv1a128::new();
        hasher.update_parts(&[], true);
        Ok(Self { actual, hasher })
    }

    /// Appends one reconstructed logical ABI record.
    pub(crate) fn verify_part(&mut self, part: NcAbiSignaturePart<'_>) {
        self.hasher.update_parts(&[part], false);
    }

    /// Compares the final fingerprint with the advertised bytes.
    pub(crate) fn finish(self) -> bool {
        self.actual == self.hasher.finish()
    }
}

/// Small mutable FNV accumulator shared by const generation and validation.
#[derive(Clone, Copy)]
struct Fnv1a128 {
    /// Accumulator after all accepted bytes.
    value: u128,
}

impl Fnv1a128 {
    /// Creates a domain-separated FNV-1a-128 stream.
    const fn new() -> Self {
        let mut out = Self { value: OFFSET };
        out.update(DOMAIN);
        out
    }

    /// Appends canonical bytes for a complete signature or an incremental part.
    const fn update_parts(&mut self, parts: &[NcAbiSignaturePart<'_>], include_signature_domain: bool) {
        let mut cursor = SignatureByteCursor::new(parts, include_signature_domain);
        while let Some(byte) = cursor.next() {
            self.update_byte(byte);
        }
    }

    /// Appends one byte using the xor-then-multiply FNV-1a recurrence.
    const fn update_byte(&mut self, byte: u8) {
        self.value ^= byte as u128;
        self.value = self.value.wrapping_mul(PRIME);
    }

    /// Appends a byte slice in encounter order.
    const fn update(&mut self, bytes: &[u8]) {
        let mut index = 0;
        while index < bytes.len() {
            self.update_byte(bytes[index]);
            index += 1;
        }
    }

    /// Returns the stable big-endian fingerprint bytes.
    const fn finish(self) -> [u8; SIGNATURE_LEN] {
        self.value.to_be_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::{OFFSET, PRIME};

    /// Computes plain FNV-1a-128 for published known-answer vectors.
    fn raw_fnv1a128(bytes: &[u8]) -> [u8; 16] {
        let mut value = OFFSET;
        for byte in bytes {
            value ^= *byte as u128;
            value = value.wrapping_mul(PRIME);
        }
        value.to_be_bytes()
    }

    #[test]
    fn fnv1a128_matches_known_answers() {
        assert_eq!(raw_fnv1a128(b""), 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d_u128.to_be_bytes());
        assert_eq!(raw_fnv1a128(b"a"), 0xd228_cb69_6f1a_8caf_7891_2b70_4e4a_8964_u128.to_be_bytes());
        assert_eq!(raw_fnv1a128(b"foobar"), 0x343e_1662_793c_64bf_6f0d_3597_ba44_6f18_u128.to_be_bytes());
    }
}
