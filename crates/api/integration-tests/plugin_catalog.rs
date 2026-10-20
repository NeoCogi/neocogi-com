//! Complete raw/facade catalog snapshots and process-lived borrowed storage.

use core::{mem::MaybeUninit, ptr};
use std::sync::atomic::{AtomicUsize, Ordering};
use neocogi_com::*;

/// Counts bootstrap calls so metadata operations cannot silently acquire objects.
static ACQUIRES: AtomicUsize = AtomicUsize::new(0);
/// Returns a distinctive ordinary acquisition error without transferring ownership.
unsafe extern "C" fn acquire(out: *mut ComRaw<IUnknown>) -> NcStatus {
    unsafe { out.write(ComRaw::null()) };
    ACQUIRES.fetch_add(1, Ordering::SeqCst);
    NC_E_CLOSED
}
/// Allocates immutable process-lifetime metadata, mirroring a pinned descriptor.
fn description(name: &'static str, dependencies: &'static [NcPluginDependencyDescriptor]) -> &'static NcPluginDescriptor {
    Box::leak(Box::new(NcPluginDescriptor {
        description_abi_version: NC_PLUGIN_DESCRIPTOR_ABI_VERSION,
        core_abi_version: NC_COM_CORE_ABI_VERSION,
        plugin_id: guid_from_qualified_name(name),
        qualified_name: NcStaticUtf8::from_static(name),
        plugin_version: NcStaticUtf8::from_static("0.1.0"),
        dependency_count: dependencies.len() as u32,
        dependencies: dependencies.as_ptr(),
        root_factory: IUnknown::INTERFACE_ID,
        root_factory_threading: NC_ABI_THREADING_THREAD_LOCAL,
        acquire: Some(acquire),
    }))
}
unsafe extern "C" {
    /// Actual linked C table, exercised in shared and static configurations.
    fn nc_com_get_runtime_api_v1() -> *const NcRuntimeApiV1;
}

/// A separate integration executable starts with an empty process registry.
#[test]
fn complete_records_are_coherent_owned_and_stable_across_growth() {
    let runtime = Runtime::global();
    assert_eq!(runtime.runtime_abi_version(), 1);
    let api = unsafe { &*nc_com_get_runtime_api_v1() };
    let mut len = usize::MAX;
    assert_eq!(unsafe { (api.plugin_infos)(ptr::null_mut(), 0, &mut len) }, NC_OK);
    assert_eq!(len, 0);
    assert!(runtime.plugins().unwrap().is_empty());

    let provider = description("nc.tests.catalog.provider", &[]);
    let second = description("nc.tests.catalog.second", &[]);
    let deps = Box::leak(Box::new([
        NcPluginDependencyDescriptor { plugin_id: second.plugin_id },
        NcPluginDependencyDescriptor { plugin_id: provider.plugin_id },
    ]));
    let consumer = description("nc.tests.catalog.consumer", deps);
    // SAFETY: Descriptors and all reachable metadata/code remain immutable and live.
    assert_eq!(unsafe { runtime.register_plugin_descriptor(consumer) }, Err(NC_E_NOT_FOUND));
    unsafe {
        runtime.register_plugin_descriptor(provider).unwrap();
        runtime.register_plugin_descriptor(second).unwrap();
    }
    unsafe {
        runtime.register_plugin_descriptor(consumer).unwrap();
    }
    unsafe {
        runtime.register_plugin_descriptor(consumer).unwrap();
    }
    assert_eq!(unsafe { (api.plugin_count)() }, 3);
    let mut raw = MaybeUninit::<NcPluginInfoV1>::uninit();
    assert_eq!(unsafe { (api.plugin_info)(consumer.plugin_id, raw.as_mut_ptr()) }, NC_OK);
    let saved = unsafe { raw.assume_init() };
    let mut out = [saved; 4];
    assert_eq!(unsafe { (api.plugin_infos)(out.as_mut_ptr(), 1, &mut len) }, NC_E_INSUFFICIENT_BUFFER);
    assert_eq!(len, 3);
    assert_eq!(out, [saved; 4]);
    assert_eq!(unsafe { (api.plugin_infos)(out.as_mut_ptr(), 4, ptr::null_mut()) }, NC_E_POINTER);
    assert_eq!(out, [saved; 4]);
    assert_eq!(unsafe { (api.plugin_infos)(ptr::null_mut(), 3, &mut len) }, NC_E_POINTER);
    assert_eq!(unsafe { (api.plugin_infos)(out.as_mut_ptr(), 4, &mut len) }, NC_OK);
    assert_eq!(out[0].plugin_id, provider.plugin_id);
    assert_eq!(out[1].plugin_id, second.plugin_id);
    assert_eq!(out[2], saved);
    assert_eq!(out[3], saved); // Capacity beyond the initialized prefix is untouched.
    assert_eq!(unsafe { (api.plugin_info)(NcGuid::nil(), &mut out[0]) }, NC_E_NOT_FOUND);
    assert_eq!(out[0].plugin_id, provider.plugin_id);
    assert_eq!(runtime.plugin_info(NcGuid::nil()).unwrap(), None);

    let mut owned = runtime.plugin_info(consumer.plugin_id).unwrap().unwrap();
    assert_eq!(owned.dependencies, [second.plugin_id, provider.plugin_id]);
    assert_eq!(owned.version, "0.1.0");
    assert_eq!(owned.factory.interface, IUnknown::INTERFACE_ID);
    assert_eq!(owned.factory.threading, NcAbiThreadingMode::ThreadLocal);
    assert_eq!(owned.origin, PluginOrigin::Static);
    let snapshot = runtime.plugins().unwrap();
    assert_eq!(snapshot[2], owned);
    owned.dependencies.clear();
    owned.qualified_name.clear();
    owned.version.clear();
    assert_eq!(runtime.plugin_info(consumer.plugin_id).unwrap().unwrap(), snapshot[2]);
    for index in 0..64 {
        let name = Box::leak(format!("nc.tests.catalog.growth{index}").into_boxed_str());
        unsafe {
            runtime.register_plugin_descriptor(description(name, &[])).unwrap();
        }
    }
    // These old pointers remain readable after the record vector has reallocated.
    assert_eq!(unsafe { *saved.dependencies }, second.plugin_id);
    assert_eq!(
        unsafe { core::str::from_utf8_unchecked(core::slice::from_raw_parts(saved.qualified_name.data, saved.qualified_name.len)) },
        "nc.tests.catalog.consumer"
    );
    let current = runtime.plugins().unwrap();
    assert_eq!(current.len(), 67);
    for plugin in current {
        assert_eq!(Some(plugin.clone()), runtime.plugin_info(plugin.plugin_id).unwrap());
    }
    assert_eq!(ACQUIRES.load(Ordering::SeqCst), 0);
    assert_eq!(runtime.acquire_registered_plugin_root(consumer.plugin_id).unwrap_err(), NC_E_CLOSED);
    assert_eq!(ACQUIRES.load(Ordering::SeqCst), 1);
}

// This final test executable owns its isolated process runtime.
extern crate nc_com as _;
