//! Runtime-owned index of registered plugins with one typed root.
//!
//! A factory plugin stores no runtime construction table: its only executable
//! entry is the fixed root-factory acquisition pointer in its immutable
//! description.

use std::collections::HashMap;
use std::vec::Vec;
use std::path::PathBuf;

use crate::descriptor::ValidatedPluginDescriptor;
use neocogi_com::{NcPluginDependencyDescriptor, NcGuid};

/// Append-only plugin records and identity lookup for the selected host owner.
/// Each plugin exposes one typed root factory; entries are plugins rather than
/// separately registered classes or factory implementations.
pub(crate) struct PluginRegistry {
    /// Records in successful registration order.
    plugins: Vec<RegisteredPlugin>,
    /// Plugin identity to ordered-record index.
    plugin_by_id: HashMap<NcGuid, usize>,
}

impl PluginRegistry {
    /// Creates an empty plugin registry.
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

    /// Looks up one immutable plugin record.
    pub(crate) fn plugin(&self, plugin_id: NcGuid) -> Option<&RegisteredPlugin> {
        self.plugin_by_id.get(&plugin_id).map(|index| &self.plugins[*index])
    }

    /// Counts registered plugins from the authoritative ordered records.
    pub(crate) fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Iterates immutable records in successful registration order.
    pub(crate) fn plugins(&self) -> impl ExactSizeIterator<Item = &RegisteredPlugin> {
        self.plugins.iter()
    }

    /// Requires every hard dependency to be registered before publication.
    pub(crate) fn validate_plugin_dependencies(&self, dependencies: &[NcPluginDependencyDescriptor]) -> Result<(), neocogi_com::NcStatus> {
        if dependencies.iter().any(|dependency| !self.contains_plugin(dependency.plugin_id)) {
            return Err(neocogi_com::NC_E_NOT_FOUND);
        }
        Ok(())
    }

    /// Appends one prevalidated record to every index.
    pub(crate) fn insert(&mut self, plugin: RegisteredPlugin) {
        debug_assert!(!self.contains_plugin(plugin.descriptor.id()));
        let index = self.plugins.len();
        self.plugin_by_id.insert(plugin.descriptor.id(), index);
        self.plugins.push(plugin);
    }
}

/// One validated plugin descriptor and its first registration provenance.
pub(crate) struct RegisteredPlugin {
    /// One validated process-lived description owns identity and dependency views.
    pub(crate) descriptor: ValidatedPluginDescriptor,
    /// Actual provenance supplied before validation and atomic publication.
    pub(crate) origin: PluginOrigin,
}

/// Immutable provenance of a registered plugin; no separate module identity exists.
pub(crate) enum PluginOrigin {
    /// Descriptor and code are supplied by process-lifetime static storage.
    Static,
    /// The first successful native load owns the path used for diagnostics.
    /// Its heap-backed path remains live after metadata readers release their
    /// catalog guards. Platform opening pins the mapping independently of this
    /// record, including failed/repeated loads and inspection-only opens.
    Dynamic {
        /// Exact nonempty UTF-8 caller path, copied before publication.
        path: PathBuf,
    },
}
