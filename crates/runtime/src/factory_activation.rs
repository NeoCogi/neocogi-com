//! Root-factory lookup and invocation for registered factory-only plugins.
//!
//! Resolution occurs under the catalog read lock, while plugin code runs only
//! after that guard is released. The runtime validates the returned object's
//! complete factory interface identity before transferring ownership to a host.

use neocogi_com_core::{
    IUnknown, NC_ABI_THREADING_THREAD_LOCAL, NC_ABI_THREADING_THREAD_SAFE, NC_E_NOINTERFACE, NC_E_NOT_FOUND, NC_E_POINTER, NC_OK, NcActivatePluginFactory,
    NcFactoryPluginDescription, NcGuid, NcInterfaceId, NcStatus, raw,
};

use crate::factory_plugin_registry::FactoryPluginRegistry;

/// Resolves one compatible activation pointer without entering plugin code.
pub(crate) fn resolve_factory(
    registry: &FactoryPluginRegistry,
    plugin_id: NcGuid,
    factory_interface: NcInterfaceId,
    requested_threading: u32,
) -> Result<NcActivatePluginFactory, NcStatus> {
    let registered = registry.plugin(plugin_id).ok_or(NC_E_NOT_FOUND)?;
    // SAFETY: The registry retains only validated immutable descriptions whose
    // static or mapped-library owner remains live while registered.
    let description: &NcFactoryPluginDescription = unsafe { &*registered.description };
    if description.root_factory_id != factory_interface.iid {
        return Err(NC_E_NOINTERFACE);
    }
    if !threading_request_is_compatible(description.root_factory_threading, requested_threading) {
        return Err(neocogi_com_core::NC_E_INVALID_ARG);
    }
    description.activate.ok_or(neocogi_com_core::NC_E_INVALID_ARG)
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
/// `out` must be valid for one pointer write. `activate` must remain callable
/// through this operation, which runtime registration and module retention
/// guarantee for values returned by [`resolve_factory`].
pub(crate) unsafe fn activate_factory(
    activate: Result<NcActivatePluginFactory, NcStatus>,
    factory_interface: NcInterfaceId,
    out: *mut *mut IUnknown,
) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: The caller grants one output write and null was rejected above.
    unsafe { *out = core::ptr::null_mut() };
    let activate = match activate {
        Ok(activate) => activate,
        Err(status) => return status,
    };

    // SAFETY: The registered pointer remains mapped and `out` satisfies its
    // fixed output contract for this synchronous call.
    let status = unsafe { activate(out) };
    if status.is_err() {
        // A plugin must clear failed outputs. Clear defensively without
        // dereferencing an untrusted value supplied on an error path.
        // SAFETY: `out` was validated above.
        unsafe { *out = core::ptr::null_mut() };
        return status;
    }
    // SAFETY: The output slot remains readable after the plugin returns.
    let root = unsafe { *out };
    if root.is_null() {
        return NC_E_POINTER;
    }
    // Read only the common vtable prefix needed for QueryInterface. A malformed
    // trusted plugin is rejected before any factory-specific method is called.
    // SAFETY: Successful activation promises one live COM interface prefix.
    let vtable = unsafe { (*root).vtable };
    if vtable.is_null() {
        // SAFETY: `out` is still the caller's writable slot.
        unsafe { *out = core::ptr::null_mut() };
        return NC_E_POINTER;
    }

    let mut projection = core::ptr::null_mut();
    // SAFETY: The successful activation result owns a live common prefix and
    // the local output slot receives one queried owned reference.
    let query_status = unsafe { ((*vtable).query_interface)(root, &factory_interface, &mut projection) };
    if query_status.is_err() || projection.is_null() {
        // Consume the activated reference rather than leaking a root that does
        // not implement the plugin's canonical factory declaration.
        // SAFETY: The activation success contract transferred one live owner.
        unsafe { ((*vtable).release)(root) };
        // SAFETY: `out` remains writable for this call.
        unsafe { *out = core::ptr::null_mut() };
        return if query_status.is_err() { query_status } else { NC_E_POINTER };
    }

    // SAFETY: QueryInterface returned one owned interface reference. Validate
    // its complete IID/signature before releasing this temporary proof.
    let validation = unsafe { raw::__nc_validate_iunknown_interface(projection.cast::<IUnknown>(), &factory_interface) };
    // SAFETY: A non-null successful QueryInterface result owns one reference;
    // every interface shares the release-compatible common prefix.
    unsafe { raw::release(projection.cast::<IUnknown>()) };
    if let Err(status) = validation {
        // SAFETY: The original activation result still owns its independent reference.
        unsafe { ((*vtable).release)(root) };
        // SAFETY: `out` remains writable for this call.
        unsafe { *out = core::ptr::null_mut() };
        return status;
    }
    NC_OK
}

