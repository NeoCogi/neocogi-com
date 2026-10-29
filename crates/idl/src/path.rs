//! Logical module names derived from portable relative source paths.

use std::{
    fmt,
    path::{Component, Path},
};

/// A validated, byte-preserving dotted IDL module name such as `neocogi.ui`.
/// Source identities and authored references share the same nonraw segment rule;
/// Rust binding metadata has its separate COM-owned path grammar.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModuleName {
    /// Validated source-path segments in declaration order.
    segments: Vec<String>,
}

impl ModuleName {
    /// Derives a module name from one relative directory containing IDL parts.
    ///
    /// Unlike [`Self::from_relative_source`], every path component contributes
    /// its complete name because no filename carries module identity.
    pub fn from_relative_directory(path: &Path) -> Result<Self, ModulePathError> {
        Self::from_relative_path(path, false)
    }

    /// Derives a module name from one `.ncidl` path relative to an IDL root.
    ///
    /// Filesystem separators become dots and the final `.ncidl` extension is
    /// removed. Absolute paths, parent traversal, non-UTF-8 names, and segments
    /// that are not portable identifiers are rejected.
    pub fn from_relative_source(path: &Path) -> Result<Self, ModulePathError> {
        Self::from_relative_path(path, true)
    }

    /// Validates either a source-file path or a module-directory path.
    fn from_relative_path(path: &Path, strip_extension: bool) -> Result<Self, ModulePathError> {
        if path.is_absolute() {
            return Err(ModulePathError::Absolute);
        }
        if strip_extension && path.extension().and_then(|extension| extension.to_str()) != Some("ncidl") {
            return Err(ModulePathError::Extension);
        }

        let mut segments = Vec::new();
        let component_count = path.components().count();
        for (index, component) in path.components().enumerate() {
            let Component::Normal(component) = component else {
                // Relative path prefixes such as `.` and `..` would make an
                // identity dependent on caller normalization, so reject them.
                return Err(ModulePathError::NonCanonical);
            };
            let text = component.to_str().ok_or(ModulePathError::NonUtf8)?;
            let segment = if strip_extension && index + 1 == component_count {
                text.strip_suffix(".ncidl").ok_or(ModulePathError::Extension)?
            } else {
                text
            };
            Self::validate_segment(segment)?;
            segments.push(segment.to_owned());
        }

        if segments.is_empty() {
            return Err(ModulePathError::Empty);
        }
        Ok(Self { segments })
    }

    /// Validate one logical segment without imposing a Rust metadata path bound.
    /// ASCII/raw/underscore admission belongs here; only the existing typed Rust
    /// keyword classification is shared with COM, as for authored IDL members.
    pub(crate) fn validate_segment(segment: &str) -> Result<(), ModulePathError> {
        if segment == "_"
            || !is_identifier(segment)
            || matches!(
                neocogi_com_core::validate_rust_path(segment),
                Err(neocogi_com_core::NcRustPathError::ReservedIdentifier { .. })
            )
        {
            return Err(ModulePathError::InvalidSegment(segment.to_owned()));
        }
        Ok(())
    }

    /// Validate logical registration keys through the same source segment rule.
    /// Callers keep their context's arity and transported metadata length bounds.
    pub(crate) fn validate_dotted(name: &str) -> Result<(), ModulePathError> {
        for segment in name.split('.') {
            Self::validate_segment(segment)?;
        }
        Ok(())
    }

    /// Returns the validated path segments without exposing mutable identity state.
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// Qualifies an exact declaration identifier within this module.
    pub fn qualify(&self, declaration: &str) -> String {
        // Capacity includes the separating dot so qualification performs one
        // allocation without hiding any name rewriting.
        let mut qualified = String::with_capacity(self.to_string().len() + declaration.len() + 1);
        qualified.push_str(&self.to_string());
        qualified.push('.');
        qualified.push_str(declaration);
        qualified
    }
}

impl fmt::Display for ModuleName {
    /// Writes the canonical dotted module name.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Joining validated segments is the sole textual normalization from a
        // source path into a module name.
        formatter.write_str(&self.segments.join("."))
    }
}

/// Failure to derive a stable module identity from a source path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModulePathError {
    /// The caller supplied an absolute path instead of a path below the IDL root.
    Absolute,
    /// The path was empty and therefore had no module identity.
    Empty,
    /// The final path component did not use the `.ncidl` extension.
    Extension,
    /// A source-path component was not valid UTF-8.
    NonUtf8,
    /// The path contained `.` or `..` and was not already normalized.
    NonCanonical,
    /// One component violated the shared nonraw logical module segment rule.
    InvalidSegment(
        /// The exact invalid component, retained for a useful diagnostic.
        String,
    ),
}

