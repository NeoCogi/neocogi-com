//! Logical module names derived from portable relative source paths.

use std::{
    fmt,
    path::{Component, Path},
};

/// A validated dotted IDL module name such as `neocogi.ui`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModuleName {
    /// Validated source-path segments in declaration order.
    segments: Vec<String>,
}

impl ModuleName {
    /// Derives a module name from one `.ncidl` path relative to an IDL root.
    ///
    /// Filesystem separators become dots and the final `.ncidl` extension is
    /// removed. Absolute paths, parent traversal, non-UTF-8 names, and segments
    /// that are not portable identifiers are rejected.
    pub fn from_relative_source(path: &Path) -> Result<Self, ModulePathError> {
        if path.is_absolute() {
            return Err(ModulePathError::Absolute);
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("ncidl") {
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
            let segment = if index + 1 == component_count {
                text.strip_suffix(".ncidl").ok_or(ModulePathError::Extension)?
            } else {
                text
            };
            if !is_identifier(segment) {
                return Err(ModulePathError::InvalidSegment(segment.to_owned()));
            }
            segments.push(segment.to_owned());
        }

        if segments.is_empty() {
            return Err(ModulePathError::Empty);
        }
        Ok(Self { segments })
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
    /// One path component was not a portable IDL identifier.
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
            Self::InvalidSegment(segment) => write!(formatter, "IDL module path segment {segment:?} is not a portable identifier"),
        }
    }
}

impl std::error::Error for ModulePathError {}

/// Returns whether one path segment is an ASCII identifier accepted on every host.
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
}
