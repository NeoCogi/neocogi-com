//! Independently compiled provider exercising nested access to the host registry.
extern crate self as nc_registry_peer;
mod contracts { include!(env!("IDL_EXAMPLE_BINDINGS")); }

use neocogi_com::{ComPtr, NcStatus, Runtime, guid_from_qualified_name};
use contracts::ExampleFactoryAbi;

/// Required provider A is admitted by the host before publishing this peer.
const SAMPLE: neocogi_com::NcGuid = guid_from_qualified_name("nc.examples.factory_plugin_component");

/// Acquiring B publishes a marker in the supplied host and acquires A through
/// that same access. Returning A's owned root also exercises cross-DSO transfer.
#[neocogi_com::plugin(name = "nc.tests.registry-peer", dependencies = [SAMPLE])]
fn acquire(runtime: Runtime) -> Result<ComPtr<ExampleFactoryAbi>, NcStatus> {
    let before = runtime.plugins()?.len();
    let new_marker = usize::from(runtime.plugin_info(MARKER.plugin_id)?.is_none());
    // SAFETY: the host has pinned this DLL before calling its bootstrap. The
    // marker and every reachable field/function share that process-lived image.
    unsafe { runtime.register_plugin_descriptor(&MARKER) }?;
    assert_eq!(runtime.plugins()?.len(), before + new_marker);
    runtime.acquire_plugin_root(SAMPLE)
}

/// Metadata publication is separate from acquisition; the host observes this
/// only after entering B, and B keeps no cached/invented runtime implementation.
static MARKER: neocogi_com::NcPluginDescriptor = neocogi_com::NcPluginDescriptor {
    plugin_id: guid_from_qualified_name("nc.tests.registry-peer.marker"),
    qualified_name: neocogi_com::NcStaticUtf8::from_static("nc.tests.registry-peer.marker"),
    ..*plugin_descriptor()
};
