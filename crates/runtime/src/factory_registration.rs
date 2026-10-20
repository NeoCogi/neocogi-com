//! Atomic publication of self-contained plugin descriptors.
//!
//! Metadata validation happens before the catalog lock is acquired.
//! The exclusive phase checks dependencies and plugin identity, then publishes
//! the plugin record and its lookup index together. Contract compatibility belongs to typed factory
//! acquisition and interface transfers, so registration never traverses IDL.

use neocogi_com_core::{NC_COM_RUNTIME_MAX_DYNAMIC_PLUGINS, NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE, NC_OK, NcPluginDescriptor, NcStatus};
use crate::descriptor::ValidatedPluginDescriptor;

use crate::factory_plugin_registry::{FactoryPluginRegistry, PluginOrigin, RegisteredFactoryPlugin};
use crate::state::RuntimeState;

/// Validates one descriptor with its actual origin before atomic publication.
///
/// # Safety
///
/// `plugin` must be null or point to a complete immutable descriptor whose
/// strings, signature bytes, dependency table, and code remain live for the
/// process-wide registration lifetime.
pub(crate) unsafe fn prepare_factory_plugin(plugin: *const NcPluginDescriptor, origin: PluginOrigin) -> Result<RegisteredFactoryPlugin, NcStatus> {
    // SAFETY: The caller provides the descriptor and every referenced lifetime.
    let descriptor = unsafe { ValidatedPluginDescriptor::new(plugin) }?;
    Ok(RegisteredFactoryPlugin { descriptor, origin })
}

impl RuntimeState {
    /// Registers one plugin after validating metadata outside the catalog lock.
    ///
    /// # Safety
    ///
    /// The descriptor and everything it references must remain readable and
    /// callable for the process-wide registry lifetime.
    pub(crate) unsafe fn register_plugin_descriptor(&self, plugin: *const NcPluginDescriptor) -> NcStatus {
        // SAFETY: The caller supplies the immutable metadata and code lifetime.
        let batch = match unsafe { prepare_factory_plugin(plugin, PluginOrigin::Static) } {
            Ok(batch) => batch,
            Err(status) => return status,
        };
        self.write_catalog().commit_factory_plugin_registration(batch)
    }
}

impl FactoryPluginRegistry {
    /// Checks every state-dependent rule before publishing any plugin indexes.
    pub(crate) fn commit_factory_plugin_registration(&mut self, plugin: RegisteredFactoryPlugin) -> NcStatus {
        if let Err(status) = self.validate_plugin_dependencies(plugin.descriptor.dependencies()) {
            return status;
        }
        match self.plugin(plugin.descriptor.id()) {
            Some(existing) if existing.descriptor.as_ptr() == plugin.descriptor.as_ptr() => return NC_OK,
            Some(_) => return NC_E_INVALID_ARG,
            None => {}
        }
        // A repeated descriptor has already returned above: count only new
        // dynamic providers, never load attempts or statically registered plugins.
        if matches!(plugin.origin, PluginOrigin::Dynamic { .. }) && self.dynamic_count() >= NC_COM_RUNTIME_MAX_DYNAMIC_PLUGINS {
            return NC_E_OUT_OF_RANGE;
        }
        // Validation derives the ID from the normative name. The existing
        // plugin index therefore detects reused names and IDs together; a
        // second pair of name/ID maps would only duplicate that authority.
        self.insert(plugin);
        NC_OK
    }
}

#[cfg(test)]
mod tests {
    use core::ptr;
    use neocogi_com_core::{
        IUnknown, NC_ABI_THREADING_THREAD_LOCAL, NC_COM_CORE_ABI_VERSION, NC_PLUGIN_DESCRIPTOR_ABI_VERSION, NC_E_NOT_FOUND, NcAbiSignature, NcInterfaceId,
        NcPluginDependencyDescriptor, NcStaticUtf8, guid_from_qualified_name,
    };
    use super::*;

    /// Metadata-only fixture: registration must never call the bootstrap.
    unsafe extern "C" fn acquire_stub(_out: *mut neocogi_com_core::ComRaw<IUnknown>) -> NcStatus {
        panic!("registration must not invoke plugin code")
    }

