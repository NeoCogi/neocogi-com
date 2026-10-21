//! Owned inspection views of self-contained factory plugin metadata.
//!
//! Inspection pins a trusted library, validates its descriptor, and copies the
//! plugin's identity, dependencies, and exact factory signature. It neither
//! registers the plugin nor invokes its bootstrap. IDL module inventories are
//! build-time information and do not participate in this runtime operation.

use std::path::Path;
use std::string::String;
use std::vec::Vec;

use neocogi_com_core::{NcAbiThreadingMode, NcGuid};

use super::{PluginLoadError, open_plugin_descriptor};

/// Owned metadata extracted without constructing a plugin's root factory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginInspection {
    /// Plugin descriptor generation validated before reading its fields.
    pub description_abi_version: u32,
    /// Core object ABI generation understood by the plugin.
    pub core_abi_version: u32,
    /// Stable plugin identifier derived from its normative name.
    pub plugin_id: NcGuid,
    /// Normative plugin name copied out of native storage.
    pub qualified_name: String,
    /// Plugin release version copied out of native storage.
    pub plugin_version: String,
    /// Identities of providers that must be registered before this plugin.
    pub dependencies: Vec<NcGuid>,
    /// Exact contract and threading promise of the root factory.
    pub factory: FactoryInspection,
}

/// Owned factory compatibility data, independent of native pointer lifetimes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactoryInspection {
    /// Stable interface identity returned by the zero-argument bootstrap.
    pub root_factory_id: NcGuid,
    /// Complete generated signature, including the transitive IDL fingerprint.
    pub root_factory_signature: [u8; 32],
    /// Threading capability promised by the returned root factory.
    pub root_factory_threading: NcAbiThreadingMode,
}

/// Opens a trusted plugin and copies its metadata without registration.
///
/// The native loader may run library initialization. The opened library remains
/// pinned for the inspector process lifetime, including after validation fails.
///
/// # Safety
///
/// `path` must identify trusted code obeying the plugin descriptor ABI.
pub unsafe fn inspect_plugin_library(path: impl AsRef<Path>) -> Result<PluginInspection, PluginLoadError> {
    let path = path.as_ref();
    // SAFETY: the selected library promises immutable process-lived metadata/code.
    let descriptor = unsafe { open_plugin_descriptor(path) }?;
    let plugin = descriptor.get();
    Ok(PluginInspection {
        description_abi_version: plugin.description_abi_version,
        core_abi_version: plugin.core_abi_version,
        plugin_id: descriptor.id(),
        qualified_name: descriptor.qualified_name().to_owned(),
        plugin_version: descriptor.version().to_owned(),
        dependencies: descriptor.dependencies().iter().map(|dependency| dependency.plugin_id).collect(),
        factory: FactoryInspection {
            root_factory_id: plugin.root_factory.iid,
            root_factory_signature: *descriptor.factory_signature(),
            root_factory_threading: descriptor.factory_threading(),
        },
    })
}
