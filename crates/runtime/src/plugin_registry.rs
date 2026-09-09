//! Storage and indexes for plugin identities and retained native modules.
//!
//! Plugin records and native handles live inside the runtime's single locked
//! catalog. Module IDs are monotonic, nonzero, and capped before handles enter
//! the retained process-lifetime list.
//!
//! Ordered vectors define ABI enumeration order. Hash indexes are lookup-only
//! and mutate only under the enclosing catalog write guard.

use std::collections::HashMap;
use std::vec::Vec;

use neocogi_com_core::{NC_COM_RUNTIME_MAX_LOADED_MODULES, NcGuid, NcPluginDescriptor, NcUtf8View};

use crate::loader::{LoadedModule, LoadedModuleId, LoadedModuleInfo};

/// Append-only catalogs for registered plugins and retained native modules.
pub(crate) struct PluginRegistry {
    /// Module handles and provenance in activation order.
    loaded_modules: LoadedModuleState,
    /// Append-only plugin records and lookup indexes.
    plugins: PluginState,
}

/// Append-only plugin records and lookup indexes.
struct PluginState {
    /// Full registration records in deterministic order.
    plugins: Vec<RegisteredPlugin>,
    /// Enumeration-only identity vector in the same order.
    plugin_ids: Vec<NcGuid>,
    /// Identity-to-record index for constant-time lookup.
    plugin_by_id: HashMap<NcGuid, usize>,
}

/// Retained native modules and their process-local identities.
struct LoadedModuleState {
    /// Next monotonic module ID, bounded by the public module limit.
    next_module_id: u64,
    /// Owned handles and provenance in activation order.
    records: Vec<LoadedModuleRecord>,
    /// Raw numeric IDs exposed by the C enumeration API.
    module_ids: Vec<u64>,
    /// Process-local ID-to-record index.
    module_by_id: HashMap<LoadedModuleId, usize>,
}

impl PluginRegistry {
    /// Creates empty plugin and module catalogs starting at module ID one.
    pub(crate) fn new() -> Self {
        Self::with_next_loaded_module_id(1)
    }

    /// Creates empty catalogs with a selected next ID for limit/exhaustion tests.
    fn with_next_loaded_module_id(next_module_id: u64) -> Self {
        Self {
            loaded_modules: LoadedModuleState {
                next_module_id,
                records: Vec::new(),
                module_ids: Vec::new(),
                module_by_id: HashMap::new(),
            },
            plugins: PluginState {
                plugins: Vec::new(),
                plugin_ids: Vec::new(),
                plugin_by_id: HashMap::new(),
            },
        }
    }

    /// Reserves the next nonzero module ID or reports permanent capacity exhaustion.
    pub(crate) fn allocate_loaded_module_id(&mut self) -> Option<LoadedModuleId> {
        let raw = self.loaded_modules.next_module_id;
        if raw == 0 || raw > NC_COM_RUNTIME_MAX_LOADED_MODULES as u64 {
            return None;
        }
        // The configured limit is far below `u64::MAX`, so this stores the
        // permanent exhausted sentinel after issuing the final valid ID.
        self.loaded_modules.next_module_id = raw + 1;
        Some(LoadedModuleId::from_raw(raw))
    }

    /// Retains a successful native module and publishes its provenance record.
    pub(crate) fn store_loaded_module(&mut self, module: LoadedModule, loaded: LoadedModuleInfo) {
        let module_id = loaded.module_id();
        debug_assert!(!self.loaded_modules.module_by_id.contains_key(&module_id));
        let index = self.loaded_modules.records.len();
        self.loaded_modules.module_ids.push(module_id.get());
        self.loaded_modules.module_by_id.insert(module_id, index);
        self.loaded_modules.records.push(LoadedModuleRecord { _module: module, loaded });
    }

    /// Borrows the ordered raw module IDs under the enclosing catalog guard.
    pub(crate) fn with_loaded_module_ids<R>(&self, read: impl FnOnce(&[u64]) -> R) -> R {
        read(&self.loaded_modules.module_ids)
    }

    /// Looks up a borrowed ABI projection for one retained module.
    pub(crate) fn module(&self, module_id: LoadedModuleId) -> Option<LoadedModuleView> {
        let record = &self.loaded_modules.records[*self.loaded_modules.module_by_id.get(&module_id)?];
        let path = record.loaded.path().to_str()?;
        Some(LoadedModuleView {
            module_id,
            plugin_id: record.loaded.plugin_id(),
            path: NcUtf8View { data: path.as_ptr(), len: path.len() },
        })
    }

    #[cfg(test)]
    /// Copies registered plugin IDs for direct registry unit tests.
    pub(crate) fn registered_plugin_ids(&self) -> Vec<NcGuid> {
        self.with_registered_plugin_ids(<[NcGuid]>::to_vec)
    }

