//! ABI hash values and the minimal const-evaluable SHA-256 implementation.
//!
//! Interface compatibility is represented by a canonical sequence of typed
//! [`NcAbiHashPart`] records rather than Rust syntax or diagnostic signatures.
//! This module encodes those records with an explicit schema prefix and hashes
//! them using an allocation-free SHA-256 engine that also works in constants.
//!
//! This module owns the public `NcAbiHash` value type, the incremental
//! descriptor hasher used by validation, and the small in-tree SHA-256 engine
//! behind them. It intentionally avoids external digest crates because `nc-com`
//! computes ABI hashes in `const` contexts.

use crate::guid::NcGuid;
use crate::metadata::{
    NcAbiAggregateKind, NcAbiFieldLayout, NcAbiHashPart, NcAbiPassing, NcAbiStructLayout,
    NcAbiThreadingMode, NcAbiType,
};

const NC_ABI_HASH_SCHEMA: &[u8] = b"nc-com:abi-hash:v2\0";

/// SHA-256 ABI hash bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NcAbiHash {
    /// Raw SHA-256 digest bytes.
    pub bytes: [u8; 32],
}

impl NcAbiHash {
    /// All-zero hash sentinel.
    pub const ZERO: Self = Self { bytes: [0; 32] };

    /// Creates a hash from raw SHA-256 digest bytes.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    /// Returns whether all digest bytes are zero.
    pub const fn is_zero(self) -> bool {
        let bytes = self.bytes;
        let word0 = u64::from_ne_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        let word1 = u64::from_ne_bytes([
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]);
        let word2 = u64::from_ne_bytes([
            bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
        ]);
        let word3 = u64::from_ne_bytes([
            bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30], bytes[31],
        ]);

        (word0 | word1 | word2 | word3) == 0
    }

    /// Hashes canonical typed ABI parts with SHA-256.
    ///
    /// This helper is primarily used by generated interface metadata. It is
    /// const so contracts can hash referenced IIDs and aggregate layouts without
    /// requiring the procedural macro to know those values while expanding.
    pub const fn from_abi_parts(parts: &[NcAbiHashPart<'_>]) -> Self {
        let mut hasher = NcSha256::new().update(NC_ABI_HASH_SCHEMA);
        let mut index = 0;
        while index < parts.len() {
            hasher = hasher.update_abi_hash_part(parts[index]);
            index += 1;
        }
        NcAbiHash::from_bytes(hasher.finalize())
    }

    /// Hashes a physical aggregate layout.
    ///
    /// This is primarily used by macro-generated plain aggregate types. It records
    /// the observable ABI facts that must match across independently built
    /// artifacts: aggregate kind, size, alignment, field order, field offset, and
    /// field ABI type.
    pub const fn from_aggregate_layout(
        kind: NcAbiAggregateKind,
        size: usize,
        align: usize,
        fields: &[NcAbiFieldLayout],
    ) -> Self {
        let mut hasher = NcSha256::new()
            .update(NC_ABI_HASH_SCHEMA)
            .update_abi_hash_part(NcAbiHashPart::Aggregate { kind })
            .update_abi_hash_part(NcAbiHashPart::Layout { size, align });
        let mut index = 0;
        while index < fields.len() {
            let field = fields[index];
            hasher = hasher.update_abi_hash_part(NcAbiHashPart::Field {
                index: field.index,
                offset: field.offset,
                ty: field.ty,
            });
            index += 1;
        }
        NcAbiHash::from_bytes(hasher.finalize())
    }

    /// Hashes a macro-generated COM struct layout.
    pub const fn from_struct_layout(layout: NcAbiStructLayout) -> Self {
        Self::from_aggregate_layout(
            NcAbiAggregateKind::Struct,
            layout.size,
            layout.align,
            layout.fields,
        )
    }
}

/// Incremental ABI hash builder for descriptor validation.
pub(crate) struct NcAbiHasher {
    inner: NcSha256,
}

impl NcAbiHasher {
    /// Starts a descriptor-validation hash with the ABI schema prefix.
    pub(crate) const fn new() -> Self {
        Self {
            inner: NcSha256::new().update(NC_ABI_HASH_SCHEMA),
        }
    }

    /// Appends one canonical logical contract record.
    pub(crate) const fn update_abi_hash_part(self, part: NcAbiHashPart<'_>) -> Self {
        Self {
            inner: self.inner.update_abi_hash_part(part),
        }
    }

    /// Finalizes the accumulated contract as an [`NcAbiHash`].
    pub(crate) const fn finalize(self) -> NcAbiHash {
        NcAbiHash::from_bytes(self.inner.finalize())
    }
}