#[cfg(test)]
mod tests {
    use core::ffi::c_void;
    use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

    use neocogi_com_core::{
        IUnknownVTable, NC_ABI_THREADING_THREAD_LOCAL, NC_E_ABI_VERSION, NcAbiSignature, NcFactoryPluginDescription, NcStaticUtf8, guid_from_qualified_name,
    };

    use super::*;
    use crate::factory_plugin_registry::RegisteredFactoryPlugin;

    /// Stable fake factory interface used by raw activation validation.
    const FACTORY_ID: NcGuid = NcGuid::from_fields(1, 2, 3, [4; 8]);
    /// Complete fake interface identity stored in the vtable prefix.
    const FACTORY_INTERFACE: NcInterfaceId = NcInterfaceId::new(FACTORY_ID, NcAbiSignature::EMPTY);
    /// Counts factory allocations destroyed by their final Release.
    static FACTORY_DROPS: AtomicUsize = AtomicUsize::new(0);

    /// Minimal owned COM storage used to exercise activation ownership rules.
    #[repr(C)]
    struct TestFactory {
        /// Publicly returned common interface header.
        unknown: IUnknown,
        /// Reference count shared by the fixture's one projection.
        references: AtomicU32,
    }

    /// Returns the same fixture projection when its complete identity matches.
    unsafe extern "C" fn query_interface(this: *mut IUnknown, requested: *const NcInterfaceId, out: *mut *mut c_void) -> NcStatus {
        if this.is_null() || requested.is_null() || out.is_null() {
            return NC_E_POINTER;
        }
        // SAFETY: The caller grants one output write.
        unsafe { *out = core::ptr::null_mut() };
        // SAFETY: Non-null requested identity remains readable for the call.
        let requested = unsafe { &*requested };
        if requested.iid != FACTORY_INTERFACE.iid {
            return NC_E_NOINTERFACE;
        }
        if requested.abi_signature != FACTORY_INTERFACE.abi_signature {
            return NC_E_ABI_VERSION;
        }
        // SAFETY: The fixture header owns a live storage back-pointer.
        unsafe { add_ref(this) };
        // SAFETY: `out` is writable and this fixture has one projection address.
        unsafe { *out = this.cast() };
        NC_OK
    }

