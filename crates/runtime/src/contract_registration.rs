//! Validation and dependency-first projection of native contract module graphs.
//!
//! Getter calls and raw descriptor traversal happen before the runtime catalog
//! write lock is acquired. The resulting owned batch contains only validated
//! identities and immutable description pointers, so publication is short and
//! cannot expose a partially registered dependency graph.

use std::collections::{HashMap, HashSet};
use std::string::String;
use std::vec::Vec;

use neocogi_com_core::{
    NC_E_CONTRACT_MISMATCH, NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE, NcContractFingerprint, NcContractModuleReference, NcGetContractModuleDescription, NcGuid,
    NcStatus, validate_contract_module_description,
};

use crate::contract_registry::RegisteredContractModule;
use crate::state::RuntimeState;

/// Maximum number of unique modules accepted from one dependency traversal.
const MAX_REGISTRATION_GRAPH_MODULES: usize = 4_096;

/// Dependency-first records prepared for one atomic registry transaction.
pub(crate) struct ContractRegistrationBatch {
    /// Validated modules ordered so every dependency precedes its dependent.
    modules: Vec<RegisteredContractModule>,
}

impl ContractRegistrationBatch {
    /// Transfers the validated dependency-first records to registry storage.
    pub(crate) fn into_modules(self) -> Vec<RegisteredContractModule> {
        self.modules
    }
}

/// Expected identity paired with a getter reached from a root or dependency.
#[derive(Clone, Copy)]
struct PendingModule {
    /// Logical module ID required by the referring description.
    module_id: NcGuid,
    /// Exact transitive ABI fingerprint required by the referring description.
    abi_fingerprint: NcContractFingerprint,
    /// Native getter expected to return this exact module revision.
    get: NcGetContractModuleDescription,
}

/// Explicit depth-first traversal frame that avoids recursive stack growth.
enum Visit {
    /// Validate one getter and schedule each direct dependency.
    Enter(PendingModule),
    /// Publish one record after every scheduled dependency has completed.
    Exit(RegisteredContractModule),
}

/// Identity already observed while building one registration batch.
struct SeenModule {
    /// Exact transitive fingerprint first observed for this module ID.
    abi_fingerprint: NcContractFingerprint,
    /// Validated module name copied from the canonical description.
    module_name: String,
}

/// Builds one dependency-first registration batch from a native module root.
///
/// # Safety
///
/// `reference` must be null or point to an immutable module reference whose
/// getter and every transitively returned description remain loaded while the
/// resulting batch is registered.
pub(crate) unsafe fn build_contract_registration_batch(reference: *const NcContractModuleReference) -> Result<ContractRegistrationBatch, NcStatus> {
    if reference.is_null() {
        return Err(neocogi_com_core::NC_E_POINTER);
    }
    // SAFETY: Null was rejected and the caller guarantees reference readability.
    let reference = unsafe { &*reference };
    let Some(get) = reference.get else {
        return Err(neocogi_com_core::NC_E_POINTER);
    };

    let mut visits = vec![Visit::Enter(PendingModule {
        module_id: reference.module_id,
        abi_fingerprint: reference.abi_fingerprint,
        get,
    })];
    let mut active = HashSet::new();
    let mut seen = HashMap::<NcGuid, SeenModule>::new();
    let mut modules = Vec::new();

    while let Some(visit) = visits.pop() {
        match visit {
            Visit::Exit(module) => {
                // An exit frame is scheduled only after its matching enter
                // inserted the ID, so removal must succeed for an acyclic graph.
                debug_assert!(active.remove(&module.module_id));
                modules.push(module);
            }
            Visit::Enter(pending) => {
                // SAFETY: The caller guarantees every getter reached through
                // the immutable native graph remains callable for registration.
                let description = unsafe { (pending.get)() };
                // SAFETY: The getter contract permits null as an invalid result;
                // otherwise its complete pointer graph remains readable here.
                let status = unsafe { validate_contract_module_description(description) };
                if status.is_err() {
                    return Err(status);
                }
                // SAFETY: Successful validation rejected null and established
                // readable identity and UTF-8 fields.
                let description = unsafe { &*description };
                if description.module_id != pending.module_id || description.abi_fingerprint != pending.abi_fingerprint {
                    return Err(NC_E_CONTRACT_MISMATCH);
                }
                // SAFETY: Whole-root validation established valid module text.
                let module_name = unsafe { description.module_name.as_str() }.map_err(|_| NC_E_INVALID_ARG)?;

                if active.contains(&description.module_id) {
                    // Generated IDL graphs are acyclic. Encountering an active
                    // node therefore identifies a forged or corrupted graph.
                    return Err(NC_E_INVALID_ARG);
                }
                if let Some(previous) = seen.get(&description.module_id) {
                    if previous.abi_fingerprint != description.abi_fingerprint {
                        return Err(NC_E_CONTRACT_MISMATCH);
                    }
                    if previous.module_name != module_name {
                        return Err(NC_E_INVALID_ARG);
                    }
                    // A completed diamond dependency is already present in the
                    // dependency-first output and needs no second publication.
                    continue;
                }
                if seen.len() == MAX_REGISTRATION_GRAPH_MODULES {
                    return Err(NC_E_OUT_OF_RANGE);
                }

                let record = RegisteredContractModule {
                    module_id: description.module_id,
                    abi_fingerprint: description.abi_fingerprint,
                    description: core::ptr::from_ref(description),
                    module_name: module_name.to_owned(),
                };
                seen.insert(
                    description.module_id,
                    SeenModule {
                        abi_fingerprint: description.abi_fingerprint,
                        module_name: module_name.to_owned(),
                    },
                );
                active.insert(description.module_id);
                visits.push(Visit::Exit(record));

                // Reverse scheduling preserves the canonical source order when
                // the LIFO traversal later appends dependency records.
                // SAFETY: Root validation established this table's pointer/count shape.
                let dependencies = unsafe { description.dependencies() }?;
                for dependency in dependencies.iter().rev() {
                    let Some(get) = dependency.get else {
                        return Err(neocogi_com_core::NC_E_POINTER);
                    };
                    visits.push(Visit::Enter(PendingModule {
                        module_id: dependency.module_id,
                        abi_fingerprint: dependency.abi_fingerprint,
                        get,
                    }));
                }
            }
        }
    }

    Ok(ContractRegistrationBatch { modules })
}

