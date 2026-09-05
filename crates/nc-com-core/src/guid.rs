//! Stable GUID representation, qualified-name derivation, formatting, and parsing.
//!
//! nc-com identities use canonical UUID byte order and deterministic UUID-v5
//! derivation from validated qualified-name bytes. The module keeps the hashing
//! implementation const-evaluable so plugin, class, and interface IDs can be
//! emitted directly into static descriptor data.

/// A plugin, class, or COM interface identifier.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NcGuid {
    /// First 32 bits of the identifier.
    pub data1: u32,
    /// Next 16 bits of the identifier.
    pub data2: u16,
    /// Next 16 bits of the identifier.
    pub data3: u16,
    /// Final 64 bits of the identifier.
    pub data4: [u8; 8],
}

/// The all-zero GUID used as a sentinel in ABI metadata.
pub const NC_GUID_NIL: NcGuid = NcGuid::from_fields(0, 0, 0, [0; 8]);

/// UUID namespace reserved for `nc-com` ABI identities.
///
/// User-defined plugin, class, and interface identifiers are UUIDv5 values
/// derived from this namespace and their exact qualified-name UTF-8 bytes.
pub const NC_COM_GUID_NAMESPACE: NcGuid = NcGuid::from_uuid_bytes([
    0x72, 0x0d, 0x6b, 0x32, 0x1f, 0x75, 0x48, 0x28, 0xa8, 0x95, 0x6f, 0xb7, 0x3f, 0x82, 0x02, 0xee,
]);

/// Derives the UUIDv5 ABI identity for an exact qualified name.
///
/// The derivation hashes the canonical bytes of [`NC_COM_GUID_NAMESPACE`]
/// followed by `qualified_name.as_bytes()` with SHA-1, then applies the RFC
/// 4122 version-5 and variant bits. No Unicode normalization, case folding,
/// trimming, or Rust-path rewriting is performed. Consequently, changing any
/// byte of the qualified name changes the identity.
///
/// This function is `const`, allocation-free, and available without `std` so
/// handwritten descriptor producers can use the same authoritative operation
/// as the proc macros and runtime validator.
pub const fn guid_from_qualified_name(qualified_name: &str) -> NcGuid {
    let namespace = NC_COM_GUID_NAMESPACE.to_uuid_bytes();
    let name = qualified_name.as_bytes();
    let message_len = namespace.len() + name.len();
    let padded_len = (message_len + 9).div_ceil(64) * 64;
    let bit_len = (message_len as u64) * 8;

    let mut state = [
        0x6745_2301u32,
        0xefcd_ab89u32,
        0x98ba_dcfeu32,
        0x1032_5476u32,
        0xc3d2_e1f0u32,
    ];
    let mut block_start = 0;
    while block_start < padded_len {
        let mut block = [0u8; 64];
        let mut offset = 0;
        while offset < block.len() {
            let index = block_start + offset;
            block[offset] = if index < namespace.len() {
                namespace[index]
            } else if index < message_len {
                name[index - namespace.len()]
            } else if index == message_len {
                0x80
            } else if index >= padded_len - 8 {
                bit_len.to_be_bytes()[index - (padded_len - 8)]
            } else {
                0
            };
            offset += 1;
        }
        sha1_compress(&mut state, &block);
        block_start += 64;
    }

    let mut digest = [0u8; 16];
    let mut word_index = 0;
    while word_index < 4 {
        let word = state[word_index].to_be_bytes();
        let start = word_index * 4;
        digest[start] = word[0];
        digest[start + 1] = word[1];
        digest[start + 2] = word[2];
        digest[start + 3] = word[3];
        word_index += 1;
    }
    digest[6] = (digest[6] & 0x0f) | 0x50;
    digest[8] = (digest[8] & 0x3f) | 0x80;
    NcGuid::from_uuid_bytes(digest)
}

