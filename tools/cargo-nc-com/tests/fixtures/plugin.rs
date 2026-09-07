//! Small valid descriptors used to exercise real native loading and graph planning.

use nc_com_core::{NcClassTable, NcGuid, NcPluginDependencyDescriptor, NcPluginDescriptor, NcStaticUtf8, NcStatus};

/// Stable fixture provider identity shared by the two independently compiled plugins.
const PROVIDER: NcGuid = nc_com_core::guid_from_qualified_name("nc.tests.provider");
/// Valid empty class table; this fixture tests deployment rather than object activation.
static CLASSES: NcClassTable = NcClassTable { class_count: 0, classes: core::ptr::null() };
/// Hard dependency that forces the provider to precede its consumer in plugins.txt.
static DEPENDENCIES: [NcPluginDependencyDescriptor; 1] = [NcPluginDependencyDescriptor { plugin_id: PROVIDER }];
/// Identity deliberately absent from the assembly, selected only for failure testing.
static MISSING: [NcPluginDependencyDescriptor; 1] = [NcPluginDependencyDescriptor { plugin_id: NcGuid::from_fields(99, 0, 0, [0; 8]) }];
/// Normal descriptor; feature selection gives each package a distinct plugin identity.
static PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
    core_abi_version: nc_com_core::NC_COM_CORE_ABI_VERSION,
    plugin_id: nc_com_core::guid_from_qualified_name(concat!("nc.tests.", env!("CARGO_PKG_NAME"))),
    qualified_name: NcStaticUtf8::from_static(concat!("nc.tests.", env!("CARGO_PKG_NAME"))),
    plugin_version: NcStaticUtf8::from_static("0.1.0"),
    dependency_count: if cfg!(feature = "dependent") { 1 } else { 0 },
    dependencies: DEPENDENCIES.as_ptr(),
    class_table: &CLASSES,
};
/// Structurally valid descriptor whose unsatisfied dependency must abort assembly.
static BROKEN: NcPluginDescriptor = NcPluginDescriptor {
    dependency_count: 1,
    dependencies: MISSING.as_ptr(),
    ..PLUGIN
};

/// Exports the fixture descriptor using the nc-com C ABI.
///
/// # Safety
/// A nonnull `out` must point to writable storage for one descriptor pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nc_get_plugin_descriptor(out: *mut *const NcPluginDescriptor) -> NcStatus {
    if out.is_null() { return nc_com_core::NC_E_POINTER; }
    let descriptor = if std::env::var_os("NC_COM_FIXTURE_MISSING_DEPENDENCY").is_some() { &BROKEN } else { &PLUGIN };
    // SAFETY: The caller supplied writable pointer storage and static metadata outlives it.
    unsafe { out.write(descriptor); }
    nc_com_core::NC_OK
}
