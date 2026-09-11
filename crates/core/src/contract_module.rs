//! Canonical C ABI descriptions exported by generated contract libraries.
//!
//! Hosts and plugins compile typed call layouts, but neither embeds this graph.
//! One fingerprint-qualified contract library owns the module name, source,
//! declaration identities, and exact dependency getter references.

use crate::{NC_E_ABI_VERSION, NC_E_CONTRACT_MISMATCH, NC_E_INVALID_ARG, NC_E_POINTER, NC_OK, NcGuid, NcStaticUtf8, NcStatus};

/// ABI version of [`NcContractModuleDescription`].
pub const NC_CONTRACT_DESCRIPTION_ABI_VERSION: u32 = 1;

/// Raw declaration-kind tag for an IDL struct.
pub const NC_CONTRACT_DECLARATION_STRUCT: u32 = 1;
/// Raw declaration-kind tag for an IDL enum.
pub const NC_CONTRACT_DECLARATION_ENUM: u32 = 2;
/// Raw declaration-kind tag for an IDL trait interface.
pub const NC_CONTRACT_DECLARATION_INTERFACE: u32 = 3;
/// Raw declaration-kind tag for an IDL class.
pub const NC_CONTRACT_DECLARATION_CLASS: u32 = 4;
/// Raw declaration-kind tag for an IDL factory.
pub const NC_CONTRACT_DECLARATION_FACTORY: u32 = 5;
/// Threading sentinel used by declarations that are not classes or factories.
pub const NC_CONTRACT_DECLARATION_NO_THREADING: u32 = u32::MAX;

/// Fixed-size SHA-256 fingerprint used by contract module identities.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NcContractFingerprint {
    /// Canonical digest bytes in ordinary SHA-256 display order.
    pub bytes: [u8; 32],
}

impl NcContractFingerprint {
    /// Creates a fingerprint from its complete canonical digest.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }
}

/// Getter exported by one fingerprint-qualified contract library.
///
/// # Safety
///
/// The returned pointer must be non-null and identify immutable metadata that
/// remains valid while the exporting library is loaded.
pub type NcGetContractModuleDescription = unsafe extern "C" fn() -> *const NcContractModuleDescription;

/// Stable reference compiled into a host or plugin by generated Rust bindings.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcContractModuleReference {
    /// Logical module identity expected from the getter.
    pub module_id: NcGuid,
    /// Exact ABI fingerprint expected from the getter.
    pub abi_fingerprint: NcContractFingerprint,
    /// Fingerprint-qualified getter imported from the contract library.
    pub get: Option<NcGetContractModuleDescription>,
}

// SAFETY: A module reference contains immutable values and a function pointer;
// it does not grant access after the defining library is unloaded.
unsafe impl Sync for NcContractModuleReference {}

impl NcContractModuleReference {
    /// Loads and verifies this exact linked module description.
    ///
    /// # Safety
    ///
    /// The contract library defining `get` and every transitive dependency
    /// library must remain loaded for the returned borrow.
    pub unsafe fn description<'a>(&self) -> Result<&'a NcContractModuleDescription, NcStatus> {
        let Some(get) = self.get else {
            return Err(NC_E_POINTER);
        };
        // SAFETY: The caller guarantees the linked getter remains callable.
        let description = unsafe { get() };
        if description.is_null() {
            return Err(NC_E_POINTER);
        }
        // SAFETY: The getter contract requires a live immutable description.
        let description = unsafe { &*description };
        if description.module_id != self.module_id || description.abi_fingerprint != self.abi_fingerprint {
            return Err(NC_E_CONTRACT_MISMATCH);
        }
        Ok(description)
    }
}

/// One direct dependency and its exact linked description getter.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcContractDependencyDescription {
    /// Stable logical identity of the imported module.
    pub module_id: NcGuid,
    /// Exact transitive ABI fingerprint required by this module revision.
    pub abi_fingerprint: NcContractFingerprint,
    /// Getter imported from the fingerprint-qualified dependency library.
    pub get: Option<NcGetContractModuleDescription>,
}

// SAFETY: Dependency descriptions are immutable static values plus a function
// pointer into a library retained by the platform loader.
unsafe impl Sync for NcContractDependencyDescription {}

/// One top-level IDL declaration retained for identity inspection.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcContractDeclarationDescription {
    /// Derived or explicitly assigned declaration ID.
    pub declaration_id: NcGuid,
    /// Positive IDL declaration version.
    pub version: u32,
    /// One `NC_CONTRACT_DECLARATION_*` tag.
    pub kind: u32,
    /// Class/factory threading capability, or
    /// [`NC_CONTRACT_DECLARATION_NO_THREADING`] for other declarations.
    pub threading: u32,
    /// Exact dotted module and declared name.
    pub qualified_name: NcStaticUtf8,
}

// SAFETY: Declaration descriptions contain only immutable value metadata and a
// library-owned static string view.
unsafe impl Sync for NcContractDeclarationDescription {}

