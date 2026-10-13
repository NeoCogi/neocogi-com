//! Runtime-owned index of registered factory-only plugins.
//!
//! A factory plugin stores no runtime construction table: its only executable
//! entry is the fixed root-factory acquisition pointer in its immutable
//! description.

use std::collections::HashMap;
use std::vec::Vec;
use std::path::PathBuf;

use crate::loader::PinnedLibrary;

use neocogi_com_core::{NcPluginDescriptor, NcGuid};

/// Append-only records for the factory-only plugin model.
pub(crate) struct FactoryPluginRegistry {
    /// Records in successful registration order.
    plugins: Vec<RegisteredFactoryPlugin>,
    /// Plugin identity to ordered-record index.
    plugin_by_id: HashMap<NcGuid, usize>,
}

impl FactoryPluginRegistry {
    /// Creates an empty factory-only registry.
    pub(crate) fn new() -> Self {
        Self {
            plugins: Vec::new(),
            plugin_by_id: HashMap::new(),
        }
    }

    /// Returns whether this model already owns a plugin identity.
    pub(crate) fn contains_plugin(&self, plugin_id: NcGuid) -> bool {
        self.plugin_by_id.contains_key(&plugin_id)
    }

    /// Counts distinct dynamic providers directly from authoritative records.
    pub(crate) fn dynamic_count(&self) -> usize {
        self.plugins
            .iter()
            .filter(|plugin| matches!(plugin.origin, PluginOrigin::Dynamic { .. }))
            .count()
    }

    /// Looks up one immutable factory-only plugin record.
    pub(crate) fn plugin(&self, plugin_id: NcGuid) -> Option<&RegisteredFactoryPlugin> {
        self.plugin_by_id.get(&plugin_id).map(|index| &self.plugins[*index])
    }

    /// Counts registered plugins from the authoritative ordered records.
    pub(crate) fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Iterates immutable records in successful registration order.
    pub(crate) fn plugins(&self) -> impl ExactSizeIterator<Item = &RegisteredFactoryPlugin> {
        self.plugins.iter()
    }

    /// Requires every hard dependency to be registered before publication.
    pub(crate) fn validate_plugin_dependencies(&self, dependency_ids: &[NcGuid]) -> Result<(), neocogi_com_core::NcStatus> {
        if dependency_ids.iter().any(|id| !self.contains_plugin(*id)) {
            return Err(neocogi_com_core::NC_E_NOT_FOUND);
        }
        Ok(())
    }

    /// Appends one prevalidated record to every index.
    pub(crate) fn insert(&mut self, plugin: RegisteredFactoryPlugin) {
        debug_assert!(!self.contains_plugin(plugin.plugin_id));
        let index = self.plugins.len();
        self.plugin_by_id.insert(plugin.plugin_id, index);
        self.plugins.push(plugin);
    }
}

/// Runtime projection of one validated plugin descriptor.
pub(crate) struct RegisteredFactoryPlugin {
    /// Stable ID derived from the plugin's normative name.
    pub(crate) plugin_id: NcGuid,
    /// Borrowed immutable description retained by static or pinned library ownership.
    pub(crate) description: *const NcPluginDescriptor,
    /// Immutable copied provider IDs used for validation and borrowed metadata.
    /// Published buffers never change; their allocations survive record movement.
    pub(crate) dependency_ids: Vec<NcGuid>,
    /// Provenance and code owner published with this immutable descriptor.
    pub(crate) origin: PluginOrigin,
}

// SAFETY: Successful registration retains only pointers to immutable metadata
// whose defining library or static owner outlives the process registry.
unsafe impl Send for RegisteredFactoryPlugin {}
// SAFETY: No runtime operation mutates through the retained description root.
unsafe impl Sync for RegisteredFactoryPlugin {}

/// Immutable provenance of a registered plugin; no separate module identity exists.
pub(crate) enum PluginOrigin {
    /// Descriptor and code are supplied by process-lifetime static storage.
    Static,
    /// The first successful native load owns the path used for diagnostics.
    /// Its heap-backed path and pinned mapping remain live for the process,
    /// including after metadata readers release their catalog guards.
    Dynamic {
        /// Exact nonempty UTF-8 caller path, copied before publication.
        path: PathBuf,
        /// Private native-loader owner; mappings are never unloaded in this runtime.
        _library: PinnedLibrary,
    },
}