    /// Borrows registered plugin IDs under the enclosing catalog read guard.
    pub(crate) fn with_registered_plugin_ids<R>(&self, read: impl FnOnce(&[NcGuid]) -> R) -> R {
        read(&self.plugins.plugin_ids)
    }

    /// Borrows one plugin's hard dependency IDs, or an empty slice when absent.
    pub(crate) fn with_plugin_dependency_ids<R>(&self, plugin_id: NcGuid, read: impl FnOnce(&[NcGuid]) -> R) -> R {
        let dependencies = self
            .plugins
            .plugin_by_id
            .get(&plugin_id)
            .map(|index| self.plugins.plugins[*index].dependency_ids.as_slice())
            .unwrap_or(&[]);
        read(dependencies)
    }

    /// Looks up the immutable descriptor pointer for one registered plugin.
    pub(crate) fn plugin(&self, plugin_id: NcGuid) -> Option<RegisteredPluginView> {
        let plugin = &self.plugins.plugins[*self.plugins.plugin_by_id.get(&plugin_id)?];
        Some(RegisteredPluginView {
            plugin_id: plugin.plugin_id,
            plugin: plugin.plugin,
        })
    }

    /// Returns whether a plugin identity is already registered.
    pub(crate) fn contains_plugin(&self, plugin_id: NcGuid) -> bool {
        self.plugins.plugin_by_id.contains_key(&plugin_id)
    }

    /// Appends a not-yet-present plugin to every ordered/indexed view.
    pub(crate) fn insert(&mut self, plugin: RegisteredPlugin) {
        if self.plugins.plugin_by_id.contains_key(&plugin.plugin_id) {
            return;
        }
        let index = self.plugins.plugins.len();
        self.plugins.plugin_ids.push(plugin.plugin_id);
        self.plugins.plugin_by_id.insert(plugin.plugin_id, index);
        self.plugins.plugins.push(plugin);
    }
}

#[derive(Clone, Copy)]
/// Copyable lookup projection over a registered plugin record.
pub(crate) struct RegisteredPluginView {
    /// Stable registered plugin identity.
    pub(crate) plugin_id: NcGuid,
    /// Borrowed immutable descriptor root.
    pub(crate) plugin: *const NcPluginDescriptor,
}

/// Runtime record retaining a plugin descriptor and copied dependency edges.
pub(crate) struct RegisteredPlugin {
    /// Stable registered plugin identity.
    pub(crate) plugin_id: NcGuid,
    /// Borrowed descriptor root kept live by caller/static/module ownership.
    pub(crate) plugin: *const NcPluginDescriptor,
    /// Validated dependency IDs copied for lock-bounded queries.
    pub(crate) dependency_ids: Vec<NcGuid>,
}

// SAFETY: Registered plugin pointers refer to immutable validated metadata that
// remains live while registered.
unsafe impl Send for RegisteredPlugin {}
// SAFETY: The registry never mutates through stored descriptor pointers.
unsafe impl Sync for RegisteredPlugin {}

/// Owned native module paired with its runtime provenance.
pub(crate) struct LoadedModuleRecord {
    /// Handle retained solely to keep plugin code and metadata mapped.
    pub(crate) _module: LoadedModule,
    /// Path, ID, and plugin association exposed through inspection.
    pub(crate) loaded: LoadedModuleInfo,
}

#[derive(Clone, Copy)]
/// Borrowed C-compatible projection of one retained module record.
pub(crate) struct LoadedModuleView {
    /// Process-local module identifier.
    pub(crate) module_id: LoadedModuleId,
    /// Plugin supplied by this module.
    pub(crate) plugin_id: NcGuid,
    /// Borrowed UTF-8 form of the retained path.
    pub(crate) path: NcUtf8View,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loaded_module_limit_issues_final_id_then_remains_exhausted() {
        let mut registry = PluginRegistry::with_next_loaded_module_id(NC_COM_RUNTIME_MAX_LOADED_MODULES as u64);

        assert_eq!(
            registry.allocate_loaded_module_id().map(LoadedModuleId::get),
            Some(NC_COM_RUNTIME_MAX_LOADED_MODULES as u64)
        );
        assert_eq!(registry.allocate_loaded_module_id(), None);
        assert_eq!(registry.allocate_loaded_module_id(), None);
    }

    #[test]
    fn loaded_module_allocator_never_publishes_zero() {
        let mut registry = PluginRegistry::with_next_loaded_module_id(0);

        assert_eq!(registry.allocate_loaded_module_id(), None);
        assert_eq!(registry.allocate_loaded_module_id(), None);
    }
}
