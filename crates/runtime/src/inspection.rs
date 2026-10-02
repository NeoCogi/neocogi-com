//! Read-only projections over factory-plugin and module registry state.
//!
//! Projection functions translate runtime-owned records into small C ABI views
//! without allocating or changing registration state. Static descriptor text is
//! exposed as borrowed UTF-8 slices; plugin dependency IDs preserve their registry
//! insertion order.
//!
//! These helpers build C ABI records without owning a second metadata catalog.
//! Descriptor text points into immutable plugin metadata; module paths point
//! into append-only loaded-module records. The C callback copies each record
//! while the referenced runtime storage remains live.

use neocogi_com_core::{NcGuid, NcModuleInfoV1, NcPluginFactoryInfoV1, NcPluginInfoV1, NcStaticUtf8, NcUtf8View};

use crate::loader::LoadedModuleId;
use crate::factory_plugin_registry::FactoryPluginRegistry;
use crate::module_registry::ModuleRegistry;

/// Projects one registered factory-only description into the common summary.
pub(crate) fn factory_plugin_info(registry: &FactoryPluginRegistry, plugin_id: NcGuid) -> Option<NcPluginInfoV1> {
    let registered = registry.plugin(plugin_id)?;
    // SAFETY: Registered pointers target validated immutable descriptions kept
    // live by static ownership or a retained dynamic module.
    let plugin = unsafe { &*registered.description };
    Some(NcPluginInfoV1 {
        plugin_id: registered.plugin_id,
        qualified_name: static_utf8_view(plugin.qualified_name),
        version: static_utf8_view(plugin.plugin_version),
    })
}

/// Projects one registered factory-only description into the factory summary.
pub(crate) fn factory_plugin_factory_info(registry: &FactoryPluginRegistry, plugin_id: NcGuid) -> Option<NcPluginFactoryInfoV1> {
    let registered = registry.plugin(plugin_id)?;
    // SAFETY: Registered pointers target validated immutable descriptions kept
    // live by static ownership or a retained dynamic module.
    let plugin = unsafe { &*registered.description };
    Some(NcPluginFactoryInfoV1 {
        plugin_id: registered.plugin_id,
        root_factory: plugin.root_factory,
        root_factory_threading: plugin.root_factory_threading,
    })
}

/// Projects one retained module record into its path and plugin provenance.
pub(crate) fn module_info(registry: &ModuleRegistry, module_id: LoadedModuleId) -> Option<NcModuleInfoV1> {
    let loaded = registry.module(module_id)?;
    Some(NcModuleInfoV1 {
        module_id: loaded.module_id.get(),
        plugin_id: loaded.plugin_id,
        path: loaded.path,
    })
}

/// Reinterprets validated static descriptor text as a borrowed runtime view.
fn static_utf8_view(value: NcStaticUtf8) -> NcUtf8View {
    NcUtf8View {
        data: value.data,
        len: value.len as usize,
    }
}
