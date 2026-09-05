//! Read-only projections over plugin and type registry state.
//!
//! Projection functions translate runtime-owned records into small C ABI views
//! without allocating or changing registration state. Static descriptor text is
//! exposed as borrowed UTF-8 slices; relationship IDs preserve their registry
//! insertion order.
//!
//! These helpers build C ABI records without owning a second metadata catalog.
//! Descriptor text points into immutable plugin metadata; module paths point
//! into append-only loaded-module records. The C callback copies each record
//! while the referenced runtime storage remains live.

#[cfg(test)]
use std::vec::Vec;

use nc_com_core::{
    NcClassInfoV1, NcGuid, NcModuleInfoV1, NcPluginInfoV1, NcStaticUtf8, NcUtf8View,
};
#[cfg(test)]
use nc_com_core::{NcInterfaceId, NcStatus};

use crate::loader::LoadedModuleId;
use crate::plugin_registry::PluginRegistry;
use crate::type_registry::TypeRegistry;

/// Projects one registered plugin descriptor into a borrowed ABI summary.
pub(crate) fn plugin_info(registry: &PluginRegistry, plugin_id: NcGuid) -> Option<NcPluginInfoV1> {
    let registered = registry.plugin(plugin_id)?;
    // SAFETY: Registered plugin pointers refer to validated immutable
    // descriptors that remain live while registered.
    let plugin = unsafe { &*registered.plugin };
    Some(NcPluginInfoV1 {
        plugin_id: registered.plugin_id,
        qualified_name: static_utf8_view(plugin.qualified_name),
        version: static_utf8_view(plugin.plugin_version),
    })
}

/// Projects one retained module record into its path and plugin provenance.
pub(crate) fn module_info(
    registry: &PluginRegistry,
    module_id: LoadedModuleId,
) -> Option<NcModuleInfoV1> {
    let loaded = registry.module(module_id)?;
    Some(NcModuleInfoV1 {
        module_id: loaded.module_id.get(),
        plugin_id: loaded.plugin_id,
        path: loaded.path,
    })
}

/// Projects one registered class into borrowed metadata and module provenance.
pub(crate) fn class_info(registry: &TypeRegistry, class_id: NcGuid) -> Option<NcClassInfoV1> {
    let registered = registry.class(class_id)?;
    // SAFETY: Registered class pointers refer to validated immutable
    // descriptors that remain live while registered.
    let class = unsafe { &*registered.class };
    Some(NcClassInfoV1 {
        class_id: registered.class_id,
        plugin_id: registered.plugin_id,
        module_id: registered.module_id.map_or(0, LoadedModuleId::get),
        qualified_name: static_utf8_view(class.qualified_name),
        version: static_utf8_view(class.class_version),
        constructor_args: class.constructor_args,
        threading: class.threading,
    })
}

#[cfg(test)]
/// Returns class IDs registered by one plugin in registration order.
pub(crate) fn plugin_class_ids(registry: &TypeRegistry, plugin_id: NcGuid) -> Vec<NcGuid> {
    registry.plugin_class_ids(plugin_id)
}

#[cfg(test)]
/// Returns hard dependency IDs copied from one registered plugin descriptor.
pub(crate) fn plugin_dependency_ids(
    registry: &PluginRegistry,
    plugin_id: NcGuid,
) -> Result<Vec<NcGuid>, NcStatus> {
    Ok(registry.with_plugin_dependency_ids(plugin_id, <[NcGuid]>::to_vec))
}

#[cfg(test)]
/// Returns canonical interface identities exposed by one registered class.
pub(crate) fn class_interface_ids(
    registry: &TypeRegistry,
    class_id: NcGuid,
) -> Result<Vec<NcInterfaceId>, NcStatus> {
    // Relationship count APIs use zero for both an absent class and a class
    // with no interfaces; record lookup is the operation that distinguishes
    // absence with NC_E_NOT_FOUND.
    Ok(registry.class_interface_ids(class_id).unwrap_or_default())
}

#[cfg(test)]
/// Returns class IDs implementing an interface in registration order.
pub(crate) fn interface_implementor_class_ids(
    registry: &TypeRegistry,
    interface_id: NcInterfaceId,
) -> Result<Vec<NcGuid>, NcStatus> {
    Ok(registry.interface_implementor_class_ids(interface_id))
}

/// Reinterprets validated static descriptor text as a borrowed runtime view.
fn static_utf8_view(value: NcStaticUtf8) -> NcUtf8View {
    NcUtf8View {
        data: value.data,
        len: value.len as usize,
    }
}