/// Applies one SHA-1 compression block for const UUID-v5 derivation.
const fn sha1_compress(state: &mut [u32; 5], block: &[u8; 64]) {
    let mut words = [0u32; 80];
    let mut index = 0;
    while index < 16 {
        let start = index * 4;
        words[index] = u32::from_be_bytes([
            block[start],
            block[start + 1],
            block[start + 2],
            block[start + 3],
        ]);
        index += 1;
    }
    while index < 80 {
        words[index] =
            (words[index - 3] ^ words[index - 8] ^ words[index - 14] ^ words[index - 16])
                .rotate_left(1);
        index += 1;
    }

    let mut a = state[0];
    let mut b = state[1];
    let mut c = state[2];
    let mut d = state[3];
    let mut e = state[4];
    index = 0;
    while index < 80 {
        let (f, k) = if index < 20 {
            ((b & c) | ((!b) & d), 0x5a82_7999)
        } else if index < 40 {
            (b ^ c ^ d, 0x6ed9_eba1)
        } else if index < 60 {
            ((b & c) | (b & d) | (c & d), 0x8f1b_bcdc)
        } else {
            (b ^ c ^ d, 0xca62_c1d6)
        };
        let next = a
            .rotate_left(5)
            .wrapping_add(f)
            .wrapping_add(e)
            .wrapping_add(k)
            .wrapping_add(words[index]);
        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = next;
        index += 1;
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
}

impl NcGuid {
    /// Returns the all-zero GUID.
    pub const fn nil() -> Self {
        NC_GUID_NIL
    }

    /// Returns whether this GUID is the all-zero sentinel value.
    pub const fn is_nil(self) -> bool {
        self.data1 == 0
            && self.data2 == 0
            && self.data3 == 0
            && self.data4[0] == 0
            && self.data4[1] == 0
            && self.data4[2] == 0
            && self.data4[3] == 0
            && self.data4[4] == 0
            && self.data4[5] == 0
            && self.data4[6] == 0
            && self.data4[7] == 0
    }

    /// Creates a GUID from its C-compatible field representation.
    pub const fn from_fields(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Self {
        Self {
            data1,
            data2,
            data3,
            data4,
        }
    }

    /// Returns this GUID's C-compatible field representation.
    pub const fn fields(self) -> (u32, u16, u16, [u8; 8]) {
        (self.data1, self.data2, self.data3, self.data4)
    }

    /// Creates a GUID from canonical UUID bytes.
    pub const fn from_uuid_bytes(bytes: [u8; 16]) -> Self {
        Self {
            data1: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            data2: u16::from_be_bytes([bytes[4], bytes[5]]),
            data3: u16::from_be_bytes([bytes[6], bytes[7]]),
            data4: [
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15],
            ],
        }
    }

    /// Returns canonical UUID bytes.
    pub const fn to_uuid_bytes(self) -> [u8; 16] {
        let data1 = self.data1.to_be_bytes();
        let data2 = self.data2.to_be_bytes();
        let data3 = self.data3.to_be_bytes();
        [
            data1[0],
            data1[1],
            data1[2],
            data1[3],
            data2[0],
            data2[1],
            data3[0],
            data3[1],
            self.data4[0],
            self.data4[1],
            self.data4[2],
            self.data4[3],
            self.data4[4],
            self.data4[5],
            self.data4[6],
            self.data4[7],
        ]
    }
}

impl core::fmt::Display for NcGuid {
    /// Writes the canonical lowercase, hyphenated UUID representation.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            self.data1,
            self.data2,
            self.data3,
            self.data4[0],
            self.data4[1],
            self.data4[2],
            self.data4[3],
            self.data4[4],
            self.data4[5],
            self.data4[6],
            self.data4[7],
        )
    }
}

impl core::str::FromStr for NcGuid {
    type Err = ParseGuidError;

    /// Parses the exact canonical UUID shape accepted by nc-com metadata.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_nc_guid(value)
    }
}

/// Error returned when parsing a canonical GUID string fails.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseGuidError {
    /// The input length was not the canonical UUID length of 36 bytes.
    InvalidLength {
        /// Actual input length in bytes.
        len: usize,
    },
    /// A required hyphen was missing at `index`.
    InvalidHyphen {
        /// Zero-based byte position of the expected hyphen.
        index: usize,
    },
    /// A non-hexadecimal byte was found at `index`.
    InvalidHex {
        /// Zero-based byte position of the invalid digit.
        index: usize,
    },
}