    /// Self-contained plugin description with no contract-module dependencies.
    static PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
        description_abi_version: NC_PLUGIN_DESCRIPTOR_ABI_VERSION,
        core_abi_version: NC_COM_CORE_ABI_VERSION,
        plugin_id: guid_from_qualified_name("nc.tests.factory_plugin"),
        qualified_name: NcStaticUtf8::from_static("nc.tests.factory_plugin"),
        plugin_version: NcStaticUtf8::from_static("1.0.0"),
        dependency_count: 0,
        dependencies: ptr::null(),
        root_factory: NcInterfaceId::new(guid_from_qualified_name("nc.tests.Factory"), NcAbiSignature::from_static(&[5; 32])),
        root_factory_threading: NC_ABI_THREADING_THREAD_LOCAL,
        acquire: Some(acquire_stub),
    };

    /// Publication and repeated registration share the same immutable identity.
    #[test]
    fn registration_is_atomic_and_idempotent_without_constructing_a_factory() {
        let runtime = RuntimeState::new();
        // SAFETY: The fixture and its referenced signature are immutable statics.
        assert_eq!(unsafe { runtime.register_plugin_descriptor(&PLUGIN) }, NC_OK);
        assert_eq!(unsafe { runtime.register_plugin_descriptor(&PLUGIN) }, NC_OK);
        // The first static origin also survives a later dynamic encounter of
        // the same descriptor without reserving any dynamic-provider slot.
        let repeat = unsafe { prepare_factory_plugin(&PLUGIN, PluginOrigin::Dynamic { path: "later-dynamic-path".into() }) }.unwrap();
        assert_eq!(runtime.write_catalog().commit_factory_plugin_registration(repeat), NC_OK);
        let catalog = runtime.read_catalog();
        assert!(catalog.contains_plugin(PLUGIN.plugin_id));
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog.dynamic_count(), 0);
        assert!(matches!(catalog.plugin(PLUGIN.plugin_id).unwrap().origin, PluginOrigin::Static));
    }

    /// A missing provider must leave both plugin and name indexes unchanged.
    #[test]
    fn missing_dependency_does_not_publish_partial_registration() {
        static DEPENDENCIES: [NcPluginDependencyDescriptor; 1] = [NcPluginDependencyDescriptor {
            plugin_id: guid_from_qualified_name("nc.tests.missing"),
        }];
        let plugin = Box::leak(Box::new(NcPluginDescriptor {
            dependency_count: 1,
            dependencies: DEPENDENCIES.as_ptr(),
            ..PLUGIN
        }));
        let runtime = RuntimeState::new();
        // SAFETY: the descriptor and its dependency table are process-lived.
        assert_eq!(unsafe { runtime.register_plugin_descriptor(plugin) }, NC_E_NOT_FOUND);
        let catalog = runtime.read_catalog();
        assert!(!catalog.contains_plugin(PLUGIN.plugin_id));
        assert_eq!(catalog.len(), 0);
    }

    /// Validates a real descriptor with process-lifetime test metadata, then adds
    /// actual dynamic provenance to isolate catalog accounting from native I/O.
    fn dynamic_fixture(name: String) -> RegisteredFactoryPlugin {
        let name: &'static str = Box::leak(name.into_boxed_str());
        let description = Box::leak(Box::new(NcPluginDescriptor {
            plugin_id: guid_from_qualified_name(name),
            qualified_name: NcStaticUtf8::from_static(name),
            ..PLUGIN
        }));
        // SAFETY: These test allocations and the inherited static fields remain
        // immutable and process-live, just like validated native metadata.
        unsafe { prepare_factory_plugin(description, PluginOrigin::Dynamic { path: std::path::PathBuf::from(name) }) }.unwrap()
    }

    /// Capacity bounds distinct dynamic providers. Repeats preserve their first
    /// origin even at capacity; static providers need no dynamic slot. Identity
    /// conflicts remain errors and every rejected publication leaves the count fixed.
    #[test]
    fn dynamic_capacity_counts_unique_providers_and_preserves_first_origin() {
        let runtime = RuntimeState::new();
        for index in 0..NC_COM_RUNTIME_MAX_DYNAMIC_PLUGINS {
            let plugin = dynamic_fixture(format!("nc.tests.capacity.provider{index}"));
            assert_eq!(runtime.write_catalog().commit_factory_plugin_registration(plugin), NC_OK);
        }
        let id = guid_from_qualified_name("nc.tests.capacity.provider0");
        let description = runtime.read_catalog().plugin(id).unwrap().descriptor.as_ptr();
        // SAFETY: The fixture allocated immutable process-lifetime metadata.
        let repeat = unsafe { prepare_factory_plugin(description, PluginOrigin::Dynamic { path: "alternate-path".into() }) }.unwrap();
        assert_eq!(runtime.write_catalog().commit_factory_plugin_registration(repeat), NC_OK);
        // SAFETY: PLUGIN is immutable static storage.
        assert_eq!(unsafe { runtime.register_plugin_descriptor(&PLUGIN) }, NC_OK);

        let overflow = dynamic_fixture("nc.tests.capacity.overflow".into());
        let overflow_id = overflow.descriptor.id();
        assert_eq!(runtime.write_catalog().commit_factory_plugin_registration(overflow), NC_E_OUT_OF_RANGE);
        let conflict = dynamic_fixture("nc.tests.capacity.provider0".into());
        assert_eq!(runtime.write_catalog().commit_factory_plugin_registration(conflict), NC_E_INVALID_ARG);

        let catalog = runtime.read_catalog();
        assert_eq!(catalog.dynamic_count(), NC_COM_RUNTIME_MAX_DYNAMIC_PLUGINS);
        assert!(!catalog.contains_plugin(overflow_id));
        assert!(matches!(&catalog.plugin(id).unwrap().origin,
            PluginOrigin::Dynamic { path, .. } if path == std::path::Path::new("nc.tests.capacity.provider0")));
    }
}