/// Canonical description root exported once by a contract library.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcContractModuleDescription {
    /// Must equal [`NC_CONTRACT_DESCRIPTION_ABI_VERSION`].
    pub description_abi_version: u32,
    /// Stable identity derived from the exact dotted module name.
    pub module_id: NcGuid,
    /// Complete canonical ABI fingerprint including dependency revisions.
    pub abi_fingerprint: NcContractFingerprint,
    /// Exact dotted logical module name.
    pub module_name: NcStaticUtf8,
    /// Number of entries addressed by `dependencies`.
    pub dependency_count: u32,
    /// Direct dependencies in canonical module-name order.
    pub dependencies: *const NcContractDependencyDescription,
    /// Number of entries addressed by `declarations`.
    pub declaration_count: u32,
    /// Top-level declarations in source order.
    pub declarations: *const NcContractDeclarationDescription,
    /// Complete canonical `.ncidl` source for tooling and diagnostics.
    pub source: NcStaticUtf8,
}

// SAFETY: Module descriptions are immutable library-owned pointer graphs.
unsafe impl Sync for NcContractModuleDescription {}

impl NcContractModuleDescription {
    /// Borrows direct dependency descriptions after checking pointer shape.
    ///
    /// # Safety
    ///
    /// This module description and its library-owned tables must remain loaded
    /// and readable for the returned lifetime.
    pub unsafe fn dependencies<'a>(&self) -> Result<&'a [NcContractDependencyDescription], NcStatus> {
        // SAFETY: The caller supplies table lifetime; the helper validates its
        // null/count shape before constructing a Rust slice.
        unsafe { descriptor_slice(self.dependencies, self.dependency_count) }
    }

    /// Borrows top-level declaration descriptions after pointer validation.
    ///
    /// # Safety
    ///
    /// This module description and its library-owned tables must remain loaded
    /// and readable for the returned lifetime.
    pub unsafe fn declarations<'a>(&self) -> Result<&'a [NcContractDeclarationDescription], NcStatus> {
        // SAFETY: The caller supplies table lifetime; the helper validates its
        // null/count shape before constructing a Rust slice.
        unsafe { descriptor_slice(self.declarations, self.declaration_count) }
    }
}

/// Validates the bounded shape and identity fields of one module description.
///
/// Dependency getters are checked for matching IDs and fingerprints by the
/// runtime's graph registration pass rather than called recursively here.
///
/// # Safety
///
/// `description` must be null or point to a readable root whose pointer/count
/// tables and UTF-8 views remain readable for this call.
pub unsafe fn validate_contract_module_description(description: *const NcContractModuleDescription) -> NcStatus {
    if description.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: Null was rejected and the caller guarantees root readability.
    let description = unsafe { &*description };
    if description.description_abi_version != NC_CONTRACT_DESCRIPTION_ABI_VERSION {
        return NC_E_ABI_VERSION;
    }
    if description.module_id.is_nil() || description.abi_fingerprint.bytes == [0; 32] {
        return NC_E_INVALID_ARG;
    }
    // SAFETY: The caller guarantees string readability for this validation.
    let Ok(module_name) = (unsafe { description.module_name.as_str() }) else {
        return NC_E_INVALID_ARG;
    };
    if module_name.is_empty() {
        return NC_E_INVALID_ARG;
    }
    // SAFETY: The caller guarantees both table pointer/count pairs are readable.
    let dependencies = match unsafe { description.dependencies() } {
        Ok(dependencies) => dependencies,
        Err(status) => return status,
    };
    // SAFETY: The caller guarantees both table pointer/count pairs are readable.
    let declarations = match unsafe { description.declarations() } {
        Ok(declarations) => declarations,
        Err(status) => return status,
    };
    for dependency in dependencies {
        if dependency.module_id.is_nil()
            || dependency.module_id == description.module_id
            || dependency.abi_fingerprint.bytes == [0; 32]
            || dependency.get.is_none()
        {
            return NC_E_INVALID_ARG;
        }
    }
    for declaration in declarations {
        let valid_kind = (NC_CONTRACT_DECLARATION_STRUCT..=NC_CONTRACT_DECLARATION_FACTORY).contains(&declaration.kind);
        let valid_threading = if matches!(declaration.kind, NC_CONTRACT_DECLARATION_CLASS | NC_CONTRACT_DECLARATION_FACTORY) {
            crate::NcAbiThreadingMode::from_raw(declaration.threading).is_some()
        } else {
            declaration.threading == NC_CONTRACT_DECLARATION_NO_THREADING
        };
        if declaration.declaration_id.is_nil() || declaration.version == 0 || !valid_kind || !valid_threading {
            return NC_E_INVALID_ARG;
        }
        // SAFETY: The caller guarantees declaration string readability.
        let Ok(name) = (unsafe { declaration.qualified_name.as_str() }) else {
            return NC_E_INVALID_ARG;
        };
        if name.is_empty() {
            return NC_E_INVALID_ARG;
        }
    }
    // SAFETY: The caller guarantees source string readability.
    if unsafe { description.source.as_str() }.is_err() {
        return NC_E_INVALID_ARG;
    }
    NC_OK
}

