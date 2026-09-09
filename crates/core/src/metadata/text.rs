//! Shared policy for qualified names and version text stored in descriptors.
//!
//! Validation is ASCII-only and const-evaluable so macros, manually authored
//! descriptors, runtime validation, and tooling apply the same length and
//! character rules. Error enums retain the exact offending byte or structural
//! condition for compile-time and runtime diagnostics.

use core::fmt;

/// Maximum UTF-8 byte length of a plugin, class, or interface qualified name.
pub const NC_DESCRIPTOR_QUALIFIED_NAME_MAX_LEN: usize = 1024;

/// Maximum UTF-8 byte length of plugin or class version text.
pub const NC_DESCRIPTOR_VERSION_MAX_LEN: usize = 128;

/// Reason a descriptor qualified name violates the ABI text policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NcQualifiedNameError {
    /// The qualified name is empty.
    Empty,
    /// The qualified name exceeds [`NC_DESCRIPTOR_QUALIFIED_NAME_MAX_LEN`].
    TooLong {
        /// Actual length of the qualified name in UTF-8 bytes.
        len: usize,
    },
    /// A segment does not start with an ASCII letter or underscore.
    InvalidSegmentStart {
        /// Byte index at which the invalid segment starts.
        index: usize,
    },
    /// A segment contains a byte outside the permitted ASCII character set.
    InvalidCharacter {
        /// Byte index of the invalid character.
        index: usize,
    },
    /// The qualified name ends with a `.` or `:` separator.
    TrailingSeparator {
        /// Byte index of the trailing separator.
        index: usize,
    },
}

impl fmt::Display for NcQualifiedNameError {
    /// Formats the policy violation as a stable human-readable diagnostic.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("qualified name must not be empty"),
            Self::TooLong { len } => write!(
                formatter,
                "qualified name is {len} bytes; the maximum is {NC_DESCRIPTOR_QUALIFIED_NAME_MAX_LEN}"
            ),
            Self::InvalidSegmentStart { index } => write!(
                formatter,
                "qualified-name segment at byte {index} must start with an ASCII letter or `_`"
            ),
            Self::InvalidCharacter { index } => write!(
                formatter,
                "qualified name contains a disallowed character at byte {index}"
            ),
            Self::TrailingSeparator { index } => write!(
                formatter,
                "qualified name ends with a separator at byte {index}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NcQualifiedNameError {}

/// Reason descriptor version text violates the ABI text policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NcVersionTextError {
    /// The version text is empty.
    Empty,
    /// The version text exceeds [`NC_DESCRIPTOR_VERSION_MAX_LEN`].
    TooLong {
        /// Actual length of the version text in UTF-8 bytes.
        len: usize,
    },
    /// The version contains a byte that is not printable, non-whitespace ASCII.
    InvalidCharacter {
        /// Byte index of the invalid character.
        index: usize,
    },
}

impl fmt::Display for NcVersionTextError {
    /// Formats the policy violation as a stable human-readable diagnostic.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("version text must not be empty"),
            Self::TooLong { len } => write!(
                formatter,
                "version text is {len} bytes; the maximum is {NC_DESCRIPTOR_VERSION_MAX_LEN}"
            ),
            Self::InvalidCharacter { index } => write!(
                formatter,
                "version text contains a non-graphic ASCII byte at byte {index}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NcVersionTextError {}

/// Validates a plugin, class, or interface qualified name.
///
/// A valid name contains `1..=1024` ASCII bytes split into nonempty segments
/// by `.` or `:`. Each segment starts with an ASCII letter or `_`; subsequent
/// bytes may additionally contain digits or `-`.
///
/// This function is allocation-free and may be evaluated in a const context.
pub const fn validate_qualified_name(value: &str) -> Result<(), NcQualifiedNameError> {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return Err(NcQualifiedNameError::Empty);
    }
    if bytes.len() > NC_DESCRIPTOR_QUALIFIED_NAME_MAX_LEN {
        return Err(NcQualifiedNameError::TooLong { len: bytes.len() });
    }

    let mut index = 0;
    let mut expect_segment_start = true;
    while index < bytes.len() {
        let byte = bytes[index];
        if expect_segment_start {
            if !is_name_start(byte) {
                return Err(NcQualifiedNameError::InvalidSegmentStart { index });
            }
            expect_segment_start = false;
        } else if byte == b'.' || byte == b':' {
            expect_segment_start = true;
        } else if !is_name_continue(byte) {
            return Err(NcQualifiedNameError::InvalidCharacter { index });
        }
        index += 1;
    }

    if expect_segment_start {
        return Err(NcQualifiedNameError::TrailingSeparator {
            index: bytes.len() - 1,
        });
    }
    Ok(())
}

/// Validates plugin or class version text.
///
/// Valid version text contains `1..=128` ASCII graphic bytes (`!` through
/// `~`). Spaces, control bytes, and non-ASCII characters are rejected.
///
/// This function is allocation-free and may be evaluated in a const context.
pub const fn validate_version_text(value: &str) -> Result<(), NcVersionTextError> {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return Err(NcVersionTextError::Empty);
    }
    if bytes.len() > NC_DESCRIPTOR_VERSION_MAX_LEN {
        return Err(NcVersionTextError::TooLong { len: bytes.len() });
    }

    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte < b'!' || byte > b'~' {
            return Err(NcVersionTextError::InvalidCharacter { index });
        }
        index += 1;
    }
    Ok(())
}

