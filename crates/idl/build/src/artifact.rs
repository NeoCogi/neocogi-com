//! Stable metadata exchanged between contract builds and assembly tooling.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Schema version for one native contract artifact record.
///
/// Producers and consumers share this constant so a schema revision cannot be
/// accepted by one side while remaining hard-coded to an older value elsewhere.
pub const ARTIFACT_SCHEMA: u32 = 1;

/// Machine-readable identity and path of one exact native contract artifact.
///
/// Contract build scripts serialize this record into their Cargo output
/// directory. Assembly tools deserialize the same type before validating and
/// staging its native files, so this definition is the schema authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactRecord {
    /// Record schema version checked by build and assembly tooling.
    pub schema: u32,
    /// Exact dotted logical module name.
    pub module: String,
    /// Stable UUID string derived from the logical module name.
    pub module_id: String,
    /// Complete lowercase SHA-256 ABI fingerprint.
    pub abi_fingerprint: String,
    /// Rust target triple for which the artifact was compiled.
    pub target: String,
    /// Native linkage mode, either `dynamic` or `static`.
    pub linkage: String,
    /// Fingerprint-qualified linker name without platform decoration.
    pub link_name: String,
    /// Absolute path to the runtime shared library or static archive.
    pub library: PathBuf,
    /// Platform linker companion files, such as a Windows import library.
    pub linker_files: Vec<PathBuf>,
    /// Fingerprint-qualified exported description getter.
    pub getter: String,
}
