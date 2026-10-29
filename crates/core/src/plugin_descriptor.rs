//! Self-contained plugin descriptors and typed factory acquisition.
//!
//! A plugin carries the exact interface identity of its host-supplied factory.
//! Compatibility is checked when that factory is requested; no runtime module
//! graph or contract-library ownership participates in registration. Object construction after
//! bootstrap occurs only through typed factory methods; no class table or erased
//! constructor frame is present in this ABI.

use crate::{
    ComInterface, ComPtr, IUnknown, NC_COM_CORE_ABI_VERSION, NC_E_ABI_VERSION, NC_E_INVALID_ARG, NC_E_POINTER, NC_OK, NcAbiThreadingMode, NcInterfaceId,
    NcGuid, NcPluginDependencyDescriptor, NcStaticUtf8, NcStatus, guid_from_qualified_name, validate_qualified_name, validate_version_text,
};

/// Pre-release plugin descriptor ABI with an exact root factory and dependencies.
///
/// Descriptor discovery remains version one; its stable version prefix is read first.
/// Consumers must also match the core generation, which selects the remaining
/// record layout, rather than infer compatibility from this number alone.
pub const NC_PLUGIN_DESCRIPTOR_ABI_VERSION: u32 = 1;

/// Exported symbol returning a plugin descriptor.
pub const NC_GET_PLUGIN_DESCRIPTOR_SYMBOL: &str = "nc_get_plugin_descriptor_v1";

/// Fixed bootstrap operation that transfers one owned root factory reference.
///
/// Acquisition may construct a factory or retain an existing instance. Each
/// successful call transfers one owner; the runtime promises no singleton or
/// identity reuse between calls. Product construction remains on typed methods.
///
/// # Safety
///
/// `host` satisfies [`crate::validate_host_api`]'s readable-prefix obligations.
/// Supported tables and callback code remain immutable for process/Wasm-instance
/// lifetime and belong to one coherent host supporting concurrent entrypoints.
/// Caller-owned contexts keep their operation-specific thread/borrow rules.
/// Null/incompatible access is rejected
/// before provider code. A copied outer record alone does not prove child lifetimes.
/// `out` must be valid for one ComRaw record write. The function must clear both
/// fields before fallible work and return one owned factory pair only on success.
/// Its erased table preserves the declared factory identity, not IUnknown's
/// identity. Common lifecycle calls are valid; typed IUnknown adoption requires
/// QueryInterface. Erasure neither casts the object nor acquires a reference.
pub type NcAcquirePluginFactory = unsafe extern "C" fn(host: *const crate::NcHostApiV1, out: *mut crate::ComRaw<IUnknown>) -> NcStatus;

/// Adapts a safe typed root-factory constructor to the fixed plugin ABI.
///
/// The plugin macro monomorphizes this helper for its typed acquisition return.
/// The ordinary interface owns identity and callable methods. This helper clears
/// output before provider code and transfers exactly one successful reference;
/// it imposes no singleton or object-construction policy.
///
/// # Safety
///
/// `out` must be null or aligned and writable for one ComRaw record. A usable
/// slot is cleared before the implementation's fallible acquisition code runs.
pub unsafe fn write_plugin_root<T: ComInterface>(
    out: *mut crate::ComRaw<IUnknown>,
    acquire: impl FnOnce() -> Result<ComPtr<T, T::Threading>, NcStatus>,
) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: The caller grants one output write and null was rejected above.
    unsafe { *out = crate::ComRaw::null() };
    let factory = match acquire() {
        Ok(factory) => factory,
        Err(status) => return status.normalize_error(),
    };
    // All generated interfaces carry the IUnknown ABI prefix at the same
    // address. Consuming the typed pointer transfers its existing ownership
    // into the untyped bootstrap slot without AddRef or Release.
    let raw_factory = factory.into_raw();
    let unknown = raw_factory.cast::<crate::IUnknown>();
    if unknown.is_null() {
        // A valid owned ComPtr cannot contain null. Retain a defensive check at
        // this unsafe plugin boundary in case a foreign implementation violates
        // the trait contract.
        return NC_E_POINTER;
    }
    // SAFETY: The caller grants one output write; ownership of the typed COM
    // pointer was transferred rather than dropped.
    unsafe { *out = unknown };
    NC_OK
}

