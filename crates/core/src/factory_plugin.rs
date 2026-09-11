//! Factory-only plugin descriptions for IDL contract modules.
//!
//! A factory plugin names the exact native contract modules it was compiled
//! against and exposes one fixed bootstrap operation. Object construction after
//! bootstrap occurs only through typed factory methods; no class table or erased
//! constructor frame is present in this ABI.

use crate::{
    ComFactoryBootstrap, ComInterface, IUnknown, NC_ABI_VALIDATION_MODE, NC_COM_CORE_ABI_VERSION, NC_E_ABI_VERSION, NC_E_INVALID_ARG, NC_E_POINTER, NC_OK,
    NcAbiThreadingMode, NcAbiValidationMode, NcContractModuleReference, NcGuid, NcPluginDependencyDescriptor, NcStaticUtf8, NcStatus, guid_from_qualified_name,
    validate_qualified_name, validate_version_text,
};

/// Exported symbol returning a factory-only plugin description.
pub const NC_GET_FACTORY_PLUGIN_DESCRIPTION_SYMBOL: &str = "nc_get_factory_plugin_description";

/// Fixed bootstrap operation that creates one owned root factory object.
///
/// # Safety
///
/// `out` must be valid for one pointer write. The function must clear it before
/// fallible work and return one owned `IUnknown` reference only on success.
pub type NcActivatePluginFactory = unsafe extern "C" fn(out: *mut *mut IUnknown) -> NcStatus;

/// Adapts a safe typed root-factory constructor to the fixed plugin ABI.
///
/// The generic is monomorphized in the plugin assembly. The contract library
/// still owns the factory identity and callable description; this adapter only
/// transfers the implementation's newly created reference to the runtime.
///
/// # Safety
///
/// `out` must be valid for one pointer write. It is always cleared before the
/// implementation's fallible bootstrap code runs.
pub unsafe extern "C" fn activate_factory_implementation<T>(out: *mut *mut IUnknown) -> NcStatus
where
    T: ComFactoryBootstrap,
{
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: The caller grants one output write and null was rejected above.
    unsafe { *out = core::ptr::null_mut() };
    let factory = match T::create_factory() {
        Ok(factory) => factory,
        Err(status) => return status.normalize_error(),
    };
    // All generated interfaces carry the IUnknown ABI prefix at the same
    // address. Consuming the typed pointer transfers its existing ownership
    // into the untyped bootstrap slot without AddRef or Release.
    let raw_factory = factory.into_raw();
    let unknown = T::Description::iunknown(raw_factory);
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

/// Immutable plugin metadata containing contracts and one root factory.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcFactoryPluginDescription {
    /// Core object ABI generation understood by this plugin.
    pub core_abi_version: u32,
    /// ABI signature representation compiled into generated interface IDs.
    pub validation_mode: NcAbiValidationMode,
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
    /// Number of directly referenced canonical contract modules.
    pub contract_count: u32,
    /// Pointer to `contract_count` generated module references.
    pub contracts: *const NcContractModuleReference,
    /// IDL identity of the root factory returned by `activate`.
    pub root_factory_id: NcGuid,
    /// Threading capability promised by the returned root factory.
    pub root_factory_threading: u32,
    /// Fixed no-argument root factory bootstrap operation.
    pub activate: Option<NcActivatePluginFactory>,
}

// SAFETY: The description and its tables are immutable static metadata. The
// activation pointer is code in the library that owns the description.
unsafe impl Sync for NcFactoryPluginDescription {}

impl NcFactoryPluginDescription {
    /// Borrows hard plugin dependencies after checking pointer/count shape.
    ///
    /// # Safety
    ///
    /// The description and dependency table must remain readable for the
    /// returned lifetime.
    pub unsafe fn dependencies<'a>(&self) -> Result<&'a [NcPluginDependencyDescriptor], NcStatus> {
        // SAFETY: The caller supplies the backing lifetime; the helper checks
        // null/count shape before constructing a slice.
        unsafe { description_slice(self.dependencies, self.dependency_count) }
    }

    /// Borrows direct contract references after checking pointer/count shape.
    ///
    /// # Safety
    ///
    /// The description and contract table must remain readable for the
    /// returned lifetime.
    pub unsafe fn contracts<'a>(&self) -> Result<&'a [NcContractModuleReference], NcStatus> {
        // SAFETY: The caller supplies the backing lifetime; the helper checks
        // null/count shape before constructing a slice.
        unsafe { description_slice(self.contracts, self.contract_count) }
    }
}

/// Getter exported by one factory-only plugin shared library.
///
/// # Safety
///
/// `out` must be valid for one pointer write. On success it receives immutable
/// metadata that remains live while the exporting library is mapped.
pub type NcGetFactoryPluginDescription = unsafe extern "C" fn(out: *mut *const NcFactoryPluginDescription) -> NcStatus;