/// Initial SHA-256 chaining state from FIPS 180-4 section 5.3.3.
const SHA256_INITIAL: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

/// SHA-256 round constants from FIPS 180-4 section 4.2.2.
const SHA256_K: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// Incremental SHA-256 state.
///
/// The type consumes `self` on update/finalize so callers can chain operations
/// inside `const fn` without mutable references escaping into constants.
#[derive(Clone, Copy)]
pub(crate) struct NcSha256 {
    h: [u32; 8],
    block: [u8; 64],
    block_len: usize,
    byte_len: u64,
}

impl NcSha256 {
    /// Creates a fresh SHA-256 state.
    pub(crate) const fn new() -> Self {
        Self {
            h: SHA256_INITIAL,
            block: [0; 64],
            block_len: 0,
            byte_len: 0,
        }
    }

    /// Appends a byte slice to the hash input.
    pub(crate) const fn update(mut self, bytes: &[u8]) -> Self {
        let mut index = 0;
        while index < bytes.len() {
            self = self.update_byte(bytes[index]);
            index += 1;
        }
        self
    }

    /// Appends a single byte and compresses full 64-byte blocks as they fill.
    pub(crate) const fn update_byte(mut self, byte: u8) -> Self {
        self.block[self.block_len] = byte;
        self.block_len += 1;
        self.byte_len += 1;
        if self.block_len == 64 {
            self = self.process_block();
            self.block_len = 0;
        }
        self
    }

    /// Appends a `u32` in SHA-256's big-endian byte order.
    pub(crate) const fn update_u32(self, value: u32) -> Self {
        self.update(&value.to_be_bytes())
    }

    /// Appends a `u64` in SHA-256's big-endian byte order.
    pub(crate) const fn update_u64(self, value: u64) -> Self {
        self.update(&value.to_be_bytes())
    }

    /// Finalizes the hash and returns the raw 32-byte SHA-256 digest.
    pub(crate) const fn finalize(mut self) -> [u8; 32] {
        let bit_len = self.byte_len * 8;

        // SHA-256 padding is one set bit, zero bytes up to the length slot, and
        // the original input length as a 64-bit big-endian bit count.
        self = self.update_byte(0x80);
        while self.block_len != 56 {
            self = self.update_byte(0);
        }

        let len = bit_len.to_be_bytes();
        let mut index = 0;
        while index < len.len() {
            self = self.update_byte(len[index]);
            index += 1;
        }

        let mut bytes = [0; 32];
        index = 0;
        while index < self.h.len() {
            let word = self.h[index].to_be_bytes();
            let offset = index * 4;
            bytes[offset] = word[0];
            bytes[offset + 1] = word[1];
            bytes[offset + 2] = word[2];
            bytes[offset + 3] = word[3];
            index += 1;
        }
        bytes
    }

    /// Compresses the current 64-byte block into the chaining state.
    const fn process_block(mut self) -> Self {
        let mut words = [0u32; 64];
        let mut index = 0;
        while index < 16 {
            let offset = index * 4;
            words[index] = u32::from_be_bytes([
                self.block[offset],
                self.block[offset + 1],
                self.block[offset + 2],
                self.block[offset + 3],
            ]);
            index += 1;
        }

        // Extend the 16 input words into the 64-word message schedule.
        while index < 64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
            index += 1;
        }

        let mut a = self.h[0];
        let mut b = self.h[1];
        let mut c = self.h[2];
        let mut d = self.h[3];
        let mut e = self.h[4];
        let mut f = self.h[5];
        let mut g = self.h[6];
        let mut h = self.h[7];

        index = 0;
        while index < 64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[index])
                .wrapping_add(words[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
            index += 1;
        }

        self.h[0] = self.h[0].wrapping_add(a);
        self.h[1] = self.h[1].wrapping_add(b);
        self.h[2] = self.h[2].wrapping_add(c);
        self.h[3] = self.h[3].wrapping_add(d);
        self.h[4] = self.h[4].wrapping_add(e);
        self.h[5] = self.h[5].wrapping_add(f);
        self.h[6] = self.h[6].wrapping_add(g);
        self.h[7] = self.h[7].wrapping_add(h);
        self
    }
}

// ABI hash serialization tags. These bytes are part of the canonical ABI hash
// schema and must change only with an explicit schema/version bump.
impl NcSha256 {
    /// Encodes a pointer-sized layout value as a stable big-endian `u64`.
    const fn update_usize(self, value: usize) -> Self {
        self.update_u64(value as u64)
    }