/// Provider-authored registration input containing metadata and one root factory.
///
/// The provider publishes its identity, release version, hard plugin dependencies,
/// exact factory interface/threading contract and acquisition function. The runtime
/// validates this record without acquiring a factory. Registration retains the
/// descriptor; it does not take ownership of the provider's storage.
///
/// This record and all reachable text, tables, signature bytes and bootstrap code
/// must remain immutable and live while used. Static registration requires process
/// lifetime storage; dynamic registration keeps the exporting library pinned.
/// A copied record copies pointers only and does not retain a library.
///
/// [`crate::NcPluginInfoV1`] is the runtime-produced lookup/enumeration output. It
/// includes registration provenance and omits the bootstrap function. Neither
/// reading that output nor inspecting this descriptor transfers a COM reference.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcPluginDescriptor {
    /// Descriptor family version; must equal [`NC_PLUGIN_DESCRIPTOR_ABI_VERSION`].
    /// This and `core_abi_version` form the prefix checked before the full record.
    pub description_abi_version: u32,
    /// Core object ABI generation understood by this plugin.
    pub core_abi_version: u32,
    /// Stable plugin identity derived from `qualified_name`.
    pub plugin_id: NcGuid,
    /// Normative qualified plugin name.
    pub qualified_name: NcStaticUtf8,
    /// Static plugin version text.
    pub plugin_version: NcStaticUtf8,
    /// Number of hard plugin dependency IDs.
    pub dependency_count: u32,
    /// Pointer to `dependency_count` distinct dependency records.
    pub dependencies: *const NcPluginDependencyDescriptor,
    /// Exact IID and transitive ABI signature expected from the root factory.
    pub root_factory: NcInterfaceId,
    /// Threading capability promised by the returned root factory.
    pub root_factory_threading: u32,
    /// Fixed bootstrap supplied with access to the acquiring host.
    pub acquire: Option<NcAcquirePluginFactory>,
}

// SAFETY: Descriptor use requires an immutable reachable metadata graph. The
// acquisition pointer is code in the library that owns the descriptor; sharing
// the record does not upgrade the factory's declared threading capability.
unsafe impl Sync for NcPluginDescriptor {}

impl NcPluginDescriptor {
    /// Borrows hard plugin dependencies after checking pointer/count shape.
    ///
    /// # Safety
    ///
    /// The descriptor and dependency table must remain readable for the
    /// returned lifetime.
    pub unsafe fn dependencies<'a>(&self) -> Result<&'a [NcPluginDependencyDescriptor], NcStatus> {
        // SAFETY: The caller supplies the backing lifetime; the helper checks
        // null/count shape before constructing a slice.
        unsafe { descriptor_slice(self.dependencies, self.dependency_count) }
    }
}

/// Getter exported by one plugin shared library.
///
/// The symbol is [`NC_GET_PLUGIN_DESCRIPTOR_SYMBOL`]. Success lends one validated
/// descriptor; it neither registers the plugin nor acquires a factory. A usable
/// output slot is cleared before fallible work and remains null on failure.
/// Null output returns [`NC_E_POINTER`]; validation failures return their status.
///
/// # Safety
///
/// `out` must be null or aligned and writable for one descriptor pointer. On
/// success it receives immutable metadata, including all reachable text, tables,
/// signatures and bootstrap code, valid while the exporting library is mapped.
/// The getter must not unwind through the C boundary.
pub type NcGetPluginDescriptor = unsafe extern "C" fn(out: *mut *const NcPluginDescriptor) -> NcStatus;

/// Validates one complete plugin descriptor without allocation.
///
/// This pass validates metadata only and never invokes plugin acquisition. The
/// caller can therefore inspect or register a plugin without constructing it.
///
/// # Safety
///
/// `plugin` must be null or point to an aligned readable two-u32 version prefix.
/// If those versions are supported, the complete current record, its tables,
/// strings and any present 32-byte digests must be readable for this call.
pub unsafe fn validate_plugin_descriptor(plugin: *const NcPluginDescriptor) -> NcStatus {
    if plugin.is_null() {
        return NC_E_POINTER;
    }
    // Read only the stable version prefix before borrowing the current full
    // record. Earlier core generations have different signature/record sizes.
    // SAFETY: The caller guarantees this aligned two-u32 prefix is readable.
    let (descriptor_version, core_version) = unsafe {
        (
            core::ptr::addr_of!((*plugin).description_abi_version).read(),
            core::ptr::addr_of!((*plugin).core_abi_version).read(),
        )
    };
    if descriptor_version != NC_PLUGIN_DESCRIPTOR_ABI_VERSION || core_version != NC_COM_CORE_ABI_VERSION {
        return NC_E_ABI_VERSION;
    }
    // SAFETY: Matching versions establish the required full-record layout,
    // whose storage and reachable pointers are covered by the caller contract.
    let plugin = unsafe { &*plugin };
    // SAFETY: The caller guarantees both static text views remain readable.
    let Ok(name) = (unsafe { plugin.qualified_name.as_str() }) else {
        return NC_E_INVALID_ARG;
    };
    // SAFETY: The caller guarantees both static text views remain readable.
    let Ok(version) = (unsafe { plugin.plugin_version.as_str() }) else {
        return NC_E_INVALID_ARG;
    };
    if validate_qualified_name(name).is_err()
        || validate_version_text(version).is_err()
        || plugin.plugin_id != guid_from_qualified_name(name)
        || plugin.root_factory.iid.is_nil()
        || plugin.root_factory.abi_signature.is_empty()
        || NcAbiThreadingMode::from_raw(plugin.root_factory_threading).is_none()
        || plugin.acquire.is_none()
    {
        return NC_E_INVALID_ARG;
    }
    // SAFETY: The caller guarantees the dependency table lifetime for this validation.
    let dependencies = match unsafe { plugin.dependencies() } {
        Ok(dependencies) => dependencies,
        Err(status) => return status,
    };
    for (index, dependency) in dependencies.iter().enumerate() {
        if dependency.plugin_id.is_nil()
            || dependency.plugin_id == plugin.plugin_id
            || dependencies[..index].iter().any(|previous| previous.plugin_id == dependency.plugin_id)
        {
            return NC_E_INVALID_ARG;
        }
    }
    NC_OK
}

