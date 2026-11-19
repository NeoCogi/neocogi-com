//! Dynamic-library loading and plugin registration errors.
//!
//! Errors preserve the failing trusted path, a stable phase category, an
//! optional neocogi-com-sdk status for metadata/registration failures, and detailed
//! platform text. Public C adapters later reduce these records to standardized
//! status categories while sending the full text to debug messengers.
//!
//! Errors retain both a broad machine-readable phase and the platform or
//! validation detail. `NcStatus` is optional because operating-system loader
//! failures do not naturally map to a core status code until the C adapter
//! deliberately reduces them to `NC_E_LOAD_FAILED`.

use std::path::{Path, PathBuf};
use std::string::String;

use neocogi_com::NcStatus;

/// Error category returned while loading a plugin shared library.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginLoadErrorKind {
    /// The operating system loader rejected the shared library path.
    OpenLibrary,
    /// The library did not expose the expected plugin descriptor symbol.
    FindSymbol,
    /// The descriptor entrypoint returned an error or a null descriptor.
    Entrypoint,
    /// The descriptor metadata could not be read as valid policy text.
    Metadata,
    /// Runtime registration rejected the descriptor.
    Registration,
    /// Runtime loading is not implemented for this target platform.
    UnsupportedPlatform,
}

impl core::fmt::Display for PluginLoadErrorKind {
    /// Functional phase names survive profiles that strip derived Debug text.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::OpenLibrary => "OpenLibrary",
            Self::FindSymbol => "FindSymbol",
            Self::Entrypoint => "Entrypoint",
            Self::Metadata => "Metadata",
            Self::Registration => "Registration",
            Self::UnsupportedPlatform => "UnsupportedPlatform",
        })
    }
}

/// Error returned by plugin loading and inspection APIs.
#[derive(Debug)]
pub struct PluginLoadError {
    /// Broad phase used for status mapping and programmatic handling.
    kind: PluginLoadErrorKind,
    /// Caller-supplied path associated with the failed operation.
    path: PathBuf,
    /// Precise neocogi-com-sdk status when the failure originated in ABI logic.
    status: Option<NcStatus>,
    /// Platform or validation detail intended for diagnostics.
    message: String,
}

impl PluginLoadError {
    /// Creates an error while normalizing owned path and message storage.
    pub(super) fn new(kind: PluginLoadErrorKind, path: impl Into<PathBuf>, status: Option<NcStatus>, message: impl Into<String>) -> Self {
        Self {
            kind,
            path: path.into(),
            status,
            message: message.into(),
        }
    }

    /// Returns the broad failure category.
    pub fn kind(&self) -> PluginLoadErrorKind {
        self.kind
    }

    /// Returns the library path that was being loaded.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the `NcStatus` associated with descriptor or registration failures,
    /// including the distinct dynamic-provider capacity bound.
    pub fn status(&self) -> Option<NcStatus> {
        self.status
    }

    /// Returns the loader or validation detail message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl core::fmt::Display for PluginLoadError {
    /// Formats phase, path, detail, and the optional ABI status.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.status {
            Some(status) => write!(f, "{} while loading {}: {} ({})", self.kind, self.path.display(), self.message, status),
            None => write!(f, "{} while loading {}: {}", self.kind, self.path.display(), self.message),
        }
    }
}

impl std::error::Error for PluginLoadError {}

#[cfg(test)]
mod tests {
    use super::{PluginLoadError, PluginLoadErrorKind};
    use neocogi_com::NC_E_ABI_VERSION;

    /// All phases retain their spelling, path, detail and optional symbolic/raw
    /// status. Run under min-size as well: Debug is intentionally unavailable.
    #[test]
    fn display_preserves_every_phase_with_and_without_status() {
        for (kind, phase) in [
            (PluginLoadErrorKind::OpenLibrary, "OpenLibrary"),
            (PluginLoadErrorKind::FindSymbol, "FindSymbol"),
            (PluginLoadErrorKind::Entrypoint, "Entrypoint"),
            (PluginLoadErrorKind::Metadata, "Metadata"),
            (PluginLoadErrorKind::Registration, "Registration"),
            (PluginLoadErrorKind::UnsupportedPlatform, "UnsupportedPlatform"),
        ] {
            assert_eq!(kind.to_string(), phase);
            for status in [None, Some(NC_E_ABI_VERSION)] {
                let error = PluginLoadError::new(kind, "provider.so", status, "validation detail");
                let suffix = if status.is_some() { " (NC_E_ABI_VERSION(-5))" } else { "" };
                assert_eq!(error.to_string(), format!("{phase} while loading provider.so: validation detail{suffix}"));
                assert_eq!(error.kind(), kind);
                assert_eq!(error.path(), std::path::Path::new("provider.so"));
                assert_eq!(error.status(), status);
                assert_eq!(error.message(), "validation detail");
            }
        }
    }
}
