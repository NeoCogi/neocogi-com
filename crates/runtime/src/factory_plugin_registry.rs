//! Runtime-owned index of registered factory-only plugins.
//!
//! A factory plugin stores no runtime construction table: its only executable
//! entry is the fixed root-factory activation pointer in its immutable
//! description.

use std::collections::HashMap;
use std::vec::Vec;

use neocogi_com_core::{NcFactoryPluginDescription, NcGuid};

/// Append-only records for the factory-only plugin model.
pub(crate) struct FactoryPluginRegistry {
    /// Records in successful registration order.
    plugins: Vec<RegisteredFactoryPlugin>,
    /// Enumeration-only identities in the same registration order.
    plugin_ids: Vec<NcGuid>,
    /// Plugin identity to ordered-record index.
    plugin_by_id: HashMap<NcGuid, usize>,
}

impl FactoryPluginRegistry {
    /// Creates an empty factory-only registry.
    pub(crate) fn new() -> Self {
        Self {
            plugins: Vec::new(),
            plugin_ids: Vec::new(),
            plugin_by_id: HashMap::new(),
        }
    }

    /// Returns whether this model already owns a plugin identity.
    pub(crate) fn contains_plugin(&self, plugin_id: NcGuid) -> bool {
        self.plugin_by_id.contains_key(&plugin_id)
    }

    /// Looks up one immutable factory-only plugin record.
    pub(crate) fn plugin(&self, plugin_id: NcGuid) -> Option<&RegisteredFactoryPlugin> {
        self.plugin_by_id.get(&plugin_id).map(|index| &self.plugins[*index])
    }

    /// Borrows registered IDs for unified plugin enumeration.
    pub(crate) fn with_plugin_ids<R>(&self, read: impl FnOnce(&[NcGuid]) -> R) -> R {
        read(&self.plugin_ids)
    }

    /// Borrows one plugin's copied hard dependency IDs.
    pub(crate) fn with_plugin_dependency_ids<R>(&self, plugin_id: NcGuid, read: impl FnOnce(&[NcGuid]) -> R) -> R {
        let dependencies = self
            .plugin_by_id
            .get(&plugin_id)
            .map(|index| self.plugins[*index].dependency_ids.as_slice())
            .unwrap_or(&[]);
        read(dependencies)
    }

    /// Appends one prevalidated record to every index.
    pub(crate) fn insert(&mut self, plugin: RegisteredFactoryPlugin) {
        debug_assert!(!self.contains_plugin(plugin.plugin_id));
        let index = self.plugins.len();
        self.plugin_ids.push(plugin.plugin_id);
        self.plugin_by_id.insert(plugin.plugin_id, index);
        self.plugins.push(plugin);
    }
}

/// Runtime projection of one validated factory-only plugin description.
pub(crate) struct RegisteredFactoryPlugin {
    /// Stable ID derived from the plugin's normative name.
    pub(crate) plugin_id: NcGuid,
    /// Borrowed immutable description retained by static or module ownership.
    pub(crate) description: *const NcFactoryPluginDescription,
    /// Copied hard dependency IDs used for lock-bounded validation.
    pub(crate) dependency_ids: Vec<NcGuid>,
}

// SAFETY: Successful registration retains only pointers to immutable metadata
// whose defining library or static owner outlives the process registry.
unsafe impl Send for RegisteredFactoryPlugin {}
// SAFETY: No runtime operation mutates through the retained description root.
unsafe impl Sync for RegisteredFactoryPlugin {}
