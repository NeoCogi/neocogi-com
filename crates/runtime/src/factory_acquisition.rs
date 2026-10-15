//! Root-factory lookup and invocation for registered plugins with one typed root.
//!
//! Resolution occurs under the catalog read lock, while plugin code runs only
//! after that guard is released. The runtime validates the returned object's
//! complete factory interface identity before transferring ownership to a host.

use neocogi_com_core::{
    IUnknown, NC_ABI_THREADING_THREAD_LOCAL, NC_ABI_THREADING_THREAD_SAFE, NC_E_NOINTERFACE, NC_E_NOT_FOUND, NC_E_POINTER, NC_OK, NcAcquirePluginFactory,
    NcPluginDescriptor, NcGuid, NcInterfaceId, NcStatus, raw,
};

use crate::factory_plugin_registry::FactoryPluginRegistry;

/// Resolves one compatible acquisition pointer without entering plugin code.
pub(crate) fn resolve_factory(
    registry: &FactoryPluginRegistry,
    plugin_id: NcGuid,
    factory_interface: NcInterfaceId,
    requested_threading: u32,
) -> Result<NcAcquirePluginFactory, NcStatus> {
    let registered = registry.plugin(plugin_id).ok_or(NC_E_NOT_FOUND)?;
    // SAFETY: The registry retains only validated immutable descriptions whose
    // static or mapped-library owner remains live while registered.
    let description: &NcPluginDescriptor = unsafe { &*registered.description };
    if description.root_factory.iid != factory_interface.iid {
        return Err(NC_E_NOINTERFACE);
    }
    // Registration proves the stored signature is readable. Compare the complete
    // contract before invoking plugin bootstrap code, including transitive IDL
    // changes that preserve the factory's nominal IID.
    if description.root_factory.abi_signature != factory_interface.abi_signature {
        return Err(neocogi_com_core::NC_E_ABI_VERSION);
    }
    if !threading_request_is_compatible(description.root_factory_threading, requested_threading) {
        return Err(neocogi_com_core::NC_E_INVALID_ARG);
    }
    description.acquire.ok_or(neocogi_com_core::NC_E_INVALID_ARG)
}

/// Applies the declared factory capability to one host threading request.
fn threading_request_is_compatible(declared: u32, requested: u32) -> bool {
    match requested {
        NC_ABI_THREADING_THREAD_LOCAL => true,
        NC_ABI_THREADING_THREAD_SAFE => declared == NC_ABI_THREADING_THREAD_SAFE,
        _ => false,
    }
}

/// Invokes a resolved plugin root and verifies its exact interface contract.
///
/// # Safety
///
/// `out` must be valid for one ComRaw record write. `acquire` must remain callable
/// through this operation, which runtime registration and module retention
/// guarantee for values returned by [`resolve_factory`].
pub(crate) unsafe fn acquire_factory(
    acquire: Result<NcAcquirePluginFactory, NcStatus>,
    factory_interface: NcInterfaceId,
    out: *mut neocogi_com_core::ComRaw<IUnknown>,
) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: The caller grants one output write and null was rejected above.
    unsafe { *out = neocogi_com_core::ComRaw::null() };
    let acquire = match acquire {
        Ok(acquire) => acquire,
        Err(status) => return status,
    };

    // SAFETY: The registered pointer remains mapped and `out` satisfies its
    // fixed output contract for this synchronous call.
    let status = unsafe { acquire(out) };
    if status.is_err() {
        // A plugin must clear failed outputs. Clear defensively without
        // dereferencing an untrusted value supplied on an error path.
        // SAFETY: `out` was validated above.
        unsafe { *out = neocogi_com_core::ComRaw::null() };
        return status;
    }
    // SAFETY: The output slot remains readable after the plugin returns.
    let root = unsafe { *out };
    if !root.is_present() {
        // Reject both absence and half-null records before any table access.
        // SAFETY: the caller's output slot is still writable.
        unsafe { *out = neocogi_com_core::ComRaw::null() };
        return NC_E_POINTER;
    }
    // SAFETY: acquisition success promises a matching live allocation/table pair.
    // The common query body validates the result and releases rejected transfers.
    let projection = unsafe { raw::__nc_query_interface(root, &factory_interface) };
    // SAFETY: the bootstrap owner is consumed whether acquisition succeeds or fails.
    unsafe { raw::release(root) };
    match projection {
        Ok(projection) => {
            // SAFETY: transfer the exact validated factory view to the caller.
            unsafe { *out = projection };
        }
        Err(status) => {
            // SAFETY: the common query already consumed any rejected reference.
            unsafe { *out = neocogi_com_core::ComRaw::null() };
            return status;
        }
    }
    NC_OK
}