impl core::fmt::Display for ParseGuidError {
    /// Describes the precise canonical-format rule that parsing violated.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::InvalidLength { len } => {
                write!(f, "invalid GUID length {len}; expected 36 bytes")
            }
            Self::InvalidHyphen { index } => {
                write!(f, "invalid GUID hyphen at byte index {index}")
            }
            Self::InvalidHex { index } => {
                write!(f, "invalid GUID hex digit at byte index {index}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseGuidError {}

/// Parses and validates all five groups of a canonical GUID string.
fn parse_nc_guid(value: &str) -> Result<NcGuid, ParseGuidError> {
    let bytes = value.as_bytes();
    if bytes.len() != 36 {
        return Err(ParseGuidError::InvalidLength { len: bytes.len() });
    }

    for index in [8, 13, 18, 23] {
        if bytes[index] != b'-' {
            return Err(ParseGuidError::InvalidHyphen { index });
        }
    }

    Ok(NcGuid {
        data1: parse_hex_u32(bytes, 0, 8)?,
        data2: parse_hex_u16(bytes, 9, 13)?,
        data3: parse_hex_u16(bytes, 14, 18)?,
        data4: [
            parse_hex_byte(bytes, 19)?,
            parse_hex_byte(bytes, 21)?,
            parse_hex_byte(bytes, 24)?,
            parse_hex_byte(bytes, 26)?,
            parse_hex_byte(bytes, 28)?,
            parse_hex_byte(bytes, 30)?,
            parse_hex_byte(bytes, 32)?,
            parse_hex_byte(bytes, 34)?,
        ],
    })
}

/// Parses one bounded hexadecimal group into a `u32`.
fn parse_hex_u32(bytes: &[u8], start: usize, end: usize) -> Result<u32, ParseGuidError> {
    let mut value = 0;
    for (index, byte) in bytes.iter().copied().enumerate().take(end).skip(start) {
        value = (value << 4) | u32::from(parse_hex_nibble(byte, index)?);
    }
    Ok(value)
}

/// Parses one bounded hexadecimal group into a `u16`.
fn parse_hex_u16(bytes: &[u8], start: usize, end: usize) -> Result<u16, ParseGuidError> {
    let mut value = 0;
    for (index, byte) in bytes.iter().copied().enumerate().take(end).skip(start) {
        value = (value << 4) | u16::from(parse_hex_nibble(byte, index)?);
    }
    Ok(value)
}

/// Parses two adjacent hexadecimal digits into one byte.
fn parse_hex_byte(bytes: &[u8], start: usize) -> Result<u8, ParseGuidError> {
    Ok((parse_hex_nibble(bytes[start], start)? << 4)
        | parse_hex_nibble(bytes[start + 1], start + 1)?)
}

/// Converts one ASCII hexadecimal digit while preserving its source index.
fn parse_hex_nibble(byte: u8, index: usize) -> Result<u8, ParseGuidError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(ParseGuidError::InvalidHex { index }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::string::ToString;

    #[test]
    fn nc_com_uuid_v5_golden_vectors_are_stable() {
        assert_eq!(
            guid_from_qualified_name("nc.tests.spec.Interface:v1").to_string(),
            "181a0cda-93b8-57de-96e8-f0bfeca9d583"
        );
        assert_eq!(
            guid_from_qualified_name("nc.tests.spec.Class:class:v1").to_string(),
            "ab2de1f2-cfee-5ac2-b0f4-9c3e6da20f5c"
        );
        assert_eq!(
            guid_from_qualified_name("nc.tests.spec.Plugin:v1").to_string(),
            "d3105b2d-858f-5b6c-a2cb-be35b0e4dde9"
        );
        assert_eq!(
            guid_from_qualified_name(
                "nc.tests.LongSegmentLongSegmentLongSegmentLongSegmentLongSegmentLongSegmentLongSegmentLongSegment:v1"
            )
            .to_string(),
            "ef6cbe80-1ed8-5ed5-bc3d-942a3f998eae"
        );
    }

    #[test]
    fn derivation_is_const_and_uses_exact_utf8_bytes() {
        const LOWER: NcGuid = guid_from_qualified_name("nc.tests.Name:v1");
        const UPPER: NcGuid = guid_from_qualified_name("nc.tests.name:v1");
        assert_ne!(LOWER, UPPER);
    }
}