    /// Adds one owned reference to the live fixture allocation.
    unsafe extern "C" fn add_ref(this: *mut IUnknown) -> u32 {
        // SAFETY: The fixture installs its allocation pointer before publication.
        let object = unsafe { &*(*this).object.cast::<TestFactory>() };
        object.references.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Releases one reference and destroys the fixture at zero.
    unsafe extern "C" fn release(this: *mut IUnknown) -> u32 {
        // SAFETY: The fixture installs its allocation pointer before publication.
        let object = unsafe { &*(*this).object.cast::<TestFactory>() };
        let previous = object.references.fetch_sub(1, Ordering::Release);
        let remaining = previous - 1;
        if remaining == 0 {
            std::sync::atomic::fence(Ordering::Acquire);
            FACTORY_DROPS.fetch_add(1, Ordering::SeqCst);
            // SAFETY: The final unique reference rebuilds the original Box once.
            unsafe { drop(Box::from_raw((*this).object.cast::<TestFactory>())) };
        }
        remaining
    }

    /// Vtable prefix advertising the exact fake factory interface identity.
    static VTABLE: IUnknownVTable = IUnknownVTable {
        interface_id: &FACTORY_INTERFACE,
        query_interface,
        add_ref,
        release,
    };

    /// Allocates and transfers one fake root factory reference.
    unsafe extern "C" fn activate(out: *mut *mut IUnknown) -> NcStatus {
        if out.is_null() {
            return NC_E_POINTER;
        }
        let object = Box::new(TestFactory {
            unknown: IUnknown {
                vtable: &VTABLE,
                object: core::ptr::null_mut(),
            },
            references: AtomicU32::new(1),
        });
        let raw = Box::into_raw(object);
        // SAFETY: `raw` is uniquely owned until the header pointer is published.
        unsafe { (*raw).unknown.object = raw.cast() };
        // SAFETY: The caller grants one output write and ownership transfers.
        unsafe { *out = core::ptr::addr_of_mut!((*raw).unknown) };
        NC_OK
    }

    /// Minimal validated registry record used only by resolution tests.
    static DESCRIPTION: NcFactoryPluginDescription = NcFactoryPluginDescription {
        core_abi_version: neocogi_com_core::NC_COM_CORE_ABI_VERSION,
        plugin_id: guid_from_qualified_name("nc.tests.activation"),
        qualified_name: NcStaticUtf8::from_static("nc.tests.activation"),
        plugin_version: NcStaticUtf8::from_static("1.0.0"),
        dependency_count: 0,
        dependencies: core::ptr::null(),
        contract_count: 0,
        contracts: core::ptr::null(),
        root_factory_id: FACTORY_ID,
        root_factory_threading: NC_ABI_THREADING_THREAD_LOCAL,
        activate: Some(activate),
    };

    /// Confirms lookup and invocation transfer one identity-checked owner.
    #[test]
    fn activates_only_the_exact_factory_interface() {
        FACTORY_DROPS.store(0, Ordering::SeqCst);
        let mut registry = FactoryPluginRegistry::new();
        registry.insert(RegisteredFactoryPlugin {
            plugin_id: DESCRIPTION.plugin_id,
            description: &DESCRIPTION,
            dependency_ids: Vec::new(),
        });
        let resolved = resolve_factory(&registry, DESCRIPTION.plugin_id, FACTORY_INTERFACE, NC_ABI_THREADING_THREAD_LOCAL);
        let mut out = core::ptr::null_mut();
        // SAFETY: The local output slot receives one owned fixture reference.
        assert_eq!(unsafe { activate_factory(resolved, FACTORY_INTERFACE, &mut out) }, NC_OK);
        assert!(!out.is_null());
        // SAFETY: Successful activation transferred the fixture's initial owner.
        unsafe { ((*(*out).vtable).release)(out) };
        assert_eq!(FACTORY_DROPS.load(Ordering::SeqCst), 1);
    }

    /// Confirms an IID match with another signature is rejected and released.
    #[test]
    fn rejects_and_releases_a_factory_with_the_wrong_signature() {
        /// Deliberately invalid digest used to exercise exact-signature rejection.
        static MISMATCHED_SIGNATURE: [u8; 32] = [0; 32];

        FACTORY_DROPS.store(0, Ordering::SeqCst);
        let mismatched = NcInterfaceId::new(FACTORY_ID, NcAbiSignature::from_static(&MISMATCHED_SIGNATURE));
        let mut out = core::ptr::null_mut();
        // SAFETY: The activation function and local output slot satisfy the ABI.
        assert_eq!(unsafe { activate_factory(Ok(activate), mismatched, &mut out) }, NC_E_ABI_VERSION);
        assert!(out.is_null());
        assert_eq!(FACTORY_DROPS.load(Ordering::SeqCst), 1);
    }
}