#[cfg(test)]
mod tests {
    use core::ffi::c_void;
    use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

    use neocogi_com_core::{
        IUnknownVTable, NC_ABI_THREADING_THREAD_LOCAL, NC_E_ABI_VERSION, NcAbiSignature, NcPluginDescriptor, NcStaticUtf8, guid_from_qualified_name,
    };

    use super::*;
    use crate::factory_plugin_registry::RegisteredFactoryPlugin;

    /// Stable fake factory interface used by raw acquisition validation.
    const FACTORY_ID: NcGuid = NcGuid::from_fields(1, 2, 3, [4; 8]);
    /// Complete fake interface identity stored in the vtable prefix.
    const FACTORY_INTERFACE: NcInterfaceId = NcInterfaceId::new(FACTORY_ID, NcAbiSignature::from_static(&[7; 32]));
    /// Counts factory allocations destroyed by their final Release.
    static FACTORY_DROPS: AtomicUsize = AtomicUsize::new(0);
    /// Serializes fixture counters so the parallel test harness cannot reset peers.
    static FIXTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    /// Records entry into the bootstrap before it can allocate an object.
    static FACTORY_ACQUISITIONS: AtomicUsize = AtomicUsize::new(0);

    /// Minimal owned COM storage used to exercise acquisition ownership rules.
    #[repr(C)]
    struct TestFactory {
        /// Reference count shared by the fixture's one projection.
        references: AtomicU32,
    }

    /// Returns the same fixture projection when its complete identity matches.
    unsafe extern "C" fn query_interface(this: *mut c_void, requested: *const NcInterfaceId, out: *mut neocogi_com_core::ComRaw<IUnknown>) -> NcStatus {
        if this.is_null() || requested.is_null() || out.is_null() {
            return NC_E_POINTER;
        }
        // SAFETY: The caller grants one output write.
        unsafe { *out = neocogi_com_core::ComRaw::null() };
        // SAFETY: Non-null requested identity remains readable for the call.
        let requested = unsafe { &*requested };
        if requested.iid != FACTORY_INTERFACE.iid {
            return NC_E_NOINTERFACE;
        }
        if requested.abi_signature != FACTORY_INTERFACE.abi_signature {
            return NC_E_ABI_VERSION;
        }
        // SAFETY: The paired table describes this live fixture allocation.
        unsafe { add_ref(this) };
        // SAFETY: `out` is writable and this fixture has one projection address.
        unsafe { *out = neocogi_com_core::ComRaw { object: this, vtable: &VTABLE } };
        NC_OK
    }

