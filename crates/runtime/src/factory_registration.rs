//! Atomic publication of self-contained factory plugin descriptions.
//!
//! Metadata validation and dependency copying happen before the catalog lock is acquired.
//! The exclusive phase checks dependencies and plugin identity, then publishes
//! the plugin record and its lookup index together. Contract compatibility belongs to typed factory
//! acquisition and interface transfers, so registration never traverses IDL.

use neocogi_com_core::{NC_E_INVALID_ARG, NC_OK, NcFactoryPluginDescription, NcGuid, NcStatus, validate_factory_plugin_description};

use crate::factory_plugin_registry::RegisteredFactoryPlugin;
use crate::state::{RuntimeCatalog, RuntimeState};

/// Validates one descriptor and copies the data needed for atomic publication.
///
/// # Safety
///
/// `plugin` must be null or point to a complete immutable descriptor whose
/// strings, signature bytes, dependency table, and code remain live for the
/// process-wide registration lifetime.
pub(crate) unsafe fn prepare_factory_plugin(plugin: *const NcFactoryPluginDescription) -> Result<RegisteredFactoryPlugin, NcStatus> {
    // SAFETY: The caller provides the descriptor and every referenced lifetime.
    let status = unsafe { validate_factory_plugin_description(plugin) };
    if status.is_err() {
        return Err(status);
    }
    // SAFETY: Validation rejected null and checked the descriptor generation.
    let description = unsafe { &*plugin };
    // SAFETY: Validation established the dependency table's pointer/count shape.
    let dependency_ids = unsafe { description.dependencies() }?.iter().map(|dependency| dependency.plugin_id).collect();
    Ok(RegisteredFactoryPlugin {
        plugin_id: description.plugin_id,
        description: plugin,
        dependency_ids,
    })
}

impl RuntimeState {
    /// Registers one plugin after validating metadata outside the catalog lock.
    ///
    /// # Safety
    ///
    /// The descriptor and everything it references must remain readable and
    /// callable for the process-wide registry lifetime.
    pub(crate) unsafe fn register_factory_plugin_description(&self, plugin: *const NcFactoryPluginDescription) -> NcStatus {
        // SAFETY: The caller supplies the immutable metadata and code lifetime.
        let batch = match unsafe { prepare_factory_plugin(plugin) } {
            Ok(batch) => batch,
            Err(status) => return status,
        };
        self.write_catalog().commit_factory_plugin_registration(batch)
    }
}

impl RuntimeCatalog {
    /// Checks every state-dependent rule before publishing any plugin indexes.
    pub(crate) fn commit_factory_plugin_registration(&mut self, plugin: RegisteredFactoryPlugin) -> NcStatus {
        if let Err(status) = self.validate_plugin_dependencies(&plugin.dependency_ids) {
            return status;
        }
        match self.factory_plugin_registry.plugin(plugin.plugin_id) {
            Some(existing) if existing.description == plugin.description => return NC_OK,
            Some(_) => return NC_E_INVALID_ARG,
            None => {}
        }
        // Validation derives the ID from the normative name. The existing
        // plugin index therefore detects reused names and IDs together; a
        // second pair of name/ID maps would only duplicate that authority.
        self.factory_plugin_registry.insert(plugin);
        NC_OK
    }
}

#[cfg(test)]
mod tests {
    use core::ptr;
    use neocogi_com_core::{
        IUnknown, NC_ABI_THREADING_THREAD_LOCAL, NC_COM_CORE_ABI_VERSION, NC_FACTORY_PLUGIN_ABI_VERSION, NC_E_NOT_FOUND, NcAbiSignature, NcInterfaceId,
        NcPluginDependencyDescriptor, NcStaticUtf8, guid_from_qualified_name,
    };
    use super::*;

    /// Metadata-only fixture: registration must never call the bootstrap.
    unsafe extern "C" fn activate_stub(_out: *mut *mut IUnknown) -> NcStatus {
        panic!("registration must not invoke plugin code")
    }

    /// Self-contained plugin description with no contract-module dependencies.
    static PLUGIN: NcFactoryPluginDescription = NcFactoryPluginDescription {
        description_abi_version: NC_FACTORY_PLUGIN_ABI_VERSION,
        core_abi_version: NC_COM_CORE_ABI_VERSION,
        plugin_id: guid_from_qualified_name("nc.tests.factory_plugin"),
        qualified_name: NcStaticUtf8::from_static("nc.tests.factory_plugin"),
        plugin_version: NcStaticUtf8::from_static("1.0.0"),
        dependency_count: 0,
        dependencies: ptr::null(),
        root_factory: NcInterfaceId::new(guid_from_qualified_name("nc.tests.Factory"), NcAbiSignature::from_static(&[5; 32])),
        root_factory_threading: NC_ABI_THREADING_THREAD_LOCAL,
        activate: Some(activate_stub),
    };

    /// Publication and repeated registration share the same immutable identity.
    #[test]
    fn registration_is_atomic_and_idempotent_without_constructing_a_factory() {
        let runtime = RuntimeState::new();
        // SAFETY: The fixture and its referenced signature are immutable statics.
        assert_eq!(unsafe { runtime.register_factory_plugin_description(&PLUGIN) }, NC_OK);
        assert_eq!(unsafe { runtime.register_factory_plugin_description(&PLUGIN) }, NC_OK);
        let catalog = runtime.read_catalog();
        assert!(catalog.factory_plugin_registry.contains_plugin(PLUGIN.plugin_id));
        assert_eq!(catalog.factory_plugin_registry.with_plugin_ids(|ids| ids.len()), 1);
    }

    /// A missing provider must leave both plugin and name indexes unchanged.
    #[test]
    fn missing_dependency_does_not_publish_partial_registration() {
        static DEPENDENCIES: [NcPluginDependencyDescriptor; 1] = [NcPluginDependencyDescriptor {
            plugin_id: guid_from_qualified_name("nc.tests.missing"),
        }];
        let plugin = NcFactoryPluginDescription {
            dependency_count: 1,
            dependencies: DEPENDENCIES.as_ptr(),
            ..PLUGIN
        };
        let runtime = RuntimeState::new();
        // SAFETY: Failed validation retains no pointer to this stack descriptor.
        assert_eq!(unsafe { runtime.register_factory_plugin_description(&plugin) }, NC_E_NOT_FOUND);
        let catalog = runtime.read_catalog();
        assert!(!catalog.factory_plugin_registry.contains_plugin(PLUGIN.plugin_id));
        assert!(catalog.factory_plugin_registry.with_plugin_ids(|ids| ids.is_empty()));
    }
}