    /// Appends a signed variant tag using its full two's-complement width.
    const fn update_i128(self, value: i128) -> Self {
        self.update(&value.to_be_bytes())
    }

    /// Appends a length-prefixed UTF-8 string.
    const fn update_str(self, value: &str) -> Self {
        self.update_u32(value.len() as u32).update(value.as_bytes())
    }

    /// Appends a GUID in canonical UUID byte order.
    const fn update_guid(self, value: NcGuid) -> Self {
        self.update(&value.to_uuid_bytes())
    }

    /// Appends the raw bytes of a nested ABI hash.
    const fn update_hash(self, value: NcAbiHash) -> Self {
        self.update(&value.bytes)
    }

    /// Serializes one logical hash-grammar record including its stable tag.
    const fn update_abi_hash_part(self, part: NcAbiHashPart<'_>) -> Self {
        match part {
            NcAbiHashPart::Interface { iid } => self.update_byte(0x01).update_guid(iid),
            NcAbiHashPart::BaseInterface { interface } => self.update_byte(0x0a).update_guid(interface.iid).update_hash(interface.abi_hash),
            NcAbiHashPart::Method { index, name } => self.update_byte(0x02).update_u32(index).update_str(name),
            NcAbiHashPart::Receiver { receiver } => self.update_byte(0x0b).update_receiver(receiver),
            NcAbiHashPart::Arg { index, passing, ty } => self.update_byte(0x03).update_u32(index).update_passing(passing).update_abi_type(ty),
            NcAbiHashPart::Return { passing, ty } => self.update_byte(0x04).update_passing(passing).update_abi_type(ty),
            NcAbiHashPart::Status => self.update_byte(0x05),
            NcAbiHashPart::Layout { size, align } => self
                .update_byte(0x06)
                .update_usize(size)
                .update_usize(align),
            NcAbiHashPart::Field { index, offset, ty } => self
                .update_byte(0x07)
                .update_u32(index)
                .update_usize(offset)
                .update_abi_type(ty),
            NcAbiHashPart::Variant { index, tag, ty } => self
                .update_byte(0x08)
                .update_u32(index)
                .update_i128(tag)
                .update_abi_type(ty),
            NcAbiHashPart::Aggregate { kind } => self.update_byte(0x09).update_aggregate_kind(kind),
        }
    }

    /// Serializes a canonical ABI type and any nested identity data.
    const fn update_abi_type(self, ty: NcAbiType) -> Self {
        match ty {
            NcAbiType::Void => self.update_byte(0x00),
            NcAbiType::Bool => self.update_byte(0x01),
            NcAbiType::U8 => self.update_byte(0x02),
            NcAbiType::U16 => self.update_byte(0x03),
            NcAbiType::U32 => self.update_byte(0x04),
            NcAbiType::U64 => self.update_byte(0x05),
            NcAbiType::USize => self.update_byte(0x06),
            NcAbiType::I8 => self.update_byte(0x07),
            NcAbiType::I16 => self.update_byte(0x08),
            NcAbiType::I32 => self.update_byte(0x09),
            NcAbiType::I64 => self.update_byte(0x0a),
            NcAbiType::ISize => self.update_byte(0x0b),
            NcAbiType::F32 => self.update_byte(0x0c),
            NcAbiType::F64 => self.update_byte(0x0d),
            NcAbiType::NcGuid => self.update_byte(0x0e),
            NcAbiType::NcStatus => self.update_byte(0x0f),
            NcAbiType::NcAbiHash => self.update_byte(0x10),
            NcAbiType::Aggregate(hash) => self.update_byte(0x11).update_hash(hash),
            // Pointer boundaries check full identities; the local grammar terminates at IIDs.
            NcAbiType::Interface { iid } => self.update_byte(0x12).update_guid(iid),
        }
    }

    /// Serializes a passing mode and its optional threading capability.
    const fn update_passing(self, passing: NcAbiPassing) -> Self {
        match passing {
            NcAbiPassing::Void => self.update_byte(0x00),
            NcAbiPassing::Value => self.update_byte(0x01),
            NcAbiPassing::Out => self.update_byte(0x02),
            NcAbiPassing::CopyFromBufferOffset => self.update_byte(0x03),
            NcAbiPassing::CopyToBufferOffset => self.update_byte(0x04),
            NcAbiPassing::ComRef { mode } => self.update_byte(0x05).update_threading_mode(mode),
            NcAbiPassing::ComPtrOut { mode } => self.update_byte(0x06).update_threading_mode(mode),
        }
    }