    /// Adds one owned reference to the live fixture allocation.
    unsafe extern "C" fn add_ref(this: *mut c_void) -> u32 {
        // SAFETY: The paired table receives its live allocation directly.
        let object = unsafe { &*this.cast::<TestFactory>() };
        object.references.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Releases one reference and destroys the fixture at zero.
    unsafe extern "C" fn release(this: *mut c_void) -> u32 {
        // SAFETY: The paired table receives its live allocation directly.
        let object = unsafe { &*this.cast::<TestFactory>() };
        let previous = object.references.fetch_sub(1, Ordering::Release);
        let remaining = previous - 1;
        if remaining == 0 {
            std::sync::atomic::fence(Ordering::Acquire);
            FACTORY_DROPS.fetch_add(1, Ordering::SeqCst);
            // SAFETY: The final unique reference rebuilds the original Box once.
            unsafe { drop(Box::from_raw(this.cast::<TestFactory>())) };
        }
        remaining
    }

    /// Vtable prefix advertising the exact fake factory interface identity.
    static VTABLE: IUnknownVTable = IUnknownVTable {
        interface_id: &FACTORY_INTERFACE,
        query_interface,
        add_ref,
        release,
        downgrade: None,
    };

    /// Allocates and transfers one fake root factory reference.
    unsafe extern "C" fn acquire(out: *mut neocogi_com_core::ComRaw<IUnknown>) -> NcStatus {
        if out.is_null() {
            return NC_E_POINTER;
        }
        FACTORY_ACQUISITIONS.fetch_add(1, Ordering::SeqCst);
        let object = Box::new(TestFactory { references: AtomicU32::new(1) });
        let raw = Box::into_raw(object);
        // SAFETY: The caller grants one output write and ownership transfers.
        unsafe { *out = neocogi_com_core::ComRaw { object: raw.cast(), vtable: &VTABLE } };
        NC_OK
    }

    /// Minimal validated registry record used only by resolution tests.
    static DESCRIPTION: NcPluginDescriptor = NcPluginDescriptor {
        description_abi_version: neocogi_com_core::NC_PLUGIN_DESCRIPTOR_ABI_VERSION,
        core_abi_version: neocogi_com_core::NC_COM_CORE_ABI_VERSION,
        plugin_id: guid_from_qualified_name("nc.tests.acquisition"),
        qualified_name: NcStaticUtf8::from_static("nc.tests.acquisition"),
        plugin_version: NcStaticUtf8::from_static("1.0.0"),
        dependency_count: 0,
        dependencies: core::ptr::null(),
        root_factory: FACTORY_INTERFACE,
        root_factory_threading: NC_ABI_THREADING_THREAD_LOCAL,
        acquire: Some(acquire),
    };

    /// Confirms lookup and invocation transfer one identity-checked owner.
    #[test]
    fn activates_only_the_exact_factory_interface() {
        let _fixture = FIXTURE_LOCK.lock().unwrap();
        FACTORY_DROPS.store(0, Ordering::SeqCst);
        let mut registry = FactoryPluginRegistry::new();
        registry.insert(RegisteredFactoryPlugin {
            plugin_id: DESCRIPTION.plugin_id,
            description: &DESCRIPTION,
            dependency_ids: Vec::new(),
            origin: crate::factory_plugin_registry::PluginOrigin::Static,
        });
        let resolved = resolve_factory(&registry, DESCRIPTION.plugin_id, FACTORY_INTERFACE, NC_ABI_THREADING_THREAD_LOCAL);
        let mut out = neocogi_com_core::ComRaw::null();
        // SAFETY: The local output slot receives one owned fixture reference.
        assert_eq!(unsafe { acquire_factory(resolved, FACTORY_INTERFACE, &mut out) }, NC_OK);
        assert!(!out.is_null());
        // SAFETY: Successful acquisition transferred the fixture's initial owner.
        unsafe { ((*out.vtable).release)(out.object) };
        assert_eq!(FACTORY_DROPS.load(Ordering::SeqCst), 1);
    }

    /// A nominal IID match cannot enter bootstrap with absent or different bytes.
    #[test]
    fn signature_mismatch_is_rejected_before_bootstrap() {
        let _fixture = FIXTURE_LOCK.lock().unwrap();
        FACTORY_ACQUISITIONS.store(0, Ordering::SeqCst);
        let mut registry = FactoryPluginRegistry::new();
        registry.insert(RegisteredFactoryPlugin {
            plugin_id: DESCRIPTION.plugin_id,
            description: &DESCRIPTION,
            dependency_ids: Vec::new(),
            origin: crate::factory_plugin_registry::PluginOrigin::Static,
        });
        /// The fixture's exact digest is [7; 32]; differ only at its final byte.
        static DIFFERENT_LAST_BYTE: [u8; 32] = {
            let mut bytes = [7; 32];
            bytes[31] = 8;
            bytes
        };
        for signature in [NcAbiSignature::EMPTY, NcAbiSignature::from_static(&DIFFERENT_LAST_BYTE)] {
            let mismatched = NcInterfaceId::new(FACTORY_ID, signature);
            let resolved = resolve_factory(&registry, DESCRIPTION.plugin_id, mismatched, NC_ABI_THREADING_THREAD_LOCAL);
            let mut out = neocogi_com_core::ComRaw {
                object: core::ptr::dangling_mut(),
                vtable: &VTABLE,
            };
            // SAFETY: The output slot is writable; rejection must clear it without invoking code.
            assert_eq!(unsafe { acquire_factory(resolved, mismatched, &mut out) }, NC_E_ABI_VERSION);
            assert!(out.is_null());
            assert_eq!(FACTORY_ACQUISITIONS.load(Ordering::SeqCst), 0);
        }
    }

    /// Confirms an IID match with another signature is rejected and released.
    #[test]
    fn rejects_and_releases_a_factory_with_the_wrong_signature() {
        /// Deliberately invalid digest used to exercise exact-signature rejection.
        static MISMATCHED_SIGNATURE: [u8; 32] = [0; 32];

        let _fixture = FIXTURE_LOCK.lock().unwrap();
        FACTORY_DROPS.store(0, Ordering::SeqCst);
        let mismatched = NcInterfaceId::new(FACTORY_ID, NcAbiSignature::from_static(&MISMATCHED_SIGNATURE));
        let mut out = neocogi_com_core::ComRaw::null();
        // SAFETY: The acquisition function and local output slot satisfy the ABI.
        assert_eq!(unsafe { acquire_factory(Ok(acquire), mismatched, &mut out) }, NC_E_ABI_VERSION);
        assert!(out.is_null());
        assert_eq!(FACTORY_DROPS.load(Ordering::SeqCst), 1);
    }
}

/// Bootstrap records can be malformed before a table is safe to read.
#[cfg(test)]
mod malformed_bootstrap_tests {
    use super::*;
    use neocogi_com_core::ComRaw;

