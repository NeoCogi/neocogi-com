//! COM-owned grammar for advertised Rust binding locations.
//!
//! This is a bounded Rust-2024 path subset, not Rust type resolution. IDL tools
//! and catalog readers share this policy; compiler frontends may still parse a
//! validated path into tokens. Keeping the checker here avoids linking a Rust
//! parser into plugins that only publish or consume metadata.
use core::fmt;

/// Maximum bytes in a Rust binding path, including separators and raw prefixes.
pub const NC_DESCRIPTOR_RUST_PATH_MAX_LEN: usize = 1024;

/// Failure to spell a supported, externally rooted Rust binding path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NcRustPathError {
    /// No crate or declaration name was supplied.
    Empty,
    /// The metadata path exceeds the bounded descriptor policy.
    TooLong {
        /// Actual UTF-8 byte length.
        len: usize,
    },
    /// A segment is empty, starts incorrectly, or consists only of `_`.
    InvalidIdentifier {
        /// Byte offset of the segment, including its possible `r#` prefix.
        index: usize,
    },
    /// An unescaped keyword or a forbidden relative identifier occurs here.
    ReservedIdentifier {
        /// Byte offset of the segment, including its possible `r#` prefix.
        index: usize,
    },
    /// Only `::` may separate segments; other punctuation or whitespace is invalid.
    InvalidSeparator {
        /// Byte offset at which `::` was expected.
        index: usize,
    },
}