/// Validates one complete factory-only plugin description without allocation.
///
/// This structural pass does not call contract getters or the plugin activation
/// function. The shared runtime performs those operations after this bounded
/// pointer validation succeeds.
///
/// # Safety
///
/// `plugin` must be null or point to readable metadata whose pointer/count
/// tables and strings remain readable for this call.
pub unsafe fn validate_factory_plugin_description(plugin: *const NcFactoryPluginDescription) -> NcStatus {
    if plugin.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: Null was rejected and the caller guarantees root readability.
    let plugin = unsafe { &*plugin };
    if plugin.core_abi_version != NC_COM_CORE_ABI_VERSION {
        return NC_E_ABI_VERSION;
    }
    if NcAbiValidationMode::from_raw(plugin.validation_mode.as_raw()).is_none() {
        return NC_E_INVALID_ARG;
    }
    #[cfg(any(
        feature = "com-validation-none",
        feature = "com-validation-sha-256",
        feature = "com-validation-fnv-1a-128",
        feature = "com-validation-mangling"
    ))]
    if plugin.validation_mode != NC_ABI_VALIDATION_MODE {
        return NC_E_ABI_VERSION;
    }
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
        || plugin.root_factory_id.is_nil()
        || NcAbiThreadingMode::from_raw(plugin.root_factory_threading).is_none()
        || plugin.activate.is_none()
    {
        return NC_E_INVALID_ARG;
    }
    // SAFETY: The caller guarantees both table lifetimes for this validation.
    let dependencies = match unsafe { plugin.dependencies() } {
        Ok(dependencies) => dependencies,
        Err(status) => return status,
    };
    // SAFETY: The caller guarantees both table lifetimes for this validation.
    let contracts = match unsafe { plugin.contracts() } {
        Ok(contracts) => contracts,
        Err(status) => return status,
    };
    if contracts.is_empty() {
        return NC_E_INVALID_ARG;
    }
    for (index, dependency) in dependencies.iter().enumerate() {
        if dependency.plugin_id.is_nil()
            || dependency.plugin_id == plugin.plugin_id
            || dependencies[..index].iter().any(|previous| previous.plugin_id == dependency.plugin_id)
        {
            return NC_E_INVALID_ARG;
        }
    }
    for (index, contract) in contracts.iter().enumerate() {
        if contract.module_id.is_nil()
            || contract.abi_fingerprint.bytes == [0; 32]
            || contract.get.is_none()
            || contracts[..index].iter().any(|previous| previous.module_id == contract.module_id)
        {
            return NC_E_INVALID_ARG;
        }
    }
    NC_OK
}

/// Publishes one validated static factory plugin description through an export.
///
/// # Safety
///
/// `out` must be null or writable for one pointer. `plugin` must remain valid
/// while its exporting library is loaded.
pub unsafe fn return_factory_plugin_description(out: *mut *const NcFactoryPluginDescription, plugin: &'static NcFactoryPluginDescription) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // Clear before validation so a failure never publishes stale metadata.
    // SAFETY: The caller guarantees one writable output slot.
    unsafe { *out = core::ptr::null() };
    // SAFETY: A shared reference proves the root is readable; macro-generated
    // descriptions point only to immutable static tables.
    let status = unsafe { validate_factory_plugin_description(plugin) };
    if status.is_err() {
        return status;
    }
    // SAFETY: The output slot was checked above and validation succeeded.
    unsafe { *out = plugin };
    NC_OK
}