    /// Simulates a producer writing an object word without a method table.
    unsafe extern "C" fn missing_table(out: *mut ComRaw<IUnknown>) -> NcStatus {
        // SAFETY: the test supplies one writable output record; no object is acquired.
        unsafe {
            *out = ComRaw {
                object: core::ptr::dangling_mut(),
                vtable: core::ptr::null(),
            };
        }
        NC_OK
    }
    /// Simulates the opposite half-null shape; its table must never be read.
    unsafe extern "C" fn missing_object(out: *mut ComRaw<IUnknown>) -> NcStatus {
        // SAFETY: the test supplies one writable output record; shape rejection comes first.
        unsafe {
            *out = ComRaw {
                object: core::ptr::null_mut(),
                vtable: core::ptr::dangling(),
            };
        }
        NC_OK
    }
    /// Both malformed shapes return pointer errors and clear the external result.
    #[test]
    fn half_null_bootstrap_never_enters_query_or_release() {
        for acquire in [missing_table as NcAcquirePluginFactory, missing_object] {
            let mut out = ComRaw::null();
            // SAFETY: these fixtures deliberately exercise validation before any pointer read.
            assert_eq!(unsafe { acquire_factory(Ok(acquire), IUnknown::INTERFACE_ID, &mut out) }, NC_E_POINTER);
            assert!(out.is_null());
        }
    }
}
