//! Deterministic SHA-256 shared by the IDL compiler and intrinsic array identities.
//!
//! Nominal declaration signatures remain tool-computed. Array metadata hashes
//! only an existing element identity and a literal length during const evaluation;
//! it never walks declarations or retains their schemas in runtime dispatch.

/// Initial SHA-256 chaining state defined by FIPS 180-4.
const INITIAL_STATE: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

/// Round constants defined by FIPS 180-4.
const ROUND_CONSTANTS: [u32; 64] = [
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

/// Mutable hash state with explicit ownership and no interior mutability.
struct Sha256 {
    /// Current eight-word chaining state.
    state: [u32; 8],
    /// Incomplete message block waiting for compression.
    block: [u8; 64],
    /// Number of meaningful bytes currently stored in `block`.
    block_len: usize,
    /// Total unpadded message length in bytes.
    byte_len: u64,
}

impl Sha256 {
    /// Creates an empty SHA-256 calculation.
    const fn new() -> Self {
        Self {
            state: INITIAL_STATE,
            block: [0; 64],
            block_len: 0,
            byte_len: 0,
        }
    }

    /// Appends bytes in their canonical encounter order.
    const fn update(&mut self, bytes: &[u8]) {
        let mut index = 0;
        while index < bytes.len() {
            self.block[self.block_len] = bytes[index];
            index += 1;
            self.block_len += 1;
            self.byte_len += 1;
            if self.block_len == self.block.len() {
                // A complete block cannot receive more input until its state
                // contribution has been compressed.
                self.compress();
                self.block_len = 0;
            }
        }
    }

    /// Pads the message and returns the final big-endian digest.
    const fn finish(mut self) -> [u8; 32] {
        let bit_len = self.byte_len.checked_mul(8).expect("IDL fingerprint input exceeds SHA-256 length capacity");
        self.update(&[0x80]);
        while self.block_len != 56 {
            self.update(&[0]);
        }
        self.update(&bit_len.to_be_bytes());

        let mut digest = [0; 32];
        let mut index = 0;
        while index < self.state.len() {
            let bytes = self.state[index].to_be_bytes();
            let mut byte = 0;
            while byte < bytes.len() {
                digest[index * 4 + byte] = bytes[byte];
                byte += 1;
            }
            index += 1;
        }
        digest
    }

    /// Compresses the current complete block into the chaining state.
    const fn compress(&mut self) {
        let mut schedule = [0u32; 64];
        let mut index = 0;
        while index < 16 {
            let offset = index * 4;
            schedule[index] = u32::from_be_bytes([self.block[offset], self.block[offset + 1], self.block[offset + 2], self.block[offset + 3]]);
            index += 1;
        }
        while index < schedule.len() {
            let s0 = schedule[index - 15].rotate_right(7) ^ schedule[index - 15].rotate_right(18) ^ (schedule[index - 15] >> 3);
            let s1 = schedule[index - 2].rotate_right(17) ^ schedule[index - 2].rotate_right(19) ^ (schedule[index - 2] >> 10);
            schedule[index] = schedule[index - 16].wrapping_add(s0).wrapping_add(schedule[index - 7]).wrapping_add(s1);
            index += 1;
        }

        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        let mut e = self.state[4];
        let mut f = self.state[5];
        let mut g = self.state[6];
        let mut h = self.state[7];
        index = 0;
        while index < schedule.len() {
            let constant = ROUND_CONSTANTS[index];
            let word = schedule[index];
            index += 1;
            let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let temporary1 = h.wrapping_add(sum1).wrapping_add(choose).wrapping_add(constant).wrapping_add(word);
            let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temporary2 = sum0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temporary1);
            d = c;
            c = b;
            b = a;
            a = temporary1.wrapping_add(temporary2);
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
}

/// Calculates SHA-256 over one complete byte sequence.
#[doc(hidden)]
pub const fn __nc_abi_digest(bytes: &[u8]) -> [u8; 32] {
    let mut state = Sha256::new();
    state.update(bytes);
    state.finish()
}

#[cfg(test)]
mod tests {
    use super::__nc_abi_digest as digest;

    /// Checks the implementation against standard short-message vectors.
    #[test]
    fn matches_known_answers() {
        assert_eq!(
            digest(b""),
            [
                0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c,
                0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
            ]
        );
        assert_eq!(
            digest(b"abc"),
            [
                0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c,
                0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00, 0x15, 0xad,
            ]
        );
    }
}