    /// Serializes the stable tag for a pointer threading mode.
    const fn update_threading_mode(self, mode: NcAbiThreadingMode) -> Self {
        match mode {
            NcAbiThreadingMode::ThreadLocal => self.update_byte(0x00),
            NcAbiThreadingMode::ThreadSafe => self.update_byte(0x01),
        }
    }

    /// Serializes the stable tag for a method receiver mode.
    const fn update_receiver(self, receiver: crate::NcAbiReceiver) -> Self {
        match receiver {
            crate::NcAbiReceiver::Shared => self.update_byte(0x00),
            crate::NcAbiReceiver::Mutable => self.update_byte(0x01),
        }
    }

    /// Serializes the stable tag for an aggregate category.
    const fn update_aggregate_kind(self, kind: NcAbiAggregateKind) -> Self {
        match kind {
            NcAbiAggregateKind::Struct => self.update_byte(0x00),
            NcAbiAggregateKind::Union => self.update_byte(0x01),
            NcAbiAggregateKind::TaggedEnum => self.update_byte(0x02),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NcAbiHash, NcSha256};
    use crate::{
        NcAbiAggregateKind, NcAbiHashPart, NcAbiPassing, NcAbiThreadingMode, NcAbiType, NcGuid,
    };

    #[test]
    fn inherited_contract_hash_matches_independent_serialization() {
        // Golden computed with Python hashlib over the documented wire grammar:
        // domain + 0x01 + UUID bytes 0..15 + 0x0a + UUID bytes 16..31 + [42; 32].
        let hash = NcAbiHash::from_abi_parts(&[
            NcAbiHashPart::Interface { iid: NcGuid::from_uuid_bytes(core::array::from_fn(|i| i as u8)) },
            NcAbiHashPart::BaseInterface { interface: crate::NcInterfaceId::new(
                NcGuid::from_uuid_bytes(core::array::from_fn(|i| (i + 16) as u8)), NcAbiHash::from_bytes([42; 32]),
            ) },
        ]);
        assert_eq!(hash.bytes, [
            0xfd, 0x4e, 0xc7, 0x74, 0x14, 0x90, 0xe8, 0x08, 0x6d, 0x72, 0x38, 0x27, 0x74, 0x1d, 0x49, 0xf9,
            0x31, 0x7f, 0x20, 0x2a, 0xcc, 0x97, 0xab, 0xc5, 0xf2, 0x0e, 0x9f, 0x8b, 0xc0, 0xef, 0x7f, 0xbf,
        ]);
    }

    // Expected SHA-256 digests for input bytes `0, 1, 2, ... len - 1`.
    // Generated with `sha256sum` and cross-checked with `openssl dgst -sha256`:
    //
    // python3 - <<'PY'
    // import os, subprocess, tempfile
    //
    // print("const SHA256_PREFIX_LENGTH_VECTORS: [&str; 129] = [")
    // with tempfile.TemporaryDirectory() as td:
    //     path = os.path.join(td, "input.bin")
    //     for n in range(129):
    //         data = bytes(range(n))
    //         with open(path, "wb") as f:
    //             f.write(data)
    //         sha256sum = subprocess.check_output(["sha256sum", path], text=True).split()[0]
    //         openssl = subprocess.check_output(
    //             ["openssl", "dgst", "-sha256", "-r", path],
    //             text=True,
    //         ).split()[0]
    //         assert sha256sum == openssl
    //         print(f'    "{sha256sum}", // len {n}')
    // print("];")
    // PY
    const SHA256_PREFIX_LENGTH_VECTORS: [&str; 129] = [
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", // len 0
        "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d", // len 1
        "b413f47d13ee2fe6c845b2ee141af81de858df4ec549a58b7970bb96645bc8d2", // len 2
        "ae4b3280e56e2faf83f414a6e3dabe9d5fbe18976544c05fed121accb85b53fc", // len 3
        "054edec1d0211f624fed0cbca9d4f9400b0e491c43742af2c5b0abebf0c990d8", // len 4
        "08bb5e5d6eaac1049ede0893d30ed022b1a4d9b5b48db414871f51c9cb35283d", // len 5
        "17e88db187afd62c16e5debf3e6527cd006bc012bc90b51a810cd80c2d511f43", // len 6
        "57355ac3303c148f11aef7cb179456b9232cde33a818dfda2c2fcb9325749a6b", // len 7
        "8a851ff82ee7048ad09ec3847f1ddf44944104d2cbd17ef4e3db22c6785a0d45", // len 8
        "f8348e0b1df00833cbbbd08f07abdecc10c0efb78829d7828c62a7f36d0cc549", // len 9
        "1f825aa2f0020ef7cf91dfa30da4668d791c5d4824fc8e41354b89ec05795ab3", // len 10
        "78a6273103d17c39a0b6126e226cec70e33337f4bc6a38067401b54a33e78ead", // len 11
        "fff3a9bcdd37363d703c1c4f9512533686157868f0d4f16a0f02d0f1da24f9a2", // len 12
        "86eba947d50c2c01570fe1bb5ca552958dabbdbb59b0657f0f26e21ff011e5c7", // len 13
        "ab107f1bd632d3c3f5c724a99d024f7faa033f33c07696384b604bfe78ac352d", // len 14
        "7071fc3188fde7e7e500d4768f1784bede1a22e991648dcab9dc3219acff1d4c", // len 15
        "be45cb2605bf36bebde684841a28f0fd43c69850a3dce5fedba69928ee3a8991", // len 16
        "3e5718fea51a8f3f5baca61c77afab473c1810f8b9db330273b4011ce92c787e", // len 17
        "7a096cc12702bcfa647ee070d4f3ba4c2d1d715b484b55b825d0edba6545803b", // len 18
        "5f9a753613d87b8a17302373c4aee56faa310d3b24b6ae1862d673aa22e1790f", // len 19
        "e7aebf577f60412f0312d442c70a1fa6148c090bf5bab404caec29482ae779e8", // len 20
        "75aee9dcc9fbe7ddc9394f5bc5d38d9f5ad361f0520f7ceab59616e38f5950b5", // len 21
        "22cb4df00cddd6067ad5cfa2bba9857f21a06843e1a6e39ad1a68cb9a45ab8b7", // len 22
        "f6a954a68555187d88cd9a026940d15ab2a7e24c7517d21ceeb028e93c96f318", // len 23
        "1d64add2a6388367c9bc2d1f1b384b069a6ef382cdaaa89771dd103e28613a25", // len 24
        "b729ce724d9a48d3884dbfcbee1d3793d922b29fa9d639e7290af4978263772b", // len 25
        "b858da80d8a57dc546905fd147612ebddd3c9188620405d058f9ee5ab1e6bc52", // len 26
        "d78750726155a89c9131d0ecf2704b973b8710865bf9e831845de4f2dcbc19da", // len 27
        "dc27f8e8ee2d08a2bccbb2dbd6c8e07ffba194101fc3458c34ded55f72c0971a", // len 28
        "d09bea65dff48928a14b79741de3274b646f55ac898b71a66fa3eae2d9facd77", // len 29
        "f2192584b67da35dfc26f743e5f53bb0376046f899dc6dabd5e7b541ae86c32f", // len 30
        "4f23c2ca8c5c962e50cd31e221bfb6d0adca19111dca8e0c62598ff146dd19c4", // len 31
        "630dcd2966c4336691125448bbb25b4ff412a49c732db2c8abc1b8581bd710dd", // len 32
        "5d8fcfefa9aeeb711fb8ed1e4b7d5c8a9bafa46e8e76e68aa18adce5a10df6ab", // len 33
        "14cdbf171499f86bd18b262243d669067efbdbb5431a48289cf02f2b5448b3d4", // len 34
        "f12dd12340cb84e4d0d9958d62be7c59bb8f7243a7420fd043177ac542a26aaa", // len 35
        "5d7e2d9b1dcbc85e7c890036a2cf2f9fe7b66554f2df08cec6aa9c0a25c99c21", // len 36
        "f4d285f47a1e4959a445ea6528e5df3efab041fa15aad94db1e2600b3f395518", // len 37
        "a2fd0e15d72c9d18f383e40016f9ddc706673c54252084285aaa47a812552577", // len 38
        "4aba23aea5e2a91b7807cf3026cdd10a1c38533ce55332683d4ccb88456e0703", // len 39
        "5faa4eec3611556812c2d74b437c8c49add3f910f10063d801441f7d75cd5e3b", // len 40
        "753629a6117f5a25d338dff10f4dd3d07e63eecc2eaf8eabe773f6399706fe67", // len 41
        "40a1ed73b46030c8d7e88682078c5ab1ae5a2e524e066e8c8743c484de0e21e5", // len 42
        "c033843682818c475e187d260d5e2edf0469862dfa3bb0c116f6816a29edbf60", // len 43
        "17619ec4250ef65f083e2314ef30af796b6f1198d0fddfbb0f272930bf9bb991", // len 44
        "a8e960c769a9508d098451e3d74dd5a2ac6c861eb0341ae94e9fc273597278c9", // len 45
        "8ebfeb2e3a159e9f39ad7cc040e6678dade70d4f59a67d529fa76af301ab2946", // len 46
        "ef8a7781a95c32fa02ebf511eda3dc6e273be59cb0f9e20a4f84d54f41427791", // len 47
        "4dbdc2b2b62cb00749785bc84202236dbc3777d74660611b8e58812f0cfde6c3", // len 48
        "7509fe148e2c426ed16c990f22fe8116905c82c561756e723f63223ace0e147e", // len 49
        "a622e13829e488422ee72a5fc92cb11d25c3d0f185a1384b8138df5074c983bf", // len 50
        "3309847cee454b4f99dcfe8fdc5511a7ba168ce0b6e5684ef73f9030d009b8b5", // len 51
        "c4c6540a15fc140a784056fe6d9e13566fb614ecb2d9ac0331e264c386442acd", // len 52
        "90962cc12ae9cdae32d7c33c4b93194b11fac835942ee41b98770c6141c66795", // len 53
        "675f28acc0b90a72d1c3a570fe83ac565555db358cf01826dc8eefb2bf7ca0f3", // len 54
        "463eb28e72f82e0a96c0a4cc53690c571281131f672aa229e0d45ae59b598b59", // len 55
        "da2ae4d6b36748f2a318f23e7ab1dfdf45acdc9d049bd80e59de82a60895f562", // len 56
        "2fe741af801cc238602ac0ec6a7b0c3a8a87c7fc7d7f02a3fe03d1c12eac4d8f", // len 57
        "e03b18640c635b338a92b82cce4ff072f9f1aba9ac5261ee1340f592f35c0499", // len 58
        "bd2de8f5dd15c73f68dfd26a614080c2e323b2b51b1b5ed9d7933e535d223bda", // len 59
        "0ddde28e40838ef6f9853e887f597d6adb5f40eb35d5763c52e1e64d8ba3bfff", // len 60
        "4b5c2783c91ceccb7c839213bcbb6a902d7fe8c2ec866877a51f433ea17f3e85", // len 61
        "c89da82cbcd76ddf220e4e9091019b9866ffda72bee30de1effe6c99701a2221", // len 62
        "29af2686fd53374a36b0846694cc342177e428d1647515f078784d69cdb9e488", // len 63
        "fdeab9acf3710362bd2658cdc9a29e8f9c757fcf9811603a8c447cd1d9151108", // len 64
        "4bfd2c8b6f1eec7a2afeb48b934ee4b2694182027e6d0fc075074f2fabb31781", // len 65
        "b6dfd259f6e0d07deb658a88148f8253f9bbbb74ddd6db3edbe159a56bc35073", // len 66
        "8fa5913b62847d42bb4b464e00a72c612d2ab0df2af0b9a96af8d323fa509077", // len 67
        "7ded979c0153ebb9ef28a15a314d0b27b41c4f8eed700b54974b48eb3ecaf91c", // len 68
        "1cf3aa651dcf35dbfe296e770ad7ebc4e00bcccd0224db296183dc952d0008c9", // len 69
        "5767d69a906d4860db9079eb7e90ab4a543e5cb032fce846554aef6ceb600e1d", // len 70
        "8189e3d54767d51e8d1942659a9e2905f9ec3ae72860c16a66e75b8cc9bd2087", // len 71
        "107de2bc788e11029f7851f8e1b0b5afb4e34379c709fc840689ebd3d1f51b5b", // len 72
        "169f6f093a9be82febe1a6a4471425697ec25d5040b472c5b1822aeea2625988", // len 73
        "2087ebd358ae3ea2a092fc19c2dfee57c5f0860296bc7b057c14e1227c5cb9d1", // len 74
        "182ab56f7739e43cee0b9ba1e92c4b2a81b088705516a5243910159744f21be9", // len 75
        "081f6c68899a48a1be455a55416104921d2fe4bdae696f4b72f9d9626a47915e", // len 76
        "5ce02376cc256861b78f87e34783814ba1aec6d09ab500d579ed8ee95c8afcc8", // len 77
        "b93e407404e3e95f20fd647365e0e7f46afabe9af1ff083af996135e00d54009", // len 78
        "e81fa832b37be8ed8f79da29987aa4d61310dcb14b2859dedf8fb1daa2541fd3", // len 79
        "c56705fea5b110b8dc63688533ced21167e628017387c885423b835a55edd5ef", // len 80
        "c2226285d08a245a17058ed2d24ad095b714f608ae364fddf119e0a7df890540", // len 81
        "f9c270da8793221a6809ac685fdd4f5387e0fe1ee6aaf01c74f1e0a719621614", // len 82
        "e69befd6ef7f685c36e343ac1702d87ad6a0e4ac8c0d5c521d04aad4ef0b7458", // len 83
        "4e3033562ad74a7d43eb5ff5fc2382622c6307cb10e245ad62da77c4c63cb178", // len 84
        "2ea17629472564a59e5eb845a2cdd04f442df2ff26bcc866e400f77158d612a1", // len 85
        "b90223df74dd49a8a1461f340f2d7a90f96903ccbb5bc3c74ea3658fc8948b20", // len 86
        "e0209f42b927ec9c0f6d6a76007ed540e9bdd6e427b3368a1ea6c5e7565972dd", // len 87
        "10d9bd424114319c0999adf6288f74060cd8918ef1228827a6269b2bf0f0880c", // len 88
        "7d1978a65ac94dbbcdc62e3d81850299fe157dd9b7bd9e01b170156210d2815a", // len 89
        "e052dff9e1c94aaa49556f86fad55029a4875839fda57f5005f4c4403876b256", // len 90
        "58d29459b2130a2e151252d408b95e6dac424c564062eb911cc76440cb926ca0", // len 91
        "4e4530c392316f598e1bd07f32166380a8f712a33a48e9eb4247131ec5dc05d3", // len 92
        "a09c9d3e42342c7dea44edb4aeb48cf6727cacd8032a12cf77a25829fc249d32", // len 93
        "eb978d0f1ac03ce5c3510b5f4a16073a7a2bdc15c4ab7777dcf01030cc316667", // len 94
        "7d1905a3ace827ea1ac51c4fa08c281ed3be87e7f4e928d696bfde35c8f2dc0f", // len 95
        "08359b108fa567f5dcf319fa3434da6abbc1d595f426372666447f09cc5a87dc", // len 96
        "a7b3830ffab0f2bbabbef6df0b169a7917008bf238880bbf8c20b8e000077312", // len 97
        "b4f5d9b1555994c5ebaebd82918d560a3bf82962a171a1614e7551939e943366", // len 98
        "014ecaea1b378900f1212898c6ddb01565d81af1d0ef78df5e28d46e9caf7cfc", // len 99
        "bce0aff19cf5aa6a7469a30d61d04e4376e4bbf6381052ee9e7f33925c954d52", // len 100
        "4565d7b898ccea3139ad260f9273115f806b30079d7683218c4e3ecd43af3b33", // len 101
        "ddadeb660fe8902c9fb2db9b6cf237c9ce5b31753398085c4367eb5910b9cc13", // len 102
        "c15a8928131f6687dd10f3c115ddf8d7c8f2df7e18d12c08c4fd16f666ce60ba", // len 103
        "ae8e3d799b1353a39815f90eceebefa265cc448fe39faf2008cb20784cb2df9f", // len 104
        "98545371a3d9981abe5ab4a32a1d7b2fadd9801d89da52a94a4f78a42740d21c", // len 105
        "6323dce2f8b3a04dcea8d205602348c40403cb200c677eb1a1c0fe37edb6eb2f", // len 106
        "8150f7c5da910d709ff02ddf85dd293c6a2672633de8cda30f2e0aa58b14b0c4", // len 107
        "44d21db70716bd7644cb0d819fa6791805ebc526ea32996a60e41dc753fcfafc", // len 108
        "b9b7c375cca45db19466ebd0fe7c9e147948cc42c1c90f0579728cfb2651956d", // len 109
        "a47a551b01e55aaaa015531a4fa26a666f1ebd4ba4573898de712b8b5e0ca7e9", // len 110
        "60780e9451bdc43cf4530ffc95cbb0c4eb24dae2c39f55f334d679e076c08065", // len 111
        "09373f127d34e61dbbaa8bc4499c87074f2ddb10e1b465f506d7d70a15011979", // len 112
        "13aaa9b5fb739cdb0e2af99d9ac0a409390adc4d1cb9b41f1ef94f8552060e92", // len 113
        "5b0a32f1219524f5d72b00ba1a1b1c09a05ff10c83bb7a86042e42988f2afc06", // len 114
        "32796a0a246ea67eb785eda2e045192b9d6e40b9fe2047b21ef0cee929039651", // len 115
        "da9ab8930992a9f65eccec4c310882cab428a708e6c899181046a8c73af00855", // len 116
        "9c94557382c966753c8cab0957eaedbe1d737b5fcb35c56c220ddd36f8a2d351", // len 117
        "d32ab00929cb935b79d44e74c5a745db460ff794dea3b79be40c1cc5cf5388ef", // len 118
        "da18797ed7c3a777f0847f429724a2d8cd5138e6ed2895c3fa1a6d39d18f7ec6", // len 119
        "f52b23db1fbb6ded89ef42a23ce0c8922c45f25c50b568a93bf1c075420bbb7c", // len 120
        "335a461692b30bba1d647cc71604e88e676c90e4c22455d0b8c83f4bd7c8ac9b", // len 121
        "3d08c4d7bdda7ec922b0741df357de46e7bd102f9ab7a5c67624ab58da6d9d75", // len 122
        "cc63be92e3a900cd067da89473b61b40579b54ef54f8305c2ffcc893743792e9", // len 123
        "865447fc4fae01471f2fc973bfb448de00217521ef02e3214d5177ea89c3ef31", // len 124
        "3daa582f9563601e290f3cd6d304bff7e25a9ee42a34ffbac5cf2bf40134e0d4", // len 125
        "5dda7cb7c2282a55676f8ad5c448092f4a9ebd65338b07ed224fcd7b6c73f5ef", // len 126
        "92ca0fa6651ee2f97b884b7246a562fa71250fedefe5ebf270d31c546bfea976", // len 127
        "471fb943aa23c511f6f72f8d1652d9c880cfa392ad80503120547703e56a2be5", // len 128
    ];

    /// Parses a known-answer hex digest into bytes without pulling in a test dependency.
    fn digest_from_hex(hex: &str) -> [u8; 32] {
        assert_eq!(hex.len(), 64);
        let mut bytes = [0; 32];
        let hex = hex.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            bytes[index] = (hex_digit(hex[index * 2]) << 4) | hex_digit(hex[index * 2 + 1]);
            index += 1;
        }
        bytes
    }

