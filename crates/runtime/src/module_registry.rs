//! Ownership and lookup indexes for dynamically loaded native modules.
//!
//! Plugin descriptions belong to the factory-plugin registry. This focused
//! store retains only native handles and the provenance needed to report
//! which library supplied a registered factory plugin. Module IDs are
//! monotonic, nonzero, and bounded by the public runtime limit.

use std::collections::HashMap;
use std::vec::Vec;

use neocogi_com_core::{NC_COM_RUNTIME_MAX_LOADED_MODULES, NcGuid, NcUtf8View};

use crate::loader::{LoadedModule, LoadedModuleId, LoadedModuleInfo};

/// Append-only store for native handles and module provenance.
pub(crate) struct ModuleRegistry {
    /// Next monotonic module ID, or an exhausted sentinel past the limit.
    next_module_id: u64,
    /// Owned handles and provenance in successful activation order.
    records: Vec<LoadedModuleRecord>,
    /// Raw numeric IDs exposed through the C enumeration API.
    module_ids: Vec<u64>,
    /// Process-local ID-to-record index for constant-time lookup.
    module_by_id: HashMap<LoadedModuleId, usize>,
}

impl ModuleRegistry {
    /// Creates an empty module store starting at process-local ID one.
    pub(crate) fn new() -> Self {
        Self::with_next_loaded_module_id(1)
    }

    /// Creates a store with a selected next ID for boundary tests.
    fn with_next_loaded_module_id(next_module_id: u64) -> Self {
        Self {
            next_module_id,
            records: Vec::new(),
            module_ids: Vec::new(),
            module_by_id: HashMap::new(),
        }
    }

    /// Reserves the next nonzero module ID or reports permanent exhaustion.
    pub(crate) fn allocate_loaded_module_id(&mut self) -> Option<LoadedModuleId> {
        let raw = self.next_module_id;
        if raw == 0 || raw > NC_COM_RUNTIME_MAX_LOADED_MODULES as u64 {
            return None;
        }
        // The configured limit is far below `u64::MAX`, so incrementing the
        // final issued ID safely produces the persistent exhausted sentinel.
        self.next_module_id = raw + 1;
        Some(LoadedModuleId::from_raw(raw))
    }

    /// Retains a successful native module and publishes its provenance.
    pub(crate) fn store_loaded_module(&mut self, module: LoadedModule, loaded: LoadedModuleInfo) {
        let module_id = loaded.module_id();
        debug_assert!(!self.module_by_id.contains_key(&module_id));
        let index = self.records.len();
        self.module_ids.push(module_id.get());
        self.module_by_id.insert(module_id, index);
        self.records.push(LoadedModuleRecord { _module: module, loaded });
    }

    /// Borrows ordered raw module IDs under the enclosing catalog guard.
    pub(crate) fn with_loaded_module_ids<R>(&self, read: impl FnOnce(&[u64]) -> R) -> R {
        read(&self.module_ids)
    }

    /// Looks up one retained module as a borrowed C-compatible view.
    pub(crate) fn module(&self, module_id: LoadedModuleId) -> Option<LoadedModuleView> {
        let record = &self.records[*self.module_by_id.get(&module_id)?];
        let path = record.loaded.path().to_str()?;
        Some(LoadedModuleView {
            module_id,
            plugin_id: record.loaded.plugin_id(),
            path: NcUtf8View { data: path.as_ptr(), len: path.len() },
        })
    }
}

/// Owned native module paired with its runtime provenance.
pub(crate) struct LoadedModuleRecord {
    /// Handle retained solely to keep plugin code and metadata mapped.
    _module: LoadedModule,
    /// Path, ID, and plugin association exposed through inspection.
    loaded: LoadedModuleInfo,
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

    /// Confirms the last permitted ID is issued exactly once.
    #[test]
    fn loaded_module_limit_issues_final_id_then_remains_exhausted() {
        let mut registry = ModuleRegistry::with_next_loaded_module_id(NC_COM_RUNTIME_MAX_LOADED_MODULES as u64);

        assert_eq!(
            registry.allocate_loaded_module_id().map(LoadedModuleId::get),
            Some(NC_COM_RUNTIME_MAX_LOADED_MODULES as u64)
        );
        assert_eq!(registry.allocate_loaded_module_id(), None);
        assert_eq!(registry.allocate_loaded_module_id(), None);
    }

    /// Confirms an invalid starting sentinel can never leak as a public ID.
    #[test]
    fn loaded_module_allocator_never_publishes_zero() {
        let mut registry = ModuleRegistry::with_next_loaded_module_id(0);

        assert_eq!(registry.allocate_loaded_module_id(), None);
        assert_eq!(registry.allocate_loaded_module_id(), None);
    }
}
