//! Real host-table admission and singleton visibility regressions.

use core::{mem, ptr};

use neocogi_com_core::{
    ComRaw, IUnknown, NC_ABI_THREADING_THREAD_LOCAL, NC_COM_CORE_ABI_VERSION, NC_COM_RUNTIME_ABI_VERSION, NC_E_ABI_VERSION, NC_E_CLOSED, NC_E_POINTER, NC_OK,
    NC_PLUGIN_DESCRIPTOR_ABI_VERSION, NcPersistenceApiV1, NcPluginDescriptor, NcPluginInfoV1, NcStaticUtf8, NcStatus, guid_from_qualified_name,
    validate_host_api,
};

use super::{HOST_API, PERSISTENCE_API_V1, RUNTIME_API_V1};

/// Descriptor-only fixture; admission and lookup must never acquire a factory.
unsafe extern "C" fn unused_acquisition(_host: *const neocogi_com_core::NcHostApiV1, out: *mut ComRaw<IUnknown>) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: A bootstrap caller grants one writable output pair.
    unsafe { out.write(ComRaw::null()) };
    NC_E_CLOSED
}

/// Process-lived metadata with a distinct identity for shared-state observation.
static DESCRIPTION: NcPluginDescriptor = NcPluginDescriptor {
    description_abi_version: NC_PLUGIN_DESCRIPTOR_ABI_VERSION,
    core_abi_version: NC_COM_CORE_ABI_VERSION,
    plugin_id: guid_from_qualified_name("nc.tests.host_api.visibility"),
    qualified_name: NcStaticUtf8::from_static("nc.tests.host_api.visibility"),
    plugin_version: NcStaticUtf8::from_static("1.0.0"),
    dependency_count: 0,
    dependencies: ptr::null(),
    root_factory: IUnknown::INTERFACE_ID,
    root_factory_threading: NC_ABI_THREADING_THREAD_LOCAL,
    acquire: Some(unused_acquisition),
};

/// Copies a complete valid table into local storage for prefix-only rejection.
fn persistence_with_core(core_abi_version: u32) -> NcPersistenceApiV1 {
    NcPersistenceApiV1 {
        size: mem::size_of::<NcPersistenceApiV1>(),
        core_abi_version,
        create_context: PERSISTENCE_API_V1.create_context,
        destroy_context: PERSISTENCE_API_V1.destroy_context,
        register_factory: PERSISTENCE_API_V1.register_factory,
        snapshot: PERSISTENCE_API_V1.snapshot,
        restore: PERSISTENCE_API_V1.restore,
    }
}

#[test]
fn initialized_host_tables_share_registration_and_keep_baseline_one() {
    let first = crate::initialize_process();
    let second = crate::initialize_process();
    assert_eq!(NC_COM_RUNTIME_ABI_VERSION, 1);
    assert_eq!(NC_COM_CORE_ABI_VERSION, 1);
    assert_eq!(NC_PLUGIN_DESCRIPTOR_ABI_VERSION, 1);
    // SAFETY: The producer guarantees complete immutable process-lived tables.
    assert_eq!(unsafe { validate_host_api(first) }, NC_OK);
    // SAFETY: The host supplies live tables and the descriptor's entire graph is static.
    let status = unsafe { ((*first.runtime).register_plugin_descriptor)(&DESCRIPTION) };
    assert_eq!(status, NC_OK);
    let mut info = mem::MaybeUninit::<NcPluginInfoV1>::uninit();
    // SAFETY: A second access description refers to the same host state and gets one output.
    let status = unsafe { ((*second.runtime).plugin_info)(DESCRIPTION.plugin_id, info.as_mut_ptr()) };
    assert_eq!(status, NC_OK);
    // SAFETY: Successful lookup initialized the complete result.
    let info = unsafe { info.assume_init() };
    assert_eq!(info.plugin_id, DESCRIPTION.plugin_id);
    assert_eq!(info.factory.interface, IUnknown::INTERFACE_ID);
}

#[test]
fn host_admission_rejects_null_and_short_outer_prefixes() {
    // SAFETY: Null is an admitted rejection input.
    assert_eq!(unsafe { validate_host_api(ptr::null()) }, NC_E_POINTER);
    let short = mem::size_of::<usize>();
    // SAFETY: Only the aligned leading usize is available/advertised; it must
    // reject without reading versions or creating a full-record reference.
    assert_eq!(unsafe { validate_host_api(ptr::from_ref(&short).cast()) }, NC_E_ABI_VERSION);
}

#[test]
fn host_admission_checks_versions_before_following_children() {
    let mut host = HOST_API;
    host.runtime = ptr::null();
    host.runtime_abi_version += 1;
    // SAFETY: This complete immutable local record has an unsupported version.
    assert_eq!(unsafe { validate_host_api(&host) }, NC_E_ABI_VERSION);
    host.runtime_abi_version = NC_COM_RUNTIME_ABI_VERSION;
    host.core_abi_version += 1;
    // SAFETY: Unsupported core also rejects before following null children.
    assert_eq!(unsafe { validate_host_api(&host) }, NC_E_ABI_VERSION);
}

#[test]
fn host_admission_requires_all_three_services() {
    for service in 0..3 {
        let mut host = HOST_API;
        match service {
            0 => host.runtime = ptr::null(),
            1 => host.debug = ptr::null(),
            2 => host.persistence = ptr::null(),
            _ => unreachable!(),
        }
        // SAFETY: The local host record and all nonnull backing are valid here.
        assert_eq!(unsafe { validate_host_api(&host) }, NC_E_POINTER);
    }
}

#[test]
fn host_admission_reads_only_available_child_prefixes() {
    let short = mem::size_of::<usize>();
    for service in 0..3 {
        let mut host = HOST_API;
        match service {
            0 => host.runtime = ptr::from_ref(&short).cast(),
            1 => host.debug = ptr::from_ref(&short).cast(),
            2 => host.persistence = ptr::from_ref(&short).cast(),
            _ => unreachable!(),
        }
        // SAFETY: The selected child advertises only its aligned size field;
        // other fields and complete function-pointer records cannot be read.
        assert_eq!(unsafe { validate_host_api(&host) }, NC_E_ABI_VERSION);
    }
}

#[test]
fn host_admission_rejects_inconsistent_service_core_versions() {
    let runtime = neocogi_com_core::NcRuntimeApiV1 {
        core_abi_version: NC_COM_CORE_ABI_VERSION + 1,
        ..RUNTIME_API_V1
    };
    let mut host = HOST_API;
    host.runtime = &runtime;
    // SAFETY: Complete local table remains valid during this nonretaining check.
    assert_eq!(unsafe { validate_host_api(&host) }, NC_E_ABI_VERSION);
    let persistence = persistence_with_core(NC_COM_CORE_ABI_VERSION + 1);
    host.runtime = HOST_API.runtime;
    host.persistence = &persistence;
    // SAFETY: Complete local table remains valid during this nonretaining check.
    assert_eq!(unsafe { validate_host_api(&host) }, NC_E_ABI_VERSION);
}