/// Publishes one validated static plugin descriptor through an export.
///
/// # Safety
///
/// `out` must be null or aligned and writable for one pointer. `plugin` must
/// remain valid while its exporting library is loaded, including every reachable metadata
/// pointer and bootstrap function. A static root reference alone does not prove
/// that those referenced allocations and code remain readable and callable.
pub unsafe fn return_plugin_descriptor(out: *mut *const NcPluginDescriptor, plugin: &'static NcPluginDescriptor) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // Clear before validation so a failure never publishes stale metadata.
    // SAFETY: The caller guarantees one writable output slot.
    unsafe { *out = core::ptr::null() };
    // SAFETY: The caller guarantees the complete metadata graph remains valid,
    // not just the root covered by the shared reference.
    let status = unsafe { validate_plugin_descriptor(plugin) };
    if status.is_err() {
        return status;
    }
    // SAFETY: The output slot was checked above and validation succeeded.
    unsafe { *out = plugin };
    NC_OK
}

/// Constructs a slice from one immutable descriptor pointer/count pair.
///
/// # Safety
///
/// A non-null `pointer` must address `count` initialized readable values that
/// remain valid for the returned lifetime.
unsafe fn descriptor_slice<'a, T>(pointer: *const T, count: u32) -> Result<&'a [T], NcStatus> {
    if count == 0 {
        return Ok(&[]);
    }
    if pointer.is_null() {
        return Err(NC_E_POINTER);
    }
    // SAFETY: The caller supplies table readability and lifetime; null/count
    // shape was checked above.
    Ok(unsafe { core::slice::from_raw_parts(pointer, count as usize) })
}

#[cfg(test)]
mod tests {
    use crate::NcAbiSignature;
    use core::ptr;

    use super::*;
    use crate::NC_ABI_THREADING_THREAD_LOCAL;

    /// Exact factory identity shared by the descriptor and acquisition tests.
    const FACTORY_ID: NcInterfaceId = NcInterfaceId::new(NcGuid::from_fields(6, 7, 8, [9; 8]), NcAbiSignature::from_static(&[5; 32]));

    /// Satisfies the fixed factory acquisition shape for descriptor validation.
    unsafe extern "C" fn acquire_factory(_host: *const crate::NcHostApiV1, out: *mut crate::ComRaw<IUnknown>) -> NcStatus {
        if out.is_null() {
            return NC_E_POINTER;
        }
        // This fixture verifies metadata only, so it reports a predictable
        // acquisition failure after honoring the required cleared-output rule.
        // SAFETY: Null was rejected and the ABI contract grants one write.
        unsafe { *out = crate::ComRaw::null() };
        NC_E_INVALID_ARG
    }