impl RuntimeState {
    /// Validates and atomically registers one complete native contract graph.
    ///
    /// # Safety
    ///
    /// `reference` and every native description reached from it must remain
    /// readable while registered.
    pub(crate) unsafe fn register_contract_module(&self, reference: *const NcContractModuleReference) -> NcStatus {
        // SAFETY: This forwards the caller's complete native graph lifetime.
        let batch = match unsafe { build_contract_registration_batch(reference) } {
            Ok(batch) => batch,
            Err(status) => return status,
        };
        self.write_catalog().contract_registry.commit(batch)
    }
}

#[cfg(test)]
mod tests {
    use core::ptr;

    use neocogi_com_core::{
        NC_CONTRACT_DESCRIPTION_ABI_VERSION, NC_E_CONTRACT_MISMATCH, NC_E_INVALID_ARG, NC_OK, NcContractDependencyDescription, NcContractModuleDescription,
        NcStaticUtf8,
    };

    use super::*;

    /// Stable identity of the leaf module fixture.
    const LEAF_ID: NcGuid = NcGuid::from_fields(1, 2, 3, [4; 8]);
    /// Stable identity of the root module fixture.
    const ROOT_ID: NcGuid = NcGuid::from_fields(5, 6, 7, [8; 8]);
    /// Exact leaf revision required by the root.
    const LEAF_FINGERPRINT: NcContractFingerprint = NcContractFingerprint::new([1; 32]);
    /// Exact root revision registered by the test.
    const ROOT_FINGERPRINT: NcContractFingerprint = NcContractFingerprint::new([2; 32]);

    /// Returns the immutable leaf fixture through the native getter ABI.
    unsafe extern "C" fn get_leaf() -> *const NcContractModuleDescription {
        &LEAF
    }

    /// Returns the immutable root fixture through the native getter ABI.
    unsafe extern "C" fn get_root() -> *const NcContractModuleDescription {
        &ROOT
    }

    /// Canonical leaf module with no dependencies or declarations.
    static LEAF: NcContractModuleDescription = NcContractModuleDescription {
        description_abi_version: NC_CONTRACT_DESCRIPTION_ABI_VERSION,
        module_id: LEAF_ID,
        abi_fingerprint: LEAF_FINGERPRINT,
        module_name: NcStaticUtf8::from_static("nc.tests.leaf"),
        dependency_count: 0,
        dependencies: ptr::null(),
        declaration_count: 0,
        declarations: ptr::null(),
    };

    /// Root-to-leaf edge containing both the expected identity and getter.
    static ROOT_DEPENDENCIES: [NcContractDependencyDescription; 1] = [NcContractDependencyDescription {
        module_id: LEAF_ID,
        abi_fingerprint: LEAF_FINGERPRINT,
        get: Some(get_leaf),
    }];

    /// Canonical root module requiring the exact leaf fixture revision.
    static ROOT: NcContractModuleDescription = NcContractModuleDescription {
        description_abi_version: NC_CONTRACT_DESCRIPTION_ABI_VERSION,
        module_id: ROOT_ID,
        abi_fingerprint: ROOT_FINGERPRINT,
        module_name: NcStaticUtf8::from_static("nc.tests.root"),
        dependency_count: ROOT_DEPENDENCIES.len() as u32,
        dependencies: ROOT_DEPENDENCIES.as_ptr(),
        declaration_count: 0,
        declarations: ptr::null(),
    };

    /// Returns a conflicting revision under the root's logical module ID.
    unsafe extern "C" fn get_conflicting_root() -> *const NcContractModuleDescription {
        &CONFLICTING_ROOT
    }

