//! Immutable readable metadata cases shared by host registration and independently built DSOs.
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use neocogi_com_core::*;

/// Actual factory entries, independent of metadata discovery/inspection operations.
static ACQUISITIONS: AtomicUsize = AtomicUsize::new(0);

/// Reports whether any metadata-only operation accidentally invoked this factory.
pub fn acquisition_count() -> usize {
    ACQUISITIONS.load(SeqCst)
}

/// The first deliberate acquisition returns CLOSED; any earlier implicit entry changes that result.
unsafe extern "C" fn acquire(out: *mut ComRaw<IUnknown>) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: the acquisition caller supplies one writable cleared-result slot.
    unsafe { *out = ComRaw::null() };
    if ACQUISITIONS.fetch_add(1, SeqCst) == 0 {
        NC_E_CLOSED
    } else {
        NC_E_INVALID_ARG
    }
}

/// A distinctive digest proves inspection copies the exact factory bytes.
pub const SIGNATURE: [u8; 32] = [0x5A; 32];
/// Declared order is deliberately different from lexical provider order.
pub static DEPENDENCIES: [NcPluginDependencyDescriptor; 2] = [
    NcPluginDependencyDescriptor {
        plugin_id: guid_from_qualified_name("nc.tests.inspection.z_provider"),
    },
    NcPluginDependencyDescriptor {
        plugin_id: guid_from_qualified_name("nc.tests.inspection.a_provider"),
    },
];
/// Duplicate identities have valid storage but violate descriptor schema.
static DUPLICATES: [NcPluginDependencyDescriptor; 2] = [DEPENDENCIES[0], DEPENDENCIES[0]];
/// Non-UTF8 is stored in valid readable bytes, never an invalid Rust str.
static INVALID_TEXT: [u8; 1] = [0xFF];
/// Common valid metadata; no bootstrap or catalog state is needed to inspect it.
const BASE: NcPluginDescriptor = NcPluginDescriptor {
    description_abi_version: NC_PLUGIN_DESCRIPTOR_ABI_VERSION,
    core_abi_version: NC_COM_CORE_ABI_VERSION,
    plugin_id: guid_from_qualified_name("nc.tests.inspection.fixture"),
    qualified_name: NcStaticUtf8::from_static("nc.tests.inspection.fixture"),
    plugin_version: NcStaticUtf8::from_static("2.3.4"),
    dependency_count: 0,
    dependencies: core::ptr::null(),
    root_factory: NcInterfaceId::new(guid_from_qualified_name("nc.tests.inspection.Factory"), NcAbiSignature::from_static(&SIGNATURE)),
    root_factory_threading: NC_ABI_THREADING_THREAD_LOCAL,
    acquire: Some(acquire),
};

/// Valid and malformed descriptors retain complete immutable storage for every reachable field.
pub static CASES: [NcPluginDescriptor; 10] = [
    BASE,
    NcPluginDescriptor {
        root_factory_threading: NC_ABI_THREADING_THREAD_SAFE,
        dependency_count: 2,
        dependencies: DEPENDENCIES.as_ptr(),
        ..BASE
    },
    NcPluginDescriptor {
        root_factory: NcInterfaceId::new(BASE.root_factory.iid, NcAbiSignature::EMPTY),
        ..BASE
    },
    NcPluginDescriptor { root_factory_threading: u32::MAX, ..BASE },
    NcPluginDescriptor {
        qualified_name: NcStaticUtf8 { data: INVALID_TEXT.as_ptr(), len: 1 },
        ..BASE
    },
    NcPluginDescriptor {
        dependency_count: 1,
        dependencies: core::ptr::null(),
        ..BASE
    },
    NcPluginDescriptor {
        dependency_count: 2,
        dependencies: DUPLICATES.as_ptr(),
        ..BASE
    },
    NcPluginDescriptor {
        core_abi_version: NC_COM_CORE_ABI_VERSION - 1,
        ..BASE
    },
    NcPluginDescriptor {
        plugin_version: NcStaticUtf8::from_static("2\n3"),
        ..BASE
    },
    NcPluginDescriptor { acquire: None, ..BASE },
];
