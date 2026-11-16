//! Explicit application bootstrap for a process-pinned host runtime image.
//!
//! No default host, global cache or static implementation dependency is selected.
//! Failed bootstrap returns an error; the caller decides startup policy. Provider
//! acquisition continues forwarding the selected host's tables unchanged.

use core::ffi::c_void;
use std::{
    fmt,
    path::{Path, PathBuf},
    string::{String, ToString},
};
use crate::{NC_GET_HOST_API_SYMBOL, NcGetHostApi, NcStatus, RuntimeHandle};
use crate::native_library::{DynamicLibrary, OpenLibraryError};

/// Phase that rejected an explicitly selected host runtime image.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostLoadErrorKind {
    /// The native loader rejected the selected filesystem path.
    OpenLibrary,
    /// The image did not export the versioned host bootstrap.
    FindSymbol,
    /// The bootstrap returned a failing COM status.
    Entrypoint,
    /// Bootstrap succeeded but its host graph failed size/version admission.
    InvalidApi,
    /// This target has no native loader implementation.
    UnsupportedPlatform,
}

impl fmt::Display for HostLoadErrorKind {
    /// Keeps diagnostics readable even when optional Debug text is stripped.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::OpenLibrary => "OpenLibrary",
            Self::FindSymbol => "FindSymbol",
            Self::Entrypoint => "Entrypoint",
            Self::InvalidApi => "InvalidApi",
            Self::UnsupportedPlatform => "UnsupportedPlatform",
        })
    }
}

/// Startup failure retaining the caller's original path and precise ABI status.
#[derive(Debug)]
pub struct HostLoadError {
    /// Bootstrap phase that failed; no fallback host is created.
    kind: HostLoadErrorKind,
    /// Original caller spelling, independent of temporary path qualification.
    path: PathBuf,
    /// Exact failure from bootstrap or host-graph admission, when available.
    status: Option<NcStatus>,
    /// Owned native-loader or contract diagnostic.
    message: String,
}

impl HostLoadError {
    /// Adds application bootstrap context to a native or admission failure.
    fn new(kind: HostLoadErrorKind, path: &Path, status: Option<NcStatus>, message: impl Into<String>) -> Self {
        Self {
            kind,
            path: path.to_path_buf(),
            status,
            message: message.into(),
        }
    }

    /// Returns the phase that failed.
    pub fn kind(&self) -> HostLoadErrorKind {
        self.kind
    }

    /// Returns the caller's original runtime-library path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the exact bootstrap/admission status, when the failure has one.
    pub fn status(&self) -> Option<NcStatus> {
        self.status
    }

    /// Returns the native-loader or host-contract diagnostic.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for HostLoadError {
    /// Formats explicit phase/path diagnostics with optional symbolic ABI status.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} while loading host {}: {}", self.kind, self.path.display(), self.message)?;
        if let Some(status) = self.status {
            write!(f, " ({status})")?;
        }
        Ok(())
    }
}
impl std::error::Error for HostLoadError {}

impl RuntimeHandle {
    /// Loads one explicitly selected host runtime image for application startup.
    ///
    /// Resolve this once in the final application instead of linking the static
    /// host implementation. Pass the resulting access to providers; their nested
    /// acquisitions use those same tables. This function creates no SDK registry,
    /// performs no plugin loading and selects no fallback host on failure.
    ///
    /// Relative paths are qualified against the working directory, including bare
    /// names; ambient library search is not a host-selection mechanism. Every
    /// successfully opened image remains pinned, including on later symbol or
    /// admission failure. Returned handles and callbacks remain process-lived.
    ///
    /// # Safety
    /// The path selects trusted native code implementing `nc_get_host_api_v3`
    /// with the exact [`NcGetHostApi`] contract. A successful graph satisfies
    /// [`RuntimeHandle::__from_host_api`]'s complete backing, immutability, coherence
    /// and callback obligations. Size/version checks cannot establish these.
    /// The application selects one owner; opening another image or initializing
    /// a statically linked implementation can create an independent registry.
    pub unsafe fn load_host_library(path: impl AsRef<Path>) -> Result<Self, HostLoadError> {
        let path = path.as_ref();
        if path.as_os_str().is_empty() {
            return Err(HostLoadError::new(HostLoadErrorKind::OpenLibrary, path, None, "host library path is empty"));
        }
        let absolute = std::path::absolute(path).map_err(|error| HostLoadError::new(HostLoadErrorKind::OpenLibrary, path, None, error.to_string()))?;
        // SAFETY: the application promises a trusted selected runtime image.
        let library = unsafe { DynamicLibrary::open(&absolute) }.map_err(|error| match error {
            OpenLibraryError::UnsupportedPlatform => HostLoadError::new(
                HostLoadErrorKind::UnsupportedPlatform,
                path,
                None,
                "dynamic loading is not implemented for this target",
            ),
            OpenLibraryError::OpenLibrary(message) => HostLoadError::new(HostLoadErrorKind::OpenLibrary, path, None, message),
        })?;
        // SAFETY: the versioned name has the trusted runtime bootstrap signature.
        let symbol = unsafe { library.symbol(NC_GET_HOST_API_SYMBOL.as_bytes()) }
            .map_err(|message| HostLoadError::new(HostLoadErrorKind::FindSymbol, path, None, message))?;
        // SAFETY: this named native export uses the public C function-pointer type.
        let get = unsafe { core::mem::transmute::<*mut c_void, NcGetHostApi>(symbol) };
        // SAFETY: the selected image honors the host graph contract and is pinned
        // even when the native wrapper leaves this scope or admission fails.
        unsafe { host_from_entry(get, path) }
    }
}

