//! Safe Rust view of the runtime plugin registry.
//!
//! This context answers only what plugin and module identities exist. Metadata
//! and relationships belong to `Inspection`, while loading remains on
//! `Runtime`, preventing one catch-all registry API from growing again.

use std::vec::Vec;

use crate::{NcGuid, NcStatus};

use super::{LoadedModuleId, Runtime, read_vec};

/// Read-only catalog of registered plugins and retained plugin modules.
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

    /// Returns all retained loaded-module IDs.
    pub fn loaded_module_ids(self) -> Result<Vec<LoadedModuleId>, NcStatus> {
        let api = self.runtime.plugin_registry_api();
        read_vec(
            || unsafe { (api.loaded_module_count)() },
            |out, capacity, out_len| unsafe { (api.loaded_module_ids)(out, capacity, out_len) },
        )
        .map(|ids| ids.into_iter().map(LoadedModuleId::from_raw).collect())
    }
}