/// Constructs a slice from one immutable description pointer/count pair.
///
/// # Safety
///
/// A non-null `pointer` must address `count` initialized readable values that
/// remain valid for the returned lifetime.
unsafe fn description_slice<'a, T>(pointer: *const T, count: u32) -> Result<&'a [T], NcStatus> {
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
    use core::ptr;

    use super::*;
    use crate::{NC_ABI_THREADING_THREAD_LOCAL, NcContractFingerprint};

    /// Stable logical identity of the contract module used by the fixture.
    const MODULE_ID: NcGuid = NcGuid::from_fields(1, 2, 3, [4; 8]);
    /// Stable nonzero fingerprint representing one exact test module revision.
    const MODULE_FINGERPRINT: NcContractFingerprint = NcContractFingerprint::new([5; 32]);
    /// Stable IDL identity of the fixture's root factory declaration.
    const FACTORY_ID: NcGuid = NcGuid::from_fields(6, 7, 8, [9; 8]);

    /// Satisfies the contract-reference function shape without being invoked.
    unsafe extern "C" fn get_contract_module() -> *const crate::NcContractModuleDescription {
        // Structural plugin validation deliberately does not enter linked
        // contract code. The runtime resolves this getter during registration.
        ptr::null()
    }

    /// Satisfies the fixed factory activation shape for descriptor validation.
    unsafe extern "C" fn activate_factory(out: *mut *mut IUnknown) -> NcStatus {
        if out.is_null() {
            return NC_E_POINTER;
        }
        // This fixture verifies metadata only, so it reports a predictable
        // activation failure after honoring the required cleared-output rule.
        // SAFETY: Null was rejected and the ABI contract grants one write.
        unsafe { *out = ptr::null_mut() };
        NC_E_INVALID_ARG
    }

    /// One direct contract revision required by the valid plugin fixture.
    static CONTRACTS: [NcContractModuleReference; 1] = [NcContractModuleReference {
        module_id: MODULE_ID,
        abi_fingerprint: MODULE_FINGERPRINT,
        get: Some(get_contract_module),
    }];

    /// Complete factory-only plugin fixture with no plugin dependencies.
    static VALID_PLUGIN: NcFactoryPluginDescription = NcFactoryPluginDescription {
        core_abi_version: NC_COM_CORE_ABI_VERSION,
        validation_mode: NC_ABI_VALIDATION_MODE,
        plugin_id: guid_from_qualified_name("nc.tests.factory_plugin"),
        qualified_name: NcStaticUtf8::from_static("nc.tests.factory_plugin"),
        plugin_version: NcStaticUtf8::from_static("1.0.0"),
        dependency_count: 0,
        dependencies: ptr::null(),
        contract_count: CONTRACTS.len() as u32,
        contracts: CONTRACTS.as_ptr(),
        root_factory_id: FACTORY_ID,
        root_factory_threading: NC_ABI_THREADING_THREAD_LOCAL,
        activate: Some(activate_factory),
    };

    /// Confirms valid metadata publishes the exact immutable description.
    #[test]
    fn validates_and_returns_factory_plugin_description() {
        // SAFETY: The fixture and all referenced metadata are immutable statics.
        assert_eq!(unsafe { validate_factory_plugin_description(&VALID_PLUGIN) }, NC_OK);
        let mut returned = ptr::null();
        // SAFETY: `returned` provides one writable pointer slot.
        assert_eq!(unsafe { return_factory_plugin_description(&mut returned, &VALID_PLUGIN) }, NC_OK);
        assert!(ptr::eq(returned, &VALID_PLUGIN));
    }

    /// Confirms plugin identity cannot disagree with its normative name.
    #[test]
    fn rejects_a_plugin_id_unrelated_to_its_name() {
        let invalid = NcFactoryPluginDescription {
            plugin_id: NcGuid::from_fields(99, 2, 3, [4; 8]),
            ..VALID_PLUGIN
        };
        // SAFETY: The stack root borrows only immutable static child tables.
        assert_eq!(unsafe { validate_factory_plugin_description(&invalid) }, NC_E_INVALID_ARG);
    }

    /// Confirms an unknown interface-signature representation is rejected.
    #[test]
    fn rejects_an_unknown_validation_mode() {
        let invalid = NcFactoryPluginDescription {
            // SAFETY: The transparent wrapper has exactly the complete `u32`
            // value domain; validation must still reject unrecognized tags.
            validation_mode: unsafe { core::mem::transmute::<u32, NcAbiValidationMode>(u32::MAX) },
            ..VALID_PLUGIN
        };
        // SAFETY: The stack root borrows only immutable static child tables.
        assert_eq!(unsafe { validate_factory_plugin_description(&invalid) }, NC_E_INVALID_ARG);
    }

    /// Locks the new descriptor's field order to its documented C ABI layout.
    #[test]
    fn factory_plugin_description_layout_is_c_stable() {
        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(core::mem::size_of::<NcFactoryPluginDescription>(), 120);
            assert_eq!(core::mem::align_of::<NcFactoryPluginDescription>(), 8);
            assert_eq!(core::mem::offset_of!(NcFactoryPluginDescription, validation_mode), 4);
            assert_eq!(core::mem::offset_of!(NcFactoryPluginDescription, plugin_id), 8);
            assert_eq!(core::mem::offset_of!(NcFactoryPluginDescription, dependencies), 64);
            assert_eq!(core::mem::offset_of!(NcFactoryPluginDescription, contracts), 80);
            assert_eq!(core::mem::offset_of!(NcFactoryPluginDescription, root_factory_id), 88);
            assert_eq!(core::mem::offset_of!(NcFactoryPluginDescription, activate), 112);
        }
        #[cfg(target_pointer_width = "32")]
        {
            assert_eq!(core::mem::size_of::<NcFactoryPluginDescription>(), 80);
            assert_eq!(core::mem::align_of::<NcFactoryPluginDescription>(), 4);
            assert_eq!(core::mem::offset_of!(NcFactoryPluginDescription, validation_mode), 4);
            assert_eq!(core::mem::offset_of!(NcFactoryPluginDescription, plugin_id), 8);
            assert_eq!(core::mem::offset_of!(NcFactoryPluginDescription, dependencies), 44);
            assert_eq!(core::mem::offset_of!(NcFactoryPluginDescription, contracts), 52);
            assert_eq!(core::mem::offset_of!(NcFactoryPluginDescription, root_factory_id), 56);
            assert_eq!(core::mem::offset_of!(NcFactoryPluginDescription, activate), 76);
        }
    }
}