/// Calls a trusted bootstrap before admitting its returned graph.
///
/// # Safety
/// `get` and every successful graph obey the process-lived `NcGetHostApi` contract.
unsafe fn host_from_entry(get: NcGetHostApi, path: &Path) -> Result<RuntimeHandle, HostLoadError> {
    let mut host = core::ptr::null();
    // SAFETY: the local output slot is writable for one host pointer.
    let status = unsafe { get(&mut host) };
    if status.is_err() {
        // Failure grants no graph access, even when an invalid pointer was written.
        return Err(HostLoadError::new(HostLoadErrorKind::Entrypoint, path, Some(status), "host bootstrap failed"));
    }
    // SAFETY: success promises readable prefixes and valid supported graph storage.
    unsafe { RuntimeHandle::__from_host_api(host) }
        .map_err(|status| HostLoadError::new(HostLoadErrorKind::InvalidApi, path, Some(status), "host API admission failed"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NC_COM_CORE_ABI_VERSION, NC_COM_RUNTIME_ABI_VERSION, NC_E_ABI_VERSION, NC_E_NOT_FOUND, NC_E_POINTER, NC_OK, NcHostApiV3};

    /// Only the advertised size exists; admission must reject before reading more.
    #[repr(C)]
    struct ShortHost {
        /// Zero advertises no complete host record.
        size: usize,
    }
    /// Real short storage exercises prefix validation without fabricated memory.
    static SHORT: ShortHost = ShortHost { size: 0 };
    /// Readable version prefix rejects the graph before null children are visited.
    static WRONG_VERSION: NcHostApiV3 = NcHostApiV3 {
        size: core::mem::size_of::<NcHostApiV3>(),
        runtime_abi_version: NC_COM_RUNTIME_ABI_VERSION + 1,
        core_abi_version: NC_COM_CORE_ABI_VERSION,
        runtime: core::ptr::null(),
        debug: core::ptr::null(),
        persistence: core::ptr::null(),
    };

    /// Failure output is deliberately unusable; no admission read is permitted.
    unsafe extern "C" fn failed(output: *mut *const NcHostApiV3) -> NcStatus {
        // SAFETY: the bootstrap caller supplies one writable pointer slot.
        unsafe { *output = core::ptr::dangling() };
        NC_E_NOT_FOUND
    }
    /// Success without a graph must fail the host validator.
    unsafe extern "C" fn absent(_output: *mut *const NcHostApiV3) -> NcStatus {
        NC_OK
    }
    /// Only the leading size is readable, as the admission contract permits.
    unsafe extern "C" fn short(output: *mut *const NcHostApiV3) -> NcStatus {
        // SAFETY: local output slot is writable; SHORT lives for process lifetime.
        unsafe { *output = core::ptr::from_ref(&SHORT).cast() };
        NC_OK
    }
    /// Unsupported versions permit prefix reads only, without child access.
    unsafe extern "C" fn wrong_version(output: *mut *const NcHostApiV3) -> NcStatus {
        // SAFETY: local output slot is writable; the complete version prefix is live.
        unsafe { *output = &WRONG_VERSION };
        NC_OK
    }

    /// Error status takes precedence over output, then success is admitted by prefixes.
    #[test]
    fn bootstrap_preserves_errors_and_validates_only_successful_outputs() {
        for (get, kind, status) in [
            (failed as NcGetHostApi, HostLoadErrorKind::Entrypoint, NC_E_NOT_FOUND),
            (absent, HostLoadErrorKind::InvalidApi, NC_E_POINTER),
            (short, HostLoadErrorKind::InvalidApi, NC_E_ABI_VERSION),
            (wrong_version, HostLoadErrorKind::InvalidApi, NC_E_ABI_VERSION),
        ] {
            let path = Path::new("selected-host");
            // SAFETY: fixtures grant precisely the readable storage permitted by
            // their status, size and version; rejected graphs grant no callbacks.
            let error = unsafe { host_from_entry(get, path) }.unwrap_err();
            assert_eq!(error.kind(), kind);
            assert_eq!(error.status(), Some(status));
            assert_eq!(error.path(), path);
            assert!(error.to_string().contains("selected-host"));
        }
    }
}
