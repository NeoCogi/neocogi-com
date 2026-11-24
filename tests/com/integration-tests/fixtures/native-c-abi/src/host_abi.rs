//! C layout and call-through proof without selecting a runtime implementation.
//!
//! These immutable fixture tables contain valid typed callbacks but no registry.
//! Only count is exercised; every service callback otherwise fails closed.
use neocogi_com_sdk::*;
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
unavailable!(info(id: NcGuid, out: *mut NcPluginInfo));
unavailable!(infos(out: *mut NcPluginInfo, capacity: usize, len: *mut usize));
unavailable!(inspect(path: *const u8, len: usize, callback: Option<NcInspectPluginDescriptorCallback>, data: *mut c_void));
unavailable!(messenger(info: *const NcDebugMessengerCreateInfo, out: *mut u64));
unavailable!(context(out: *mut *mut NcPersistenceContext));
unavailable!(factory(context: *mut NcPersistenceContext, id: NcGuid, factory: ComRaw<persistence::IPersistenceFactoryAbi>));
unavailable!(save_object_tree(object: ComRaw<IUnknown>, limits: NcPersistenceLimits, write: Option<NcPersistenceWriteBytes>, data: *mut c_void));
unavailable!(restore_object_tree(context: *const NcPersistenceContext, bytes: *const u8, len: usize, limits: NcPersistenceLimits, out: *mut ComRaw<IUnknown>));
/// Distinguishes the forwarded host table from a null/erased or swapped argument.
unsafe extern "C" fn count() -> usize {
    17
}
/// No fixture messenger is ever created.
unsafe extern "C" fn destroy_messenger(_handle: u64) {}
/// No fixture context is ever created.
unsafe extern "C" fn destroy_context(_context: *mut NcPersistenceContext) {}
static RUNTIME: NcPluginApi = NcPluginApi {
    size: size_of::<NcPluginApi>(),
    register_plugin_descriptor: register,
    load_plugin_library_utf8: load,
    acquire_factory_plugin: acquire,
    plugin_info: info,
    plugin_count: count,
    plugin_infos: infos,
    inspect_plugin_library_utf8: inspect,
};
static DEBUG: NcDebugApi = NcDebugApi {
    size: size_of::<NcDebugApi>(),
    create_messenger: messenger,
    destroy_messenger,
};
static PERSISTENCE: NcPersistenceApi = NcPersistenceApi {
    size: size_of::<NcPersistenceApi>(),
    create_context: context,
    destroy_context,
    register_factory: factory,
    save_object_tree,
    restore_object_tree,
};
static HOST: NcHostApi = NcHostApi {
    size: size_of::<NcHostApi>(),
    abi_magic: NC_COM_ABI_MAGIC,
    abi_version: NC_COM_ABI_VERSION,
    runtime: &RUNTIME,
    debug: &DEBUG,
    persistence: &PERSISTENCE,
};
/// Counts entries into authored acquisition after the generated host check.
static ACQUISITIONS: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

/// Receives the admitted host from the C fixture and keeps the output empty.
#[neocogi_com_sdk::plugin(name = "nc.tests.native_c_abi_host")]
fn acquire_root(runtime: RuntimeHandle) -> Result<ComPtr<IUnknown>, NcStatus> {
    assert_eq!(runtime.abi_version(), NC_COM_ABI_VERSION);
    ACQUISITIONS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    Err(NC_E_CLOSED)
}

unsafe extern "C" {
    /// Reads independently declared C records and calls the generated adapter.
    fn nc_c_exercise_host_bootstrap(host: *const NcHostApi, plugin: *const NcPluginDescriptor, expected: NcStatus, incompatible: NcStatus) -> i32;
}

/// The C caller passes one complete host bundle and one retired version prefix.
#[test]
fn plain_c_forwards_host_access_to_the_typed_bootstrap() {
    // SAFETY: Both C and Rust borrow the complete static records. The C caller
    // also supplies an aligned, readable version prefix for the retired ABI.
    assert_eq!(
        unsafe { nc_c_exercise_host_bootstrap(ptr::from_ref(&HOST), plugin_descriptor(), NC_E_CLOSED, NC_E_ABI_VERSION) },
        1
    );
    assert_eq!(ACQUISITIONS.load(core::sync::atomic::Ordering::Relaxed), 1);
}