impl fmt::Display for NcRustPathError {
    /// Explains the policy violation; callers attach the source or manifest field.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("Rust binding path must not be empty"),
            Self::TooLong { len } => write!(formatter, "Rust binding path is {len} bytes; the maximum is {NC_DESCRIPTOR_RUST_PATH_MAX_LEN}"),
            Self::InvalidIdentifier { index } => write!(formatter, "invalid ASCII Rust identifier at byte {index}"),
            Self::ReservedIdentifier { index } => write!(formatter, "reserved Rust identifier at byte {index}"),
            Self::InvalidSeparator { index } => write!(formatter, "expected a Rust path separator `::` at byte {index}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NcRustPathError {}

/// Validates the common syntax for a Rust module location or concrete type path.
///
/// Accepts `crate_name::Module::Type`, optional leading `::`, and `r#` identifiers.
/// Segments contain ASCII letters, digits and underscores, with a nondigit start.
/// `_` alone, Rust-2024 strict/reserved keywords without `r#`, and `self`, `Self`,
/// `super`, or `crate` even with `r#` are rejected. Weak keywords such as `union`
/// remain ordinary identifiers. Whitespace, comments, Unicode, generic arguments,
/// qualified-self paths, lifetimes and expressions are outside this grammar.
///
/// This allocation-free, const-evaluable check preserves spelling. A compiler
/// consumer may normalize the optional leading `::` when building its tokens.
/// A successful check does not establish crate availability, visibility, type
/// existence, trait implementations, or agreement with a COM identity.
pub const fn validate_rust_path(value: &str) -> Result<(), NcRustPathError> {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return Err(NcRustPathError::Empty);
    }
    if bytes.len() > NC_DESCRIPTOR_RUST_PATH_MAX_LEN {
        return Err(NcRustPathError::TooLong { len: bytes.len() });
    }
    let mut index = if bytes.len() >= 2 && bytes[0] == b':' && bytes[1] == b':' { 2 } else { 0 };
    loop {
        let segment = index;
        let raw = index + 1 < bytes.len() && bytes[index] == b'r' && bytes[index + 1] == b'#';
        if raw {
            index += 2;
        }
        let start = index;
        if index == bytes.len() || !identifier_start(bytes[index]) {
            return Err(NcRustPathError::InvalidIdentifier { index: segment });
        }
        index += 1;
        while index < bytes.len() && (identifier_start(bytes[index]) || (bytes[index] >= b'0' && bytes[index] <= b'9')) {
            index += 1;
        }
        if index == start + 1 && bytes[start] == b'_' {
            return Err(NcRustPathError::InvalidIdentifier { index: segment });
        }
        // Rust forbids these raw spellings too; permitting relative module
        // anchors would make producer and consuming application scopes disagree.
        if word(bytes, start, index, "self") || word(bytes, start, index, "Self") || word(bytes, start, index, "super") || word(bytes, start, index, "crate") {
            return Err(NcRustPathError::ReservedIdentifier { index: segment });
        }
        if !raw {
            let mut keyword = 0;
            while keyword < RESERVED.len() {
                if word(bytes, start, index, RESERVED[keyword]) {
                    return Err(NcRustPathError::ReservedIdentifier { index: segment });
                }
                keyword += 1;
            }
        }
        if index == bytes.len() {
            return Ok(());
        }
        match bytes[index] {
            b':' if index + 1 < bytes.len() && bytes[index + 1] == b':' => index += 2,
            _ => return Err(NcRustPathError::InvalidSeparator { index }),
        }
    }
}

/// ASCII identifier start; subsequent bytes additionally permit decimal digits.
const fn identifier_start(byte: u8) -> bool {
    byte == b'_' || (byte >= b'a' && byte <= b'z') || (byte >= b'A' && byte <= b'Z')
}

/// Compares a segment without slicing strings or allocating during const evaluation.
const fn word(bytes: &[u8], start: usize, end: usize, expected: &str) -> bool {
    let expected = expected.as_bytes();
    if end - start != expected.len() {
        return false;
    }
    let mut index = 0;
    while index < expected.len() {
        if bytes[start + index] != expected[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// Rust-2024 strict/reserved words other than the four module/type anchors above.
/// One authoritative table serves generated metadata and foreign catalog reads.
const RESERVED: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "dyn", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match",
    "mod", "move", "mut", "pub", "ref", "return", "static", "struct", "trait", "true", "type", "unsafe", "use", "where", "while", "abstract", "become", "box",
    "do", "final", "gen", "macro", "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
];

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::format;

    /// Compile-time producers use precisely the same checker as runtime readers.
    const RAW_PATH: Result<(), NcRustPathError> = validate_rust_path("::custom_contracts::r#type::CounterEvent");

    /// Ordinary, raw, weak-keyword and crate-only locations share one grammar.
    #[test]
    fn accepted_paths_preserve_supported_identifier_forms() {
        assert_eq!(RAW_PATH, Ok(()));
        for path in [
            "api",
            "api::_private::Type2",
            "api::union::safe::raw::macro_rules",
            "api::r#ordinary::Thing",
            "::api::r#gen::Type",
        ] {
            assert_eq!(validate_rust_path(path), Ok(()), "{path}");
        }
    }

    /// Every reserved word is rejected unescaped and accepted with a legal raw prefix.
    #[test]
    fn keyword_policy_is_shared_for_every_segment() {
        for keyword in RESERVED {
            assert_eq!(
                validate_rust_path(&format!("api::{keyword}::Type")),
                Err(NcRustPathError::ReservedIdentifier { index: 5 })
            );
            assert!(validate_rust_path(&format!("api::r#{keyword}::Type")).is_ok(), "{keyword}");
        }
        for anchor in ["self", "Self", "super", "crate"] {
            for path in [format!("{anchor}::Type"), format!("api::{anchor}"), format!("api::r#{anchor}")] {
                assert!(matches!(validate_rust_path(&path), Err(NcRustPathError::ReservedIdentifier { .. })), "{path}");
            }
        }
    }

    /// Unsupported syntax cannot be accepted by skipping punctuation or empty segments.
    #[test]
    fn malformed_paths_report_the_offending_byte() {
        assert_eq!(validate_rust_path(""), Err(NcRustPathError::Empty));
        assert_eq!(validate_rust_path("api::"), Err(NcRustPathError::InvalidIdentifier { index: 5 }));
        assert_eq!(validate_rust_path("api::type"), Err(NcRustPathError::ReservedIdentifier { index: 5 }));
        assert_eq!(validate_rust_path("api::Type<u8>"), Err(NcRustPathError::InvalidSeparator { index: 9 }));
        for path in [
            "::",
            ":api",
            "api:",
            "api:::Type",
            "api::::Type",
            "_",
            "api::_",
            "api::r#_",
            "api::r#",
            "9api::T",
            "api::9T",
            "api::r#9T",
            "api::Ty pe",
            "api::Type\n",
            "api /*comment*/::Type",
            "api::Type;panic!()",
            "api::f()",
            "api::T<'a>",
            "api::<T>",
            "<T as api::Trait>::Item",
            "api::T-Name",
            "api::Café",
            " api::T",
            "api::r##type",
        ] {
            assert!(validate_rust_path(path).is_err(), "{path}");
        }
        assert!(validate_rust_path(&"a".repeat(NC_DESCRIPTOR_RUST_PATH_MAX_LEN)).is_ok());
        assert_eq!(
            validate_rust_path(&"a".repeat(NC_DESCRIPTOR_RUST_PATH_MAX_LEN + 1)),
            Err(NcRustPathError::TooLong { len: NC_DESCRIPTOR_RUST_PATH_MAX_LEN + 1 })
        );
    }
}
