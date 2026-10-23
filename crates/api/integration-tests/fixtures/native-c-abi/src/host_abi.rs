//! C layout and call-through proof without selecting a runtime implementation.
//!
//! These immutable fixture tables contain valid typed callbacks but no registry.
//! Only count is exercised; every service callback otherwise fails closed.
use neocogi_com::*;
use core::{ffi::c_void, mem::size_of, ptr};

/// Defines typed fail-closed callbacks; no synthetic allocator/registry is hidden here.
macro_rules! unavailable {
    ($name:ident($($arg:ident: $ty:ty),* $(,)?)) => {
        unsafe extern "C" fn $name($($arg: $ty),*) -> NcStatus {
            let _ = ($($arg),*);
            NC_E_CLOSED
        }
    };
}
unavailable!(register(plugin: *const NcPluginDescriptor));
unavailable!(load(path: *const u8, len: usize, id: *mut NcGuid));
unavailable!(acquire(id: NcGuid, interface: NcInterfaceId, threading: u32, out: *mut ComRaw<IUnknown>));
unavailable!(info(id: NcGuid, out: *mut NcPluginInfoV1));
unavailable!(infos(out: *mut NcPluginInfoV1, capacity: usize, len: *mut usize));
unavailable!(inspect(path: *const u8, len: usize, callback: Option<NcInspectPluginDescriptorCallbackV1>, data: *mut c_void));
unavailable!(messenger(info: *const NcDebugMessengerCreateInfoV1, out: *mut u64));
unavailable!(context(out: *mut *mut NcPersistenceContext));
unavailable!(factory(context: *mut NcPersistenceContext, id: NcGuid, factory: ComRaw<persistence::IPersistenceFactoryAbi>));
unavailable!(snapshot(object: ComRaw<IUnknown>, limits: NcPersistenceLimitsV1, write: Option<NcPersistenceWriteBytesV1>, data: *mut c_void));
unavailable!(restore(context: *const NcPersistenceContext, bytes: *const u8, len: usize, limits: NcPersistenceLimitsV1, out: *mut ComRaw<IUnknown>));
/// Distinguishes the forwarded host table from a null/erased or swapped argument.
unsafe extern "C" fn count() -> usize {
    17
}
/// No fixture messenger is ever created.
unsafe extern "C" fn destroy_messenger(_handle: u64) {}
/// No fixture context is ever created.
unsafe extern "C" fn destroy_context(_context: *mut NcPersistenceContext) {}
static RUNTIME: NcRuntimeApiV1 = NcRuntimeApiV1 {
    size: size_of::<NcRuntimeApiV1>(),
    core_abi_version: 1,
    register_plugin_descriptor: register,
    load_plugin_library_utf8: load,
    acquire_factory_plugin: acquire,
    plugin_info: info,
    plugin_count: count,
    plugin_infos: infos,
    inspect_plugin_library_utf8: inspect,
};
static DEBUG: NcDebugApiV1 = NcDebugApiV1 {
    size: size_of::<NcDebugApiV1>(),
    create_messenger: messenger,
    destroy_messenger,
};
static PERSISTENCE: NcPersistenceApiV1 = NcPersistenceApiV1 {
    size: size_of::<NcPersistenceApiV1>(),
    core_abi_version: 1,
    create_context: context,
    destroy_context,
    register_factory: factory,
    snapshot,
    restore,
};
static HOST: NcHostApiV1 = NcHostApiV1 {
    size: size_of::<NcHostApiV1>(),
    runtime_abi_version: 1,
    core_abi_version: 1,
    runtime: &RUNTIME,
    debug: &DEBUG,
    persistence: &PERSISTENCE,
};
/// Admits the exact record forwarded by plain C and publishes no owned reference.
unsafe extern "C" fn bootstrap(host: *const NcHostApiV1, out: *mut ComRaw<IUnknown>) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: C supplies one writable output record.
    unsafe { out.write(ComRaw::null()) };
    // SAFETY: C forwards HOST's complete immutable static graph unchanged.
    let status = unsafe { validate_host_api(host) };
    if status.is_err() {
        return status;
    }
    // SAFETY: size/version admission preceded all current-layout interpretation.
    assert_eq!(unsafe { ((*(*host).runtime).plugin_count)() }, 17);
    NC_E_CLOSED
}
unsafe extern "C" {
    /// Borrows the host record and calls the supplied function using its C typedef.
    fn nc_c_exercise_host_bootstrap(host: *const NcHostApiV1, acquire: NcAcquirePluginFactory, expected: NcStatus) -> i32;
}
/// Size/offset checks are independent of this actual C-to-Rust function call.
#[test]
fn plain_c_forwards_host_access_to_the_typed_bootstrap() {
    // SAFETY: both the C implementation and Rust bootstrap borrow this live graph.
    assert_eq!(unsafe { nc_c_exercise_host_bootstrap(ptr::from_ref(&HOST), bootstrap, NC_E_CLOSED) }, 1);
}
