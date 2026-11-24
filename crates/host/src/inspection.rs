//! One allocation-free projection for plugin lookup and enumeration.
//! Referenced buffers are immutable heap allocations or static/pinned descriptor
//! storage. Moving the enclosing catalog records never moves these allocations.

use neocogi_com::{NcFactoryInfo, NcPluginInfo, NcStaticUtf8, NcUtf8View};
use crate::plugin_registry::{PluginOrigin, RegisteredPlugin};

/// Borrows a complete record from process-lived registration storage.
pub(crate) fn plugin_info(registered: &RegisteredPlugin) -> NcPluginInfo {
    let plugin = registered.descriptor.get();
    NcPluginInfo {
        plugin_id: plugin.plugin_id,
        qualified_name: static_utf8_view(plugin.qualified_name),
        version: static_utf8_view(plugin.plugin_version),
        library_path: match &registered.origin {
            PluginOrigin::Static => NcUtf8View::EMPTY,
            PluginOrigin::Dynamic { path, .. } => {
                let path = path.to_str().expect("registered library paths are UTF-8");
                NcUtf8View { data: path.as_ptr(), len: path.len() }
            }
        },
        factory: NcFactoryInfo {
            interface: plugin.root_factory,
            threading: plugin.root_factory_threading,
        },
        dependencies: plugin.dependencies,
        dependency_count: plugin.dependency_count as usize,
    }
}

/// Reinterprets validated static descriptor text as a borrowed runtime view.
fn static_utf8_view(value: NcStaticUtf8) -> NcUtf8View {
    NcUtf8View {
        data: value.data,
        len: value.len as usize,
    }
}
