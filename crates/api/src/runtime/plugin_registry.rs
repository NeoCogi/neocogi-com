//! Read-only enumeration of registered plugin GUIDs.
//!
//! Metadata and provenance belong to inspection; loading remains on `Runtime`.
//! The facade owns its enumeration result and never borrows catalog storage.

use std::vec::Vec;

use crate::{NcGuid, NcStatus};

use super::{Runtime, read_vec};

/// Read-only catalog of registered plugins.
#[derive(Clone, Copy, Debug)]
pub struct PluginRegistry<'runtime> {
    /// Zero-sized facade borrow; this does not borrow registry storage or a lock.
    runtime: &'runtime Runtime,
}

impl<'runtime> PluginRegistry<'runtime> {
    /// Creates a plugin-registry context borrowing the runtime facade.
    pub(super) const fn new(runtime: &'runtime Runtime) -> Self {
        Self { runtime }
    }

    /// Returns all registered plugin IDs.
    pub fn plugin_ids(self) -> Result<Vec<NcGuid>, NcStatus> {
        let api = self.runtime.plugin_registry_api();
        read_vec(
            || unsafe { (api.plugin_count)() },
            |out, capacity, out_len| unsafe { (api.plugin_ids)(out, capacity, out_len) },
        )
    }
}
