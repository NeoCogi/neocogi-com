//! Legacy one-argument bootstrap and a separately built current positive control.
//! The legacy descriptor has valid static storage and the exact current factory
//! identity, but must be rejected using its prefix before its callback is read.

use neocogi_com_sdk::{persistence::IObjectChildrenAbi, NcStatus};
#[cfg(feature = "current")]
use neocogi_com_sdk::{persistence::IObjectChildren, ComPtr, IUnknown, NC_E_OUT_OF_RANGE};

/// Makes successful current bootstrap observable independently of a returned root.
fn record_marker(value: &[u8]) {
    let path = std::env::var_os("NC_GENERATION_BOOTSTRAP_MARKER").expect("parent supplies bootstrap marker");
    std::fs::write(path, value).expect("write bootstrap marker");
}

/// Ordinary provider-local object for the compatible control.
#[cfg(feature = "current")]
#[neocogi_com_sdk::com_object(neocogi_com_sdk::persistence::IObjectChildren)]
struct EmptyChildren;

#[cfg(feature = "current")]
impl IObjectChildren for EmptyChildren {
    fn count(&self) -> Result<u32, NcStatus> {
        Ok(0)
    }
    fn get(&self, _index: u32) -> Result<ComPtr<IUnknown>, NcStatus> {
        Err(NC_E_OUT_OF_RANGE)
    }
}

/// The normal generated adapter validates the supplied current host before here.
#[cfg(feature = "current")]
#[neocogi_com_sdk::plugin(name = "nc.tests.legacy_bootstrap")]
fn acquire(_runtime: neocogi_com_sdk::RuntimeHandle) -> Result<ComPtr<IObjectChildrenAbi>, NcStatus> {
    record_marker(b"current");
    EmptyChildren::from_value(EmptyChildren).query()
}

#[cfg(not(feature = "current"))]
mod legacy {
    use super::{IObjectChildrenAbi, NcStatus, record_marker};
    use neocogi_com::{
        ComInterface, ComRaw, IUnknown, NcGuid, NcInterfaceId, NcPluginDependencyDescriptor, NcStaticUtf8, NC_E_POINTER, NC_OK, ThreadingModel,
        guid_from_qualified_name,
    };

    /// Stale-prefix rejection fixture with a deliberately retired one-argument trap.
    /// It need not model published callback semantics: no callback may be read.
    #[repr(C)]
    struct LegacyDescriptor {
        /// Current family for core rejection, or the older symbol's family.
        description_abi_version: u32,
        /// Prior core generation, incompatible with development core 22.
        core_abi_version: u32,
        /// Same plugin identity as the compatible control.
        plugin_id: NcGuid,
        /// Process-lived valid policy text.
        qualified_name: NcStaticUtf8,
        /// Application version text, independent of SDK version.
        plugin_version: NcStaticUtf8,
        /// No required peer providers.
        dependency_count: u32,
        /// Empty arrays may use a null pointer.
        dependencies: *const NcPluginDependencyDescriptor,
        /// Exact factory IID and signature match the compatible control.
        root_factory: NcInterfaceId,
        /// Exact factory threading model, not an alternative rejection cause.
        root_factory_threading: u32,
        /// Retired callback signature; no supplied-host argument exists.
        acquire: Option<unsafe extern "C" fn(*mut ComRaw<IUnknown>) -> NcStatus>,
    }

    // SAFETY: All reachable records, strings and code are immutable static data.
    // The loader pins the mapping, including after prefix rejection.
    unsafe impl Sync for LegacyDescriptor {}

    /// Trap only: a supported host must never invoke this incompatible signature.
    /// No output or host pointer is dereferenced even if a regression reaches it.
    unsafe extern "C" fn retired_acquire(_out: *mut ComRaw<IUnknown>) -> NcStatus {
        record_marker(b"legacy");
        std::process::abort()
    }

    /// Full, aligned legacy storage prevents malformed-memory rejection masking
    /// the generation mismatch that must precede any complete-record borrow.
    static DESCRIPTOR: LegacyDescriptor = LegacyDescriptor {
        description_abi_version: if cfg!(feature = "legacy-symbol") {
            1
        } else if cfg!(feature = "legacy-v2-symbol") {
            2
        } else if cfg!(feature = "legacy-v3-symbol") {
            3
        } else {
            neocogi_com::NC_PLUGIN_DESCRIPTOR_ABI_VERSION
        },
        core_abi_version: if cfg!(feature = "legacy-symbol") { 18 } else if cfg!(feature = "legacy-v2-symbol") { 19 } else { 21 },
        plugin_id: guid_from_qualified_name("nc.tests.legacy_bootstrap"),
        qualified_name: NcStaticUtf8::from_static("nc.tests.legacy_bootstrap"),
        plugin_version: NcStaticUtf8::from_static("0.0.0"),
        dependency_count: 0,
        dependencies: core::ptr::null(),
        root_factory: <IObjectChildrenAbi as ComInterface>::INTERFACE_ID,
        root_factory_threading: <<IObjectChildrenAbi as ComInterface>::Threading as ThreadingModel>::ABI_THREADING_MODE.as_raw(),
        acquire: Some(retired_acquire),
    };

    /// Fixed getter ABI returns an opaque pointer with a readable stable prefix.
    ///
    /// # Safety
    /// `out` must be null or writable for one native pointer. The host may read
    /// only the prefix until it has admitted the legacy generation.
    #[cfg_attr(feature = "legacy-symbol", unsafe(export_name = "nc_get_plugin_descriptor_v1"))]
    #[cfg_attr(
        all(feature = "legacy-v2-symbol", not(feature = "legacy-symbol")),
        unsafe(export_name = "nc_get_plugin_descriptor_v2")
    )]
    #[cfg_attr(
        all(feature = "legacy-v3-symbol", not(any(feature = "legacy-symbol", feature = "legacy-v2-symbol"))),
        unsafe(export_name = "nc_get_plugin_descriptor_v3")
    )]
    #[cfg_attr(
        not(any(feature = "legacy-symbol", feature = "legacy-v2-symbol", feature = "legacy-v3-symbol")),
        unsafe(export_name = "nc_get_plugin_descriptor_v4")
    )]
    unsafe extern "C" fn legacy_descriptor(out: *mut *const LegacyDescriptor) -> NcStatus {
        if out.is_null() {
            return NC_E_POINTER;
        }
        // SAFETY: The caller grants one pointer write; storage stays mapped.
        unsafe { out.write(&DESCRIPTOR) };
        NC_OK
    }
}
