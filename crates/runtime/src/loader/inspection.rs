//! Owned inspection views of self-contained factory plugin metadata.
//!
//! Inspection pins a trusted library, validates its descriptor, and copies the
//! plugin's identity, dependencies, and exact factory signature. It neither
//! registers the plugin nor invokes its bootstrap. IDL module inventories are
//! build-time information and do not participate in this runtime operation.

use std::mem::ManuallyDrop;
use std::path::Path;
use std::string::String;
use std::vec::Vec;

use neocogi_com_core::{NC_E_INVALID_ARG, NcAbiThreadingMode, NcFactoryPluginDescription, NcGuid, NcStaticUtf8, validate_factory_plugin_description};

use super::{LoadedModule, PluginLoadError, PluginLoadErrorKind, factory_plugin_description_from_entrypoint};

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
    // SAFETY: The caller trusts the selected library and its fixed C entrypoint.
    let plugin = unsafe { inspect_factory_plugin_description(path) }?;
    // SAFETY: Validation established readable metadata in a pinned library.
    let plugin = unsafe { &*plugin };
    // SAFETY: The signature was validated and its storage remains pinned.
    let root_factory_signature = *unsafe { plugin.root_factory.abi_signature.as_digest() }.ok_or_else(|| {
        PluginLoadError::new(PluginLoadErrorKind::Metadata, path, Some(NC_E_INVALID_ARG), "missing root factory signature")
    })?;
    let root_factory_threading = NcAbiThreadingMode::from_raw(plugin.root_factory_threading)
        .ok_or_else(|| PluginLoadError::new(PluginLoadErrorKind::Metadata, path, Some(NC_E_INVALID_ARG), "invalid root factory threading"))?;
    // SAFETY: Descriptor validation checked this immutable pointer/count table.
    let dependencies = unsafe { plugin.dependencies() }
        .map_err(|status| PluginLoadError::new(PluginLoadErrorKind::Metadata, path, Some(status), "invalid plugin dependencies"))?
        .iter()
        .map(|dependency| dependency.plugin_id)
        .collect();
    Ok(PluginInspection {
        description_abi_version: plugin.description_abi_version,
        core_abi_version: plugin.core_abi_version,
        plugin_id: plugin.plugin_id,
        // SAFETY: Both views refer to validated UTF-8 in the pinned library.
        qualified_name: unsafe { copy_static_utf8(path, plugin.qualified_name) }?,
        plugin_version: unsafe { copy_static_utf8(path, plugin.plugin_version) }?,
        dependencies,
        factory: FactoryInspection {
            root_factory_id: plugin.root_factory.iid,
            root_factory_signature,
            root_factory_threading,
        },
    })
}

/// Opens and validates a descriptor retained for a synchronous C callback.
///
/// # Safety
///
/// `path` must identify trusted plugin code. Every pointer in its descriptor
/// must obey the immutable metadata lifetime promised by the C entrypoint.
pub(crate) unsafe fn inspect_factory_plugin_description(path: &Path) -> Result<*const NcFactoryPluginDescription, PluginLoadError> {
    let module = ManuallyDrop::new(LoadedModule::open(path)?);
    // SAFETY: The caller trusts the pinned module and its versioned C symbol.
    let get = unsafe { module.get_factory_plugin_description(path)? };
    // SAFETY: The pinned library keeps the returned descriptor alive.
    let plugin = unsafe { factory_plugin_description_from_entrypoint(get, path)? };
    // SAFETY: The entrypoint promises complete immutable metadata storage.
    let status = unsafe { validate_factory_plugin_description(plugin) };
    if status.is_err() {
        return Err(PluginLoadError::new(
            PluginLoadErrorKind::Metadata,
            path,
            Some(status),
            "invalid factory plugin description",
        ));
    }
    Ok(plugin)
}

/// Copies native text so an inspection result exposes no foreign borrow.
///
/// # Safety
///
/// `value` must refer to readable immutable bytes for this synchronous copy.
unsafe fn copy_static_utf8(path: &Path, value: NcStaticUtf8) -> Result<String, PluginLoadError> {
    // SAFETY: The caller establishes readability; the helper validates UTF-8.
    unsafe { value.as_str() }
        .map(str::to_owned)
        .map_err(|status| PluginLoadError::new(PluginLoadErrorKind::Metadata, path, Some(status), "invalid plugin text"))
}