    /// Complete plugin descriptor fixture with no plugin dependencies.
    static VALID_PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
        description_abi_version: NC_PLUGIN_DESCRIPTOR_ABI_VERSION,
        core_abi_version: NC_COM_CORE_ABI_VERSION,
        plugin_id: guid_from_qualified_name("nc.tests.factory_plugin"),
        qualified_name: NcStaticUtf8::from_static("nc.tests.factory_plugin"),
        plugin_version: NcStaticUtf8::from_static("1.0.0"),
        dependency_count: 0,
        dependencies: ptr::null(),
        root_factory: FACTORY_ID,
        root_factory_threading: NC_ABI_THREADING_THREAD_LOCAL,
        acquire: Some(acquire_factory),
    };

    /// Confirms valid metadata publishes the exact immutable description.
    #[test]
    fn validates_and_returns_plugin_descriptor() {
        // SAFETY: The fixture and all referenced metadata are immutable statics.
        assert_eq!(unsafe { validate_plugin_descriptor(&VALID_PLUGIN) }, NC_OK);
        let mut returned = ptr::null();
        // SAFETY: `returned` provides one writable pointer slot.
        assert_eq!(unsafe { return_plugin_descriptor(&mut returned, &VALID_PLUGIN) }, NC_OK);
        assert!(ptr::eq(returned, &VALID_PLUGIN));
    }

    /// Confirms plugin identity cannot disagree with its normative name.
    #[test]
    fn rejects_a_plugin_id_unrelated_to_its_name() {
        let invalid = NcPluginDescriptor {
            plugin_id: NcGuid::from_fields(99, 2, 3, [4; 8]),
            ..VALID_PLUGIN
        };
        // SAFETY: The stack root borrows only immutable static child tables.
        assert_eq!(unsafe { validate_plugin_descriptor(&invalid) }, NC_E_INVALID_ARG);
    }

    /// Rejects incompatible descriptor layouts before inspecting root metadata.
    #[test]
    fn rejects_descriptor_version_and_missing_factory_signature() {
        let incompatible = NcPluginDescriptor {
            description_abi_version: NC_PLUGIN_DESCRIPTOR_ABI_VERSION + 1,
            ..VALID_PLUGIN
        };
        // SAFETY: Both fixtures have readable roots and immutable child storage.
        assert_eq!(unsafe { validate_plugin_descriptor(&incompatible) }, NC_E_ABI_VERSION);
        let unsigned = NcPluginDescriptor {
            root_factory: NcInterfaceId::new(FACTORY_ID.iid, NcAbiSignature::EMPTY),
            ..VALID_PLUGIN
        };
        assert_eq!(unsafe { validate_plugin_descriptor(&unsigned) }, NC_E_INVALID_ARG);
    }

    /// Unsupported versions are rejected using only their common prefix,
    /// without a complete descriptor borrow, signature read or bootstrap access.
    #[test]
    fn rejects_unsupported_versions_from_the_prefix_alone() {
        assert_eq!(core::mem::offset_of!(NcPluginDescriptor, description_abi_version), 0);
        assert_eq!(core::mem::offset_of!(NcPluginDescriptor, core_abi_version), 4);
        for versions in [
            [NC_PLUGIN_DESCRIPTOR_ABI_VERSION, 0],
            [NC_PLUGIN_DESCRIPTOR_ABI_VERSION, 1],
            [NC_PLUGIN_DESCRIPTOR_ABI_VERSION, 17],
            [NC_PLUGIN_DESCRIPTOR_ABI_VERSION, 2],
            [NC_PLUGIN_DESCRIPTOR_ABI_VERSION, u32::MAX],
            [0, NC_COM_CORE_ABI_VERSION],
            [NC_PLUGIN_DESCRIPTOR_ABI_VERSION + 1, NC_COM_CORE_ABI_VERSION],
        ] {
            // SAFETY: Only the readable version prefix is promised for an
            // unsupported generation. No trailing record even exists here.
            assert_eq!(unsafe { validate_plugin_descriptor(versions.as_ptr().cast()) }, NC_E_ABI_VERSION);
        }
    }

    /// Locks the new descriptor's field order to its documented C ABI layout.
    #[test]
    fn plugin_descriptor_layout_is_c_stable() {
        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(core::mem::size_of::<NcPluginDescriptor>(), 112);
            assert_eq!(core::mem::align_of::<NcPluginDescriptor>(), 8);
            assert_eq!(core::mem::offset_of!(NcPluginDescriptor, plugin_id), 8);
            assert_eq!(core::mem::offset_of!(NcPluginDescriptor, dependencies), 64);
            assert_eq!(core::mem::offset_of!(NcPluginDescriptor, root_factory), 72);
            assert_eq!(core::mem::offset_of!(NcPluginDescriptor, acquire), 104);
        }
        #[cfg(target_pointer_width = "32")]
        {
            assert_eq!(core::mem::size_of::<NcPluginDescriptor>(), 76);
            assert_eq!(core::mem::align_of::<NcPluginDescriptor>(), 4);
            assert_eq!(core::mem::offset_of!(NcPluginDescriptor, plugin_id), 8);
            assert_eq!(core::mem::offset_of!(NcPluginDescriptor, dependencies), 44);
            assert_eq!(core::mem::offset_of!(NcPluginDescriptor, root_factory), 48);
            assert_eq!(core::mem::offset_of!(NcPluginDescriptor, acquire), 72);
        }
    }
}