/// Constructs a slice from an immutable descriptor pointer/count pair.
///
/// # Safety
///
/// A non-null `pointer` must address `count` readable initialized values that
/// remain valid for the returned lifetime.
unsafe fn descriptor_slice<'a, T>(pointer: *const T, count: u32) -> Result<&'a [T], NcStatus> {
    if count == 0 {
        return Ok(&[]);
    }
    if pointer.is_null() {
        return Err(NC_E_POINTER);
    }
    // SAFETY: The caller supplies the exact readability and lifetime contract;
    // the null/count shape was checked above.
    Ok(unsafe { core::slice::from_raw_parts(pointer, count as usize) })
}

#[cfg(test)]
mod tests {
    use core::ptr;

    use super::*;

    /// Returns the test description through the same ABI as a linked library.
    unsafe extern "C" fn get_test_module() -> *const NcContractModuleDescription {
        &VALID_MODULE
    }

    /// Stable test module identity.
    const MODULE_ID: NcGuid = NcGuid::from_fields(1, 2, 3, [4; 8]);
    /// Stable nonzero test fingerprint.
    const FINGERPRINT: NcContractFingerprint = NcContractFingerprint::new([7; 32]);
    /// One declaration in the valid fixture.
    static DECLARATIONS: [NcContractDeclarationDescription; 1] = [NcContractDeclarationDescription {
        declaration_id: NcGuid::from_fields(8, 9, 10, [11; 8]),
        version: 1,
        kind: NC_CONTRACT_DECLARATION_INTERFACE,
        threading: NC_CONTRACT_DECLARATION_NO_THREADING,
        qualified_name: NcStaticUtf8::from_static("nc.tests.IValue"),
    }];
    /// Complete valid leaf module fixture.
    static VALID_MODULE: NcContractModuleDescription = NcContractModuleDescription {
        description_abi_version: NC_CONTRACT_DESCRIPTION_ABI_VERSION,
        module_id: MODULE_ID,
        abi_fingerprint: FINGERPRINT,
        module_name: NcStaticUtf8::from_static("nc.tests"),
        dependency_count: 0,
        dependencies: ptr::null(),
        declaration_count: DECLARATIONS.len() as u32,
        declarations: DECLARATIONS.as_ptr(),
        source: NcStaticUtf8::from_static("#[version(1)] trait IValue {}"),
    };

    #[test]
    fn validates_and_resolves_an_exact_module_reference() {
        // SAFETY: The fixture and getter reference immutable test statics.
        assert_eq!(unsafe { validate_contract_module_description(&VALID_MODULE) }, NC_OK);
        let reference = NcContractModuleReference {
            module_id: MODULE_ID,
            abi_fingerprint: FINGERPRINT,
            get: Some(get_test_module),
        };
        // SAFETY: The getter and returned static remain live for this test.
        let description = unsafe { reference.description() }.unwrap();
        assert_eq!(description.module_id, MODULE_ID);
    }

    #[test]
    fn rejects_null_and_mismatched_roots() {
        // SAFETY: Null is explicitly accepted as a validation input.
        assert_eq!(unsafe { validate_contract_module_description(ptr::null()) }, NC_E_POINTER);
        let reference = NcContractModuleReference {
            module_id: NcGuid::from_fields(99, 2, 3, [4; 8]),
            abi_fingerprint: FINGERPRINT,
            get: Some(get_test_module),
        };
        // SAFETY: The getter remains live; the expected ID is deliberately wrong.
        assert_eq!(unsafe { reference.description() }.err(), Some(NC_E_CONTRACT_MISMATCH));
    }

    #[test]
    fn contract_description_layouts_are_c_stable() {
        if cfg!(target_pointer_width = "64") {
            assert_eq!(core::mem::size_of::<NcContractModuleReference>(), 56);
            assert_eq!(core::mem::size_of::<NcContractDependencyDescription>(), 56);
            assert_eq!(core::mem::size_of::<NcContractDeclarationDescription>(), 48);
            assert_eq!(core::mem::size_of::<NcContractModuleDescription>(), 120);
            assert_eq!(core::mem::offset_of!(NcContractModuleDescription, dependencies), 80);
            assert_eq!(core::mem::offset_of!(NcContractModuleDescription, declarations), 96);
            assert_eq!(core::mem::offset_of!(NcContractModuleDescription, source), 104);
        }
        if cfg!(target_pointer_width = "32") {
            assert_eq!(core::mem::size_of::<NcContractModuleReference>(), 52);
            assert_eq!(core::mem::size_of::<NcContractDependencyDescription>(), 52);
            assert_eq!(core::mem::size_of::<NcContractDeclarationDescription>(), 36);
            assert_eq!(core::mem::size_of::<NcContractModuleDescription>(), 84);
            assert_eq!(core::mem::offset_of!(NcContractModuleDescription, dependencies), 64);
            assert_eq!(core::mem::offset_of!(NcContractModuleDescription, declarations), 72);
            assert_eq!(core::mem::offset_of!(NcContractModuleDescription, source), 76);
        }
    }
}