/// Returns whether an ASCII byte may begin a qualified-name segment.
const fn is_name_start(byte: u8) -> bool {
    (byte >= b'A' && byte <= b'Z') || (byte >= b'a' && byte <= b'z') || byte == b'_'
}

/// Returns whether an ASCII byte may continue a qualified-name segment.
const fn is_name_continue(byte: u8) -> bool {
    is_name_start(byte) || (byte >= b'0' && byte <= b'9') || byte == b'-'
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::string::ToString;

    const VALID_NAME: Result<(), NcQualifiedNameError> =
        validate_qualified_name("example.image.Decoder:v1");
    const VALID_VERSION: Result<(), NcVersionTextError> = validate_version_text("1.2.3-alpha+4");

    #[test]
    fn validators_are_const_evaluable() {
        assert_eq!(VALID_NAME, Ok(()));
        assert_eq!(VALID_VERSION, Ok(()));
    }

    #[test]
    fn qualified_name_policy_reports_precise_failures() {
        assert_eq!(
            validate_qualified_name(""),
            Err(NcQualifiedNameError::Empty)
        );
        assert_eq!(
            validate_qualified_name(".example"),
            Err(NcQualifiedNameError::InvalidSegmentStart { index: 0 })
        );
        assert_eq!(
            validate_qualified_name("example::Decoder"),
            Err(NcQualifiedNameError::InvalidSegmentStart { index: 8 })
        );
        assert_eq!(
            validate_qualified_name("example.Decoder:"),
            Err(NcQualifiedNameError::TrailingSeparator { index: 15 })
        );
        assert_eq!(
            validate_qualified_name("example.Décoder"),
            Err(NcQualifiedNameError::InvalidCharacter { index: 9 })
        );

        let maximum = "a".repeat(NC_DESCRIPTOR_QUALIFIED_NAME_MAX_LEN);
        assert_eq!(validate_qualified_name(&maximum), Ok(()));
        let overlong = "a".repeat(NC_DESCRIPTOR_QUALIFIED_NAME_MAX_LEN + 1);
        assert_eq!(
            validate_qualified_name(&overlong),
            Err(NcQualifiedNameError::TooLong {
                len: NC_DESCRIPTOR_QUALIFIED_NAME_MAX_LEN + 1,
            })
        );
    }

    #[test]
    fn version_text_policy_reports_precise_failures() {
        assert_eq!(validate_version_text(""), Err(NcVersionTextError::Empty));
        assert_eq!(
            validate_version_text("1.0 beta"),
            Err(NcVersionTextError::InvalidCharacter { index: 3 })
        );
        assert_eq!(
            validate_version_text("1.β"),
            Err(NcVersionTextError::InvalidCharacter { index: 2 })
        );

        let maximum = "v".repeat(NC_DESCRIPTOR_VERSION_MAX_LEN);
        assert_eq!(validate_version_text(&maximum), Ok(()));
        let overlong = "v".repeat(NC_DESCRIPTOR_VERSION_MAX_LEN + 1);
        assert_eq!(
            validate_version_text(&overlong),
            Err(NcVersionTextError::TooLong {
                len: NC_DESCRIPTOR_VERSION_MAX_LEN + 1,
            })
        );

        assert_eq!(
            NcVersionTextError::InvalidCharacter { index: 4 }.to_string(),
            "version text contains a non-graphic ASCII byte at byte 4"
        );
    }
}