impl fmt::Display for ModulePathError {
    /// Describes the violated module-path rule.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absolute => formatter.write_str("IDL source path must be relative to its configured root"),
            Self::Empty => formatter.write_str("IDL source path must identify one module"),
            Self::Extension => formatter.write_str("IDL source path must end in .ncidl"),
            Self::NonUtf8 => formatter.write_str("IDL source path must be valid UTF-8"),
            Self::NonCanonical => formatter.write_str("IDL source path must not contain . or .. components"),
            Self::InvalidSegment(segment) => write!(
                formatter,
                "IDL module segment {segment:?} must be a nonraw ASCII identifier other than `_` or a Rust 2024 keyword"
            ),
        }
    }
}

impl std::error::Error for ModulePathError {}

/// Recognizes ASCII segment shape; ModuleName owns keyword/underscore admission.
fn is_identifier(segment: &str) -> bool {
    let mut bytes = segment.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !(first == b'_' || first.is_ascii_alphabetic()) {
        return false;
    }
    // IDL module paths intentionally avoid raw identifiers and Unicode
    // normalization so their byte representation is portable and predictable.
    bytes.all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::{ModuleName, ModulePathError};
    use std::path::Path;

    /// Confirms path normalization preserves exact identifier spelling.
    #[test]
    fn derives_dotted_module_name() {
        let module = ModuleName::from_relative_source(Path::new("NeoCogi/ui_core.ncidl")).unwrap();
        assert_eq!(module.to_string(), "NeoCogi.ui_core");
        assert_eq!(module.qualify("IWidget"), "NeoCogi.ui_core.IWidget");
    }

    /// Confirms unstable traversal never enters an identity preimage.
    #[test]
    fn rejects_noncanonical_paths() {
        assert_eq!(
            ModuleName::from_relative_source(Path::new("neocogi/../ui.ncidl")),
            Err(ModulePathError::NonCanonical)
        );
    }

    /// All identity constructors and logical keys reject the same unsupported words.
    #[test]
    fn logical_segments_share_source_and_registration_admission() {
        for invalid in [
            "type",
            "async",
            "gen",
            "_",
            "self",
            "Self",
            "super",
            "crate",
            "r#type",
            "r#ordinary",
            "Café",
            "9name",
            "",
        ] {
            for name in [format!("{invalid}.owner"), format!("proof.{invalid}"), format!("proof.{invalid}.owner")] {
                assert!(ModuleName::validate_dotted(&name).is_err(), "{name}");
                // Empty filesystem components normalize away; dotted keys still
                // reject them. The constructors separately own canonical paths.
                if invalid.is_empty() {
                    continue;
                }
                let directory = name.replace('.', "/");
                assert!(ModuleName::from_relative_directory(Path::new(&directory)).is_err(), "{directory}");
                assert!(
                    ModuleName::from_relative_source(Path::new(&format!("{directory}.ncidl"))).is_err(),
                    "{directory}"
                );
            }
        }
    }

    /// Legal spelling remains byte-exact, without importing metadata-only bounds.
    #[test]
    fn logical_segments_preserve_weak_names_case_and_unbounded_shape() {
        for segment in [
            "union",
            "safe",
            "raw",
            "macro_rules",
            "_private",
            "CaseName",
            "case_name",
            "generation",
            "gen_value",
        ] {
            let name = format!("proof.{segment}");
            ModuleName::validate_dotted(&name).unwrap();
            let module = ModuleName::from_relative_source(Path::new(&format!("proof/{segment}.ncidl"))).unwrap();
            assert_eq!(module.to_string(), name);
        }
        let long = "a".repeat(neocogi_com_core::NC_DESCRIPTOR_RUST_PATH_MAX_LEN + 1);
        ModuleName::validate_dotted(&long).unwrap();
        ModuleName::from_relative_source(Path::new(&format!("{long}.ncidl"))).unwrap();
        let names = ["proof/A_b.ncidl", "proof/a_b.ncidl", "proof/a/b.ncidl"].map(|path| ModuleName::from_relative_source(Path::new(path)).unwrap());
        for (index, left) in names.iter().enumerate() {
            for right in names.iter().skip(index + 1) {
                assert_ne!(left, right);
            }
        }
    }
}
