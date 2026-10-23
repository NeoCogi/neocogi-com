//! Runtime-free persistent facts shared by the two sample assemblies.
use neocogi_com::{ComDeserialize, ComSerialize, NcGuid};

/// Recipe identity used by the explicitly registered persistence adapter.
pub const PERSISTENT_KIND: NcGuid = neocogi_com::guid_from_qualified_name("nc.examples.extent-state");

/// Version-one persistent state; neither Rust layout nor event receivers are saved.
#[derive(ComSerialize, ComDeserialize)]
pub struct ExtentState {
    /// Authored/current width.
    pub width: u32,
    /// Authored/current height.
    pub height: u32,
}