    /// Deliberately incompatible root revision used for collision testing.
    static CONFLICTING_ROOT: NcContractModuleDescription = NcContractModuleDescription {
        description_abi_version: NC_CONTRACT_DESCRIPTION_ABI_VERSION,
        module_id: ROOT_ID,
        abi_fingerprint: NcContractFingerprint::new([9; 32]),
        module_name: NcStaticUtf8::from_static("nc.tests.root"),
        dependency_count: 0,
        dependencies: ptr::null(),
        declaration_count: 0,
        declarations: ptr::null(),
    };

    #[test]
    fn registers_dependencies_first_and_rejects_another_revision_atomically() {
        let runtime = RuntimeState::new();
        let root = NcContractModuleReference {
            module_id: ROOT_ID,
            abi_fingerprint: ROOT_FINGERPRINT,
            get: Some(get_root),
        };
        // SAFETY: Every fixture is immutable static metadata.
        assert_eq!(unsafe { runtime.register_contract_module(&root) }, NC_OK);
        let catalog = runtime.read_catalog();
        catalog.contract_registry.with_module_ids(|ids| assert_eq!(ids, &[LEAF_ID, ROOT_ID]));
        drop(catalog);

        let conflict = NcContractModuleReference {
            module_id: ROOT_ID,
            abi_fingerprint: NcContractFingerprint::new([9; 32]),
            get: Some(get_conflicting_root),
        };
        // SAFETY: The conflicting fixture is also immutable static metadata.
        assert_eq!(unsafe { runtime.register_contract_module(&conflict) }, NC_E_CONTRACT_MISMATCH);
        runtime
            .read_catalog()
            .contract_registry
            .with_module_ids(|ids| assert_eq!(ids, &[LEAF_ID, ROOT_ID]));
    }

    /// Returns cycle fixture A.
    unsafe extern "C" fn get_cycle_a() -> *const NcContractModuleDescription {
        &CYCLE_A
    }

    /// Returns cycle fixture B.
    unsafe extern "C" fn get_cycle_b() -> *const NcContractModuleDescription {
        &CYCLE_B
    }

    /// Stable identity of cycle fixture A.
    const CYCLE_A_ID: NcGuid = NcGuid::from_fields(10, 2, 3, [4; 8]);
    /// Stable identity of cycle fixture B.
    const CYCLE_B_ID: NcGuid = NcGuid::from_fields(11, 2, 3, [4; 8]);
    /// Fingerprint of cycle fixture A.
    const CYCLE_A_FINGERPRINT: NcContractFingerprint = NcContractFingerprint::new([10; 32]);
    /// Fingerprint of cycle fixture B.
    const CYCLE_B_FINGERPRINT: NcContractFingerprint = NcContractFingerprint::new([11; 32]);
    /// Edge from cycle fixture A to B.
    static CYCLE_A_DEPENDENCIES: [NcContractDependencyDescription; 1] = [NcContractDependencyDescription {
        module_id: CYCLE_B_ID,
        abi_fingerprint: CYCLE_B_FINGERPRINT,
        get: Some(get_cycle_b),
    }];
    /// Edge from cycle fixture B back to A.
    static CYCLE_B_DEPENDENCIES: [NcContractDependencyDescription; 1] = [NcContractDependencyDescription {
        module_id: CYCLE_A_ID,
        abi_fingerprint: CYCLE_A_FINGERPRINT,
        get: Some(get_cycle_a),
    }];
    /// First canonical root participating in a malformed dependency cycle.
    static CYCLE_A: NcContractModuleDescription = NcContractModuleDescription {
        description_abi_version: NC_CONTRACT_DESCRIPTION_ABI_VERSION,
        module_id: CYCLE_A_ID,
        abi_fingerprint: CYCLE_A_FINGERPRINT,
        module_name: NcStaticUtf8::from_static("nc.tests.cycle_a"),
        dependency_count: 1,
        dependencies: CYCLE_A_DEPENDENCIES.as_ptr(),
        declaration_count: 0,
        declarations: ptr::null(),
    };
    /// Second canonical root participating in a malformed dependency cycle.
    static CYCLE_B: NcContractModuleDescription = NcContractModuleDescription {
        description_abi_version: NC_CONTRACT_DESCRIPTION_ABI_VERSION,
        module_id: CYCLE_B_ID,
        abi_fingerprint: CYCLE_B_FINGERPRINT,
        module_name: NcStaticUtf8::from_static("nc.tests.cycle_b"),
        dependency_count: 1,
        dependencies: CYCLE_B_DEPENDENCIES.as_ptr(),
        declaration_count: 0,
        declarations: ptr::null(),
    };

    #[test]
    fn rejects_a_native_dependency_cycle_without_partial_publication() {
        let runtime = RuntimeState::new();
        let root = NcContractModuleReference {
            module_id: CYCLE_A_ID,
            abi_fingerprint: CYCLE_A_FINGERPRINT,
            get: Some(get_cycle_a),
        };
        // SAFETY: Both fixtures are immutable and live for the complete test.
        assert_eq!(unsafe { runtime.register_contract_module(&root) }, NC_E_INVALID_ARG);
        runtime.read_catalog().contract_registry.with_module_ids(|ids| assert!(ids.is_empty()));
    }
}