    fn hex_digit(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => panic!("invalid hex digit"),
        }
    }

    #[test]
    fn sha256_matches_standard_abc_known_answer() {
        assert_eq!(
            NcSha256::new().update(b"abc").finalize(),
            [
                0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
                0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
                0xf2, 0x00, 0x15, 0xad,
            ]
        );
    }

    #[test]
    fn sha256_matches_cross_checked_vectors_through_128_bytes() {
        let mut bytes = [0; 128];
        let mut index = 0;
        while index < bytes.len() {
            bytes[index] = index as u8;
            index += 1;
        }

        for (len, expected) in SHA256_PREFIX_LENGTH_VECTORS.iter().enumerate() {
            assert_eq!(
                NcSha256::new().update(&bytes[..len]).finalize(),
                digest_from_hex(expected),
                "SHA-256 mismatch for prefix length {len}",
            );
        }
    }

    #[test]
    fn typed_part_tags_match_the_draft_v2_grammar() {
        let iid = NcGuid::from_fields(0x1234_5678, 0x9abc, 0xdef0, [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0]);
        let nested_hash = NcAbiHash::from_bytes([0x5a; 32]);
        let hash = NcAbiHash::from_abi_parts(&[
            NcAbiHashPart::Interface { iid },
            NcAbiHashPart::Method {
                index: 7,
                name: "grammar",
            },
            NcAbiHashPart::Arg {
                index: 3,
                passing: NcAbiPassing::ComRef { mode: NcAbiThreadingMode::ThreadSafe },
                ty: NcAbiType::Interface { iid },
            },
            NcAbiHashPart::Return {
                passing: NcAbiPassing::Out,
                ty: NcAbiType::Aggregate(nested_hash),
            },
            NcAbiHashPart::Status,
            NcAbiHashPart::Layout { size: 24, align: 8 },
            NcAbiHashPart::Field {
                index: 2,
                offset: 16,
                ty: NcAbiType::Bool,
            },
            NcAbiHashPart::Variant {
                index: 4,
                tag: -2,
                ty: NcAbiType::U16,
            },
            NcAbiHashPart::Aggregate {
                kind: NcAbiAggregateKind::TaggedEnum,
            },
        ]);
        let actual = hash.bytes.iter().map(|byte| std::format!("{byte:02x}")).collect::<std::string::String>();
        assert_eq!(actual, "b1082d3722c7d86902132c1654cde80bd4ec8a2a6659412cb292c14cc939bb3d");
    }
}
