//! Real host-table admission and singleton visibility regressions.

use core::{mem, ptr};

use neocogi_com::{
    ComRaw, IUnknown, NC_ABI_THREADING_THREAD_LOCAL, NC_COM_ABI_VERSION, NC_E_ABI_VERSION, NC_E_CLOSED, NC_E_POINTER, NC_OK, NcHostApi, NcPluginDescriptor,
    NcPluginInfo, NcStaticUtf8, NcStatus, guid_from_qualified_name, validate_host_api,
};

use super::HOST_API;

/// Descriptor-only fixture; admission and lookup must never acquire a factory.
unsafe extern "C" fn unused_acquisition(_host: *const neocogi_com::NcHostApi, out: *mut ComRaw<IUnknown>) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: A bootstrap caller grants one writable output pair.
    unsafe { out.write(ComRaw::null()) };
    NC_E_CLOSED
}

/// Process-lived metadata with a distinct identity for shared-state observation.
static DESCRIPTION: NcPluginDescriptor = NcPluginDescriptor {
    abi_version: NC_COM_ABI_VERSION,
    plugin_id: guid_from_qualified_name("nc.tests.host_api.visibility"),
    qualified_name: NcStaticUtf8::from_static("nc.tests.host_api.visibility"),
    plugin_version: NcStaticUtf8::from_static("1.0.0"),
    dependency_count: 0,
    dependencies: ptr::null(),
    root_factory: IUnknown::INTERFACE_ID,
    root_factory_threading: NC_ABI_THREADING_THREAD_LOCAL,
    acquire: Some(unused_acquisition),
};

#[test]
fn initialized_host_tables_share_registration() {
    let first = crate::initialize_process();
    let second = crate::initialize_process();
    // SAFETY: The producer guarantees complete immutable process-lived tables.
    assert_eq!(unsafe { validate_host_api(first) }, NC_OK);
    // SAFETY: The host supplies live tables and the descriptor's entire graph is static.
    let status = unsafe { ((*first.runtime).register_plugin_descriptor)(&DESCRIPTION) };
    assert_eq!(status, NC_OK);
    let mut info = mem::MaybeUninit::<NcPluginInfo>::uninit();
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
fn host_admission_checks_the_abi_before_following_children() {
    let mut host = HOST_API;
    host.runtime = ptr::null();
    host.abi_version += 1;
    // SAFETY: The unsupported ABI version rejects before following null children.
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

/// A bundle advertising the old v4 layout stops at its version fields.
#[test]
fn host_admission_rejects_the_previous_host_prefix() {
    #[repr(C)]
    struct PreviousHostPrefix {
        size: usize,
        family: u32,
        generation: u32,
    }
    let previous = PreviousHostPrefix {
        size: mem::size_of::<usize>() * 4 + 8,
        family: 4,
        generation: 23,
    };
    assert_eq!(previous.size, mem::size_of::<NcHostApi>());
    // SAFETY: The legacy prefix is aligned and initialized through both version
    // words. Its unsupported version requires no child fields to be present.
    assert_eq!(unsafe { validate_host_api(ptr::from_ref(&previous).cast()) }, NC_E_ABI_VERSION);
}

/// Each service's fixed size excludes tables from other native ABI versions.
#[test]
fn host_admission_rejects_oversized_service_prefixes() {
    let mut host = HOST_API;
    // SAFETY: The host supplies complete static children for all three services.
    let old_runtime_size = unsafe { (*host.runtime).size } + mem::size_of::<usize>();
    let old_persistence_size = unsafe { (*host.persistence).size } + mem::size_of::<usize>();
    let oversized_debug_size = unsafe { (*host.debug).size } + mem::size_of::<usize>();

    host.runtime = ptr::from_ref(&old_runtime_size).cast();
    // SAFETY: The advertised wrong size rejects before any callback is read.
    assert_eq!(unsafe { validate_host_api(&host) }, NC_E_ABI_VERSION);
    host.runtime = HOST_API.runtime;
    host.debug = ptr::from_ref(&oversized_debug_size).cast();
    // SAFETY: The advertised wrong size rejects before any callback is read.
    assert_eq!(unsafe { validate_host_api(&host) }, NC_E_ABI_VERSION);
    host.debug = HOST_API.debug;
    host.persistence = ptr::from_ref(&old_persistence_size).cast();
    // SAFETY: The advertised wrong size rejects before any callback is read.
    assert_eq!(unsafe { validate_host_api(&host) }, NC_E_ABI_VERSION);
}
