//! Atomic registration of validated plugin and class metadata.
//!
//! Descriptor graphs are validated and projected into owned/index-ready batches
//! before the global catalog write lock is taken. The locked phase checks current
//! dependencies, identity mappings, and collisions, then commits every record
//! together so readers never observe a partial registration.

use std::collections::HashMap;
use std::string::String;
use std::vec::Vec;

use neocogi_com_core::{NC_E_INVALID_ARG, NC_E_NOT_FOUND, NC_OK, NcGuid, NcInterfaceId, NcPluginDescriptor, NcStatus, validate_plugin_descriptor_with_issues};

use crate::loader::LoadedModuleId;
use crate::plugin_registry::RegisteredPlugin;
use crate::state::{RegistrationState, RuntimeCatalog, RuntimeState};
use crate::type_registry::{RegisteredClass, RegisteredClassView};

/// One validated plugin and all records that must be published with it.
///
/// The batch is private to the runtime so validated metadata cannot be forged
/// by a caller. It owns every derived collection needed by registration; raw
/// pointers continue to borrow immutable descriptors from the caller or loaded
/// module under the public registration safety contract.
pub(crate) struct PluginRegistrationBatch {
    /// Plugin root and copied dependency edges.
    plugin: RegisteredPlugin,
    /// Classes and copied interface identities ready for type-registry insertion.
    classes: Vec<RegisteredClass>,
    /// One-to-one GUID/name mappings reachable from this descriptor graph.
    identities: RegistrationIdentities,
}

impl PluginRegistrationBatch {
    /// Returns the stable plugin ID represented by this batch.
    pub(crate) fn plugin_id(&self) -> NcGuid {
        self.plugin.plugin_id
    }

    /// Associates every projected class with one retained dynamic module.
    pub(crate) fn set_module_id(&mut self, module_id: LoadedModuleId) {
        for class in &mut self.classes {
            class.module_id = Some(module_id);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Phase in which registration batch construction failed.
pub(crate) enum PluginRegistrationBatchErrorKind {
    /// Raw descriptor structure or metadata validation failed.
    Metadata,
    /// Owned registration projection violated runtime identity rules.
    Registration,
}

/// Internal batch error preserving phase and exact ABI status.
pub(crate) struct PluginRegistrationBatchError {
    /// Broad phase used by loader error mapping.
    kind: PluginRegistrationBatchErrorKind,
    /// Exact status returned to direct registration callers.
    status: NcStatus,
}

impl PluginRegistrationBatchError {
    /// Creates a descriptor metadata failure.
    fn metadata(status: NcStatus) -> Self {
        Self {
            kind: PluginRegistrationBatchErrorKind::Metadata,
            status,
        }
    }

    /// Creates an owned projection/identity failure.
    fn registration(status: NcStatus) -> Self {
        Self {
            kind: PluginRegistrationBatchErrorKind::Registration,
            status,
        }
    }

    /// Returns the phase that rejected batch construction.
    pub(crate) fn kind(&self) -> PluginRegistrationBatchErrorKind {
        self.kind
    }

    /// Returns the exact ABI failure status.
    pub(crate) fn status(&self) -> NcStatus {
        self.status
    }
}

/// One stable GUID-to-qualified-name mapping.
struct RegistrationIdentity {
    /// Stable plugin, class, or interface identity.
    id: NcGuid,
    /// Owned normative qualified name.
    name: String,
}

/// Final owned identity mappings carried into the transaction phase.
struct RegistrationIdentities {
    /// Ordered mappings used for ID-keyed validation and insertion.
    names_by_id: Vec<RegistrationIdentity>,
    /// Reverse mapping used for name-keyed validation and insertion.
    ids_by_name: HashMap<String, NcGuid>,
}

/// Deduplicating builder for a descriptor graph's identity mappings.
struct RegistrationIdentityBatch {
    /// First-seen mappings in descriptor traversal order.
    names_by_id: Vec<RegistrationIdentity>,
    /// GUID-to-vector index used to detect conflicting repeated IDs.
    identity_index_by_id: HashMap<NcGuid, usize>,
    /// Name-to-GUID map used to detect conflicting repeated names.
    ids_by_name: HashMap<String, NcGuid>,
}

impl RegistrationIdentityBatch {
    /// Preallocates all identity containers for the expected graph size.
    fn with_capacity(capacity: usize) -> Self {
        Self {
            names_by_id: Vec::with_capacity(capacity),
            identity_index_by_id: HashMap::with_capacity(capacity),
            ids_by_name: HashMap::with_capacity(capacity),
        }
    }

    /// Inserts an idempotent mapping or rejects either direction of conflict.
    fn push(&mut self, id: NcGuid, name: &str) -> Result<(), NcStatus> {
        if self.identity_index_by_id.get(&id).is_some_and(|index| self.names_by_id[*index].name != name)
            || self.ids_by_name.get(name).is_some_and(|existing| *existing != id)
        {
            return Err(NC_E_INVALID_ARG);
        }

        if self.identity_index_by_id.contains_key(&id) {
            return Ok(());
        }

        let index = self.names_by_id.len();
        self.identity_index_by_id.insert(id, index);
        self.ids_by_name.insert(name.to_owned(), id);
        self.names_by_id.push(RegistrationIdentity { id, name: name.to_owned() });
        Ok(())
    }

    /// Discards the construction-only GUID index and returns transaction data.
    fn finish(self) -> RegistrationIdentities {
        RegistrationIdentities {
            names_by_id: self.names_by_id,
            ids_by_name: self.ids_by_name,
        }
    }
}

/// Validates a descriptor and assembles its complete registration batch.
///
/// # Safety
///
/// `plugin` must be null or point to readable plugin descriptor metadata whose
/// transitive pointers remain readable for this call and remain valid while
/// registered after a successful commit.
pub(crate) unsafe fn build_plugin_registration_batch(
    plugin: *const NcPluginDescriptor,
    module_id: Option<LoadedModuleId>,
) -> Result<PluginRegistrationBatch, PluginRegistrationBatchError> {
    // SAFETY: The caller provides the raw descriptor readability contract.
    let status = unsafe { validate_descriptor_for_registration(plugin) };
    if status.is_err() {
        return Err(PluginRegistrationBatchError::metadata(status));
    }

    // SAFETY: Successful validation rejected null and established readability.
    let descriptor = unsafe { &*plugin };
    // SAFETY: Successful validation established valid static UTF-8 metadata.
    let qualified_name = unsafe { descriptor.qualified_name.as_str() }.map_err(PluginRegistrationBatchError::metadata)?;
    // SAFETY: Validation established the dependency pointer/count invariant.
    let dependency_ids = unsafe { descriptor.dependencies() }
        .map_err(PluginRegistrationBatchError::metadata)?
        .iter()
        .map(|dependency| dependency.plugin_id)
        .collect();

    // SAFETY: Validation established the class-table pointer and class slice.
    let class_descriptors = unsafe { (&*descriptor.class_table).classes() }.map_err(PluginRegistrationBatchError::metadata)?;
    let mut identities = RegistrationIdentityBatch::with_capacity(class_descriptors.len() + 1);
    identities
        .push(descriptor.plugin_id, qualified_name)
        .map_err(PluginRegistrationBatchError::registration)?;
    let mut classes = Vec::with_capacity(class_descriptors.len());

    // This single projection pass prepares class records, interface indexes,
    // and identity mappings together. No descriptor graph traversal remains
    // for the transaction-locked phase.
    for class in class_descriptors {
        // SAFETY: Validation established valid class text and interface tables.
        let class_name = unsafe { class.qualified_name.as_str() }.map_err(PluginRegistrationBatchError::metadata)?;
        identities
            .push(class.class_id, class_name)
            .map_err(PluginRegistrationBatchError::registration)?;

        // SAFETY: The class descriptor passed full graph validation above.
        let class_interfaces = unsafe { class.interfaces() }.map_err(PluginRegistrationBatchError::metadata)?;
        let mut interface_ids = Vec::with_capacity(class_interfaces.len());
        for class_interface in class_interfaces {
            // SAFETY: Full validation rejected null interface descriptor pointers.
            let interface = unsafe { class_interface.interface_descriptor() }.map_err(PluginRegistrationBatchError::metadata)?;
            // SAFETY: Full validation established valid interface text.
            let interface_name = unsafe { interface.qualified_name.as_str() }.map_err(PluginRegistrationBatchError::metadata)?;
            identities
                .push(interface.interface_id, interface_name)
                .map_err(PluginRegistrationBatchError::registration)?;
            interface_ids.push(NcInterfaceId::new(interface.interface_id, interface.abi_signature));
        }

        classes.push(RegisteredClass {
            plugin_id: descriptor.plugin_id,
            module_id,
            class_id: class.class_id,
            class: core::ptr::from_ref(class),
            interface_ids,
        });
    }

    Ok(PluginRegistrationBatch {
        plugin: RegisteredPlugin {
            plugin_id: descriptor.plugin_id,
            plugin,
            dependency_ids,
        },
        classes,
        identities: identities.finish(),
    })
}

/// Runs whole-graph validation and increments the test-only validation counter.
unsafe fn validate_descriptor_for_registration(plugin: *const NcPluginDescriptor) -> NcStatus {
    #[cfg(test)]
    REGISTRATION_VALIDATION_CALLS.with(|calls| calls.set(calls.get() + 1));

    // SAFETY: The caller provides the descriptor readability contract.
    unsafe { validate_plugin_descriptor_with_issues(plugin, |_| {}) }
}

#[cfg(test)]
std::thread_local! {
    static REGISTRATION_VALIDATION_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
/// Resets the thread-local validation-call counter used by projection tests.
pub(crate) fn reset_registration_validation_calls() {
    REGISTRATION_VALIDATION_CALLS.with(|calls| calls.set(0));
}

#[cfg(test)]
/// Returns the thread-local number of descriptor validation passes.
pub(crate) fn registration_validation_calls() -> usize {
    REGISTRATION_VALIDATION_CALLS.with(std::cell::Cell::get)
}

impl RuntimeState {
    /// Registers a plugin descriptor after validating its exported contracts.
    ///
    /// Registration copies only small identity relationships into runtime-owned
    /// state. The descriptor itself must remain valid while the plugin shared
    /// library is loaded.
    ///
    /// Re-registering the same root descriptor address is idempotent. A
    /// different descriptor with an existing plugin ID, or a different class
    /// descriptor with an existing class ID, is invalid.
    ///
    /// Concurrent registration serializes duplicate-class checks and class
    /// insertion as one blocking catalog transaction. Read-only lookups share
    /// the catalog read lock and may run concurrently. Declared hard
    /// dependencies must already be registered.
    /// Registration checks their IDs but never locates or loads libraries.
    /// It also enforces one GUID-to-qualified-name mapping across all registered
    /// plugins, classes, and interfaces. Validation and mapping checks are
    /// registration-only work and do not affect lookup or dispatch hot paths.
    ///
    /// # Safety
    ///
    /// `plugin` must be null or point to readable plugin descriptor metadata
    /// whose class table and interface descriptors remain valid while
    /// registered.
    pub unsafe fn register_plugin_descriptor(&self, plugin: *const NcPluginDescriptor) -> NcStatus {
        // SAFETY: This public unsafe function forwards the caller's descriptor
        // validity contract to the private batch builder.
        let batch = match unsafe { build_plugin_registration_batch(plugin, None) } {
            Ok(batch) => batch,
            Err(error) => return error.status(),
        };
        self.commit_plugin_registration_batch(batch)
    }

    /// Publishes a prevalidated static registration under one catalog write lock.
    pub(crate) fn commit_plugin_registration_batch(&self, batch: PluginRegistrationBatch) -> NcStatus {
        self.write_catalog().commit_plugin_registration_batch(batch)
    }
}

impl RuntimeCatalog {
    /// Checks current state and publishes one prevalidated batch.
    pub(crate) fn commit_plugin_registration_batch(&mut self, mut batch: PluginRegistrationBatch) -> NcStatus {
        // All raw traversal, signature validation, string copying, and record assembly has
        // completed. The enclosing write guard protects state-dependent checks
        // and publication of every catalog projection.

        if let Err(status) = self.validate_registered_dependencies(&batch.plugin.dependency_ids) {
            return status;
        }
        if let Err(status) = validate_registered_identity_mappings(&batch.identities, &self.registration) {
            return status;
        }

        let plugin_registered = match self.retain_new_registration_records(&mut batch) {
            Ok(plugin_registered) => plugin_registered,
            Err(status) => return status,
        };
        if plugin_registered && batch.classes.is_empty() {
            return NC_OK;
        }

        let PluginRegistrationBatch { plugin, classes, identities } = batch;
        self.insert_plugin_and_classes((!plugin_registered).then_some(plugin), classes);
        register_identity_mappings(&mut self.registration, identities);
        NC_OK
    }

    /// Requires every declared hard dependency to be registered already.
    fn validate_registered_dependencies(&self, dependency_ids: &[NcGuid]) -> Result<(), NcStatus> {
        if dependency_ids.iter().any(|dependency_id| !self.plugin_registry.contains_plugin(*dependency_id)) {
            return Err(NC_E_NOT_FOUND);
        }
        Ok(())
    }

    /// Removes exact idempotent records and rejects conflicting existing records.
    fn retain_new_registration_records(&self, batch: &mut PluginRegistrationBatch) -> Result<bool, NcStatus> {
        let existing_plugin = self.plugin_registry.plugin(batch.plugin.plugin_id);
        if existing_plugin.is_some_and(|registered| registered.plugin != batch.plugin.plugin) {
            return Err(NC_E_INVALID_ARG);
        }
        let plugin_registered = existing_plugin.is_some();

        let mut conflict = false;
        batch.classes.retain(|class| {
            if let Some(existing) = self.type_registry.class(class.class_id) {
                if !registered_class_view_matches(&existing, class) {
                    conflict = true;
                }
                false
            } else {
                true
            }
        });
        if conflict {
            return Err(NC_E_INVALID_ARG);
        }
        Ok(plugin_registered)
    }

    /// Publishes plugin and class records inside the enclosing catalog transaction.
    fn insert_plugin_and_classes(&mut self, plugin: Option<RegisteredPlugin>, classes: Vec<RegisteredClass>) {
        if let Some(plugin) = plugin {
            self.plugin_registry.insert(plugin);
        }
        self.type_registry.insert_classes(classes);
    }
}

/// Checks a batch's GUID/name mappings against all previously committed mappings.
fn validate_registered_identity_mappings(identities: &RegistrationIdentities, registration: &RegistrationState) -> Result<(), NcStatus> {
    for identity in &identities.names_by_id {
        if registration.name_by_id.get(&identity.id).is_some_and(|name| name != &identity.name)
            || registration.id_by_name.get(&identity.name).is_some_and(|id| *id != identity.id)
        {
            return Err(NC_E_INVALID_ARG);
        }
    }
    Ok(())
}

/// Tests whether an existing class is the exact same plugin/descriptor registration.
fn registered_class_view_matches(existing: &RegisteredClassView, class: &RegisteredClass) -> bool {
    existing.plugin_id == class.plugin_id && existing.class == class.class
}

/// Inserts newly observed bidirectional identity mappings after a successful commit.
fn register_identity_mappings(registration: &mut RegistrationState, identities: RegistrationIdentities) {
    for identity in identities.names_by_id {
        registration.name_by_id.entry(identity.id).or_insert(identity.name);
    }
    for (name, id) in identities.ids_by_name {
        registration.id_by_name.entry(name).or_insert(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neocogi_com_core::{
        IUnknown, NC_COM_CORE_ABI_VERSION, NC_E_ABI_VERSION, NcAbiSignature, NcAbiSignaturePart, NcClassDescriptor, NcClassInterfaceDescriptor, NcClassTable,
        NcInterfaceDescriptor, NcInterfaceId, NcPluginDependencyDescriptor, NcStaticUtf8,
    };
    use std::rc::Rc;
    use std::sync::{
        Arc, Barrier,
        atomic::{AtomicUsize, Ordering},
    };

    static TEST_CONSTRUCTOR_CALLS: AtomicUsize = AtomicUsize::new(0);
    static MULTI_PLUGIN_CONSTRUCTOR_CALLS: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn test_constructor(_class: *const NcClassDescriptor, _arguments: *const core::ffi::c_void, _out: *mut *mut IUnknown) -> NcStatus {
        TEST_CONSTRUCTOR_CALLS.fetch_add(1, Ordering::SeqCst);
        NC_E_INVALID_ARG
    }

    unsafe extern "C" fn multi_plugin_test_constructor(
        _class: *const NcClassDescriptor,
        _arguments: *const core::ffi::c_void,
        _out: *mut *mut IUnknown,
    ) -> NcStatus {
        MULTI_PLUGIN_CONSTRUCTOR_CALLS.fetch_add(1, Ordering::SeqCst);
        NC_E_INVALID_ARG
    }

    #[test]
    fn module_id_indexes_registered_class_metadata() {
        const MODULE_ID: LoadedModuleId = LoadedModuleId::from_raw(77);
        const PLUGIN_ID: NcGuid = neocogi_com_core::guid_from_qualified_name("nc.tests.registry.module_indexed_plugin");
        const CLASS_ID: NcGuid = neocogi_com_core::guid_from_qualified_name("nc.tests.registry.ModuleIndexedClass");
        const INTERFACE_ID: NcGuid = neocogi_com_core::guid_from_qualified_name("nc.tests.registry.ModuleIndexedInterface");
        const INTERFACE_SIGNATURE: NcAbiSignature = neocogi_com_core::abi_signature!(&[NcAbiSignaturePart::Interface { iid: INTERFACE_ID }]);
        static INTERFACE: NcInterfaceDescriptor = NcInterfaceDescriptor {
            interface_id: INTERFACE_ID,
            qualified_name: NcStaticUtf8::from_static("nc.tests.registry.ModuleIndexedInterface"),
            abi_signature: INTERFACE_SIGNATURE,
            base: neocogi_com_core::NcInterfaceId::NONE,
            method_count: 0,
            methods: core::ptr::null(),
        };
        static CLASS_INTERFACES: [NcClassInterfaceDescriptor; 2] = [
            NcClassInterfaceDescriptor {
                interface_descriptor: &IUnknown::INTERFACE_DESCRIPTOR,
            },
            NcClassInterfaceDescriptor { interface_descriptor: &INTERFACE },
        ];
        static CLASSES: [NcClassDescriptor; 1] = [NcClassDescriptor {
            class_id: CLASS_ID,
            qualified_name: NcStaticUtf8::from_static("nc.tests.registry.ModuleIndexedClass"),
            class_version: NcStaticUtf8::from_static("1.0.0"),
            constructor_abi_signature: NcAbiSignature::EMPTY,
            threading: neocogi_com_core::NC_ABI_THREADING_THREAD_LOCAL,
            construct: Some(test_constructor),
            interface_count: CLASS_INTERFACES.len() as u32,
            interfaces: CLASS_INTERFACES.as_ptr(),
        }];
        static CLASS_TABLE: NcClassTable = NcClassTable {
            class_count: CLASSES.len() as u32,
            classes: CLASSES.as_ptr(),
        };
        static PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
            core_abi_version: NC_COM_CORE_ABI_VERSION,
            validation_mode: neocogi_com_core::NC_ABI_VALIDATION_MODE,
            plugin_id: PLUGIN_ID,
            qualified_name: NcStaticUtf8::from_static("nc.tests.registry.module_indexed_plugin"),
            plugin_version: NcStaticUtf8::from_static("1.0.0"),
            dependency_count: 0,
            dependencies: core::ptr::null(),
            class_table: &CLASS_TABLE,
        };

        let runtime = RuntimeState::new();
        TEST_CONSTRUCTOR_CALLS.store(0, Ordering::SeqCst);
        let batch = unsafe { build_plugin_registration_batch(&PLUGIN, Some(MODULE_ID)) }.unwrap_or_else(|error| panic!("batch failed with {}", error.status()));
        let status = runtime.commit_plugin_registration_batch(batch);
        assert_eq!(status, NC_OK);
        assert_eq!(runtime.read_catalog().plugin_registry.registered_plugin_ids(), vec![PLUGIN_ID]);
        assert_eq!(runtime.read_catalog().type_registry.registered_class_ids(), vec![CLASS_ID]);
        assert_eq!(
            runtime.read_catalog().type_registry.registered_interface_ids().unwrap(),
            vec![IUnknown::INTERFACE_ID, NcInterfaceId::new(INTERFACE_ID, INTERFACE_SIGNATURE),]
        );
        assert_eq!(
            crate::inspection::plugin_class_ids(&runtime.read_catalog().type_registry, PLUGIN_ID,),
            vec![CLASS_ID]
        );
        assert_eq!(
            crate::inspection::class_interface_ids(&runtime.read_catalog().type_registry, CLASS_ID,).unwrap(),
            vec![IUnknown::INTERFACE_ID, NcInterfaceId::new(INTERFACE_ID, INTERFACE_SIGNATURE),]
        );
        assert_eq!(
            crate::inspection::interface_implementor_class_ids(&runtime.read_catalog().type_registry, IUnknown::INTERFACE_ID,).unwrap(),
            vec![CLASS_ID]
        );
        assert_eq!(
            crate::inspection::interface_implementor_class_ids(&runtime.read_catalog().type_registry, NcInterfaceId::new(INTERFACE_ID, INTERFACE_SIGNATURE),)
                .unwrap(),
            vec![CLASS_ID]
        );
        let plugin_info = crate::inspection::plugin_info(&runtime.read_catalog().plugin_registry, PLUGIN_ID).unwrap();
        assert_eq!(plugin_info.plugin_id, PLUGIN_ID);
        let class_info = crate::inspection::class_info(&runtime.read_catalog().type_registry, CLASS_ID).unwrap();
        assert_eq!(class_info.plugin_id, PLUGIN_ID);
        assert_eq!(class_info.module_id, MODULE_ID.get());
        assert_eq!(class_info.constructor_abi_signature, NcAbiSignature::EMPTY);
        assert_eq!(class_info.threading, neocogi_com_core::NC_ABI_THREADING_THREAD_LOCAL);

        let mut out = core::ptr::null_mut();
        let class = crate::construction::resolve_class(&runtime.read_catalog().type_registry, CLASS_ID, neocogi_com_core::NC_ABI_THREADING_THREAD_SAFE);
        let status = unsafe { crate::construction::construct_class(class, NcAbiSignature::EMPTY, core::ptr::null(), &mut out) };
        assert_eq!(status, NC_E_INVALID_ARG);
        assert!(out.is_null());
        assert_eq!(TEST_CONSTRUCTOR_CALLS.load(Ordering::SeqCst), 0);

        let class = crate::construction::resolve_class(&runtime.read_catalog().type_registry, CLASS_ID, u32::MAX);
        let status = unsafe { crate::construction::construct_class(class, NcAbiSignature::EMPTY, core::ptr::null(), &mut out) };
        assert_eq!(status, NC_E_INVALID_ARG);
        assert!(out.is_null());
        assert_eq!(TEST_CONSTRUCTOR_CALLS.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn runtime_rejects_a_second_descriptor_with_the_same_plugin_id() {
        const PLUGIN_ID: NcGuid = neocogi_com_core::guid_from_qualified_name("nc.tests.registry.same_id");
        static EMPTY_CLASSES: NcClassTable = NcClassTable {
            class_count: 0,
            classes: core::ptr::null(),
        };
        static PLUGIN_A: NcPluginDescriptor = NcPluginDescriptor {
            core_abi_version: NC_COM_CORE_ABI_VERSION,
            validation_mode: neocogi_com_core::NC_ABI_VALIDATION_MODE,
            plugin_id: PLUGIN_ID,
            qualified_name: NcStaticUtf8::from_static("nc.tests.registry.same_id"),
            plugin_version: NcStaticUtf8::from_static("1.0.0"),
            dependency_count: 0,
            dependencies: core::ptr::null(),
            class_table: &EMPTY_CLASSES,
        };
        static PLUGIN_B: NcPluginDescriptor = NcPluginDescriptor {
            core_abi_version: NC_COM_CORE_ABI_VERSION,
            validation_mode: neocogi_com_core::NC_ABI_VALIDATION_MODE,
            plugin_id: PLUGIN_ID,
            qualified_name: NcStaticUtf8::from_static("nc.tests.registry.same_id"),
            plugin_version: NcStaticUtf8::from_static("1.0.0"),
            dependency_count: 0,
            dependencies: core::ptr::null(),
            class_table: &EMPTY_CLASSES,
        };

        let runtime = RuntimeState::new();
        assert_eq!(unsafe { runtime.register_plugin_descriptor(&PLUGIN_A) }, NC_OK);
        assert_eq!(unsafe { runtime.register_plugin_descriptor(&PLUGIN_B) }, NC_E_INVALID_ARG);
        assert_eq!(runtime.read_catalog().plugin_registry.registered_plugin_ids(), vec![PLUGIN_ID]);
    }

    #[test]
    fn exact_reregistration_is_idempotent() {
        const PLUGIN_ID: NcGuid = neocogi_com_core::guid_from_qualified_name("nc.tests.registry.idempotent_plugin");
        static EMPTY_CLASSES: NcClassTable = NcClassTable {
            class_count: 0,
            classes: core::ptr::null(),
        };
        static PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
            core_abi_version: NC_COM_CORE_ABI_VERSION,
            validation_mode: neocogi_com_core::NC_ABI_VALIDATION_MODE,
            plugin_id: PLUGIN_ID,
            qualified_name: NcStaticUtf8::from_static("nc.tests.registry.idempotent_plugin"),
            plugin_version: NcStaticUtf8::from_static("1.0.0"),
            dependency_count: 0,
            dependencies: core::ptr::null(),
            class_table: &EMPTY_CLASSES,
        };

        let runtime = RuntimeState::new();
        reset_registration_validation_calls();
        assert_eq!(unsafe { runtime.register_plugin_descriptor(&PLUGIN) }, NC_OK);
        assert_eq!(registration_validation_calls(), 1);
        reset_registration_validation_calls();
        assert_eq!(unsafe { runtime.register_plugin_descriptor(&PLUGIN) }, NC_OK);
        assert_eq!(registration_validation_calls(), 1);
        assert_eq!(runtime.read_catalog().plugin_registry.registered_plugin_ids(), vec![PLUGIN_ID]);
    }

    #[test]
    fn dependency_must_be_registered_before_dependent() {
        const BASE_ID: NcGuid = neocogi_com_core::guid_from_qualified_name("nc.tests.registry.base");
        const DEPENDENT_ID: NcGuid = neocogi_com_core::guid_from_qualified_name("nc.tests.registry.dependent");
        static EMPTY_CLASSES: NcClassTable = NcClassTable {
            class_count: 0,
            classes: core::ptr::null(),
        };
        static DEPENDENCIES: [NcPluginDependencyDescriptor; 1] = [NcPluginDependencyDescriptor { plugin_id: BASE_ID }];
        static BASE: NcPluginDescriptor = NcPluginDescriptor {
            core_abi_version: NC_COM_CORE_ABI_VERSION,
            validation_mode: neocogi_com_core::NC_ABI_VALIDATION_MODE,
            plugin_id: BASE_ID,
            qualified_name: NcStaticUtf8::from_static("nc.tests.registry.base"),
            plugin_version: NcStaticUtf8::from_static("1.0.0"),
            dependency_count: 0,
            dependencies: core::ptr::null(),
            class_table: &EMPTY_CLASSES,
        };
        static DEPENDENT: NcPluginDescriptor = NcPluginDescriptor {
            core_abi_version: NC_COM_CORE_ABI_VERSION,
            validation_mode: neocogi_com_core::NC_ABI_VALIDATION_MODE,
            plugin_id: DEPENDENT_ID,
            qualified_name: NcStaticUtf8::from_static("nc.tests.registry.dependent"),
            plugin_version: NcStaticUtf8::from_static("1.0.0"),
            dependency_count: DEPENDENCIES.len() as u32,
            dependencies: DEPENDENCIES.as_ptr(),
            class_table: &EMPTY_CLASSES,
        };

        let runtime = RuntimeState::new();
        assert_eq!(unsafe { runtime.register_plugin_descriptor(&DEPENDENT) }, NC_E_NOT_FOUND);
        assert!(runtime.read_catalog().plugin_registry.registered_plugin_ids().is_empty());
        assert_eq!(unsafe { runtime.register_plugin_descriptor(&BASE) }, NC_OK);
        assert_eq!(unsafe { runtime.register_plugin_descriptor(&DEPENDENT) }, NC_OK);
        assert_eq!(runtime.read_catalog().plugin_registry.registered_plugin_ids(), vec![BASE_ID, DEPENDENT_ID]);
    }

    #[test]
    fn five_plugin_dependency_chain_plans_and_registers_in_process() {
        use crate::{PluginInspection, plan_plugin_load_order};

        const COUNTER_PLUGIN_ID: NcGuid = neocogi_com_core::guid_from_qualified_name("nc.examples.multi.counter");
        const FLAG_PLUGIN_ID: NcGuid = neocogi_com_core::guid_from_qualified_name("nc.examples.multi.flag");
        const METRIC_PLUGIN_ID: NcGuid = neocogi_com_core::guid_from_qualified_name("nc.examples.multi.metric");
        const STATE_PLUGIN_ID: NcGuid = neocogi_com_core::guid_from_qualified_name("nc.examples.multi.state");
        const EXTRA_PLUGIN_ID: NcGuid = neocogi_com_core::guid_from_qualified_name("nc.examples.multi.extra");
        const COUNTER_CLASS_ID: NcGuid = neocogi_com_core::guid_from_qualified_name("nc.examples.multi.Counter");
        const FLAG_CLASS_ID: NcGuid = neocogi_com_core::guid_from_qualified_name("nc.examples.multi.Flag");
        const METRIC_CLASS_ID: NcGuid = neocogi_com_core::guid_from_qualified_name("nc.examples.multi.Metric");
        const STATE_CLASS_ID: NcGuid = neocogi_com_core::guid_from_qualified_name("nc.examples.multi.State");
        const EXTRA_CLASS_ID: NcGuid = neocogi_com_core::guid_from_qualified_name("nc.examples.multi.Extra");
        const INTERFACE_ID: NcGuid = neocogi_com_core::guid_from_qualified_name("nc.tests.registry.MultiPluginInterface");
        const INTERFACE_SIGNATURE: NcAbiSignature = neocogi_com_core::abi_signature!(&[NcAbiSignaturePart::Interface { iid: INTERFACE_ID }]);

        static INTERFACE: NcInterfaceDescriptor = NcInterfaceDescriptor {
            interface_id: INTERFACE_ID,
            qualified_name: NcStaticUtf8::from_static("nc.tests.registry.MultiPluginInterface"),
            abi_signature: INTERFACE_SIGNATURE,
            base: neocogi_com_core::NcInterfaceId::NONE,
            method_count: 0,
            methods: core::ptr::null(),
        };
        static CLASS_INTERFACES: [NcClassInterfaceDescriptor; 2] = [
            NcClassInterfaceDescriptor {
                interface_descriptor: &IUnknown::INTERFACE_DESCRIPTOR,
            },
            NcClassInterfaceDescriptor { interface_descriptor: &INTERFACE },
        ];

        macro_rules! class_table {
            ($classes:ident, $table:ident, $id:ident, $name:literal) => {
                static $classes: [NcClassDescriptor; 1] = [NcClassDescriptor {
                    class_id: $id,
                    qualified_name: NcStaticUtf8::from_static($name),
                    class_version: NcStaticUtf8::from_static("1.0.0"),
                    constructor_abi_signature: NcAbiSignature::EMPTY,
                    threading: neocogi_com_core::NC_ABI_THREADING_THREAD_LOCAL,
                    construct: Some(multi_plugin_test_constructor),
                    interface_count: CLASS_INTERFACES.len() as u32,
                    interfaces: CLASS_INTERFACES.as_ptr(),
                }];
                static $table: NcClassTable = NcClassTable {
                    class_count: $classes.len() as u32,
                    classes: $classes.as_ptr(),
                };
            };
        }

        class_table!(COUNTER_CLASSES, COUNTER_CLASS_TABLE, COUNTER_CLASS_ID, "nc.examples.multi.Counter");
        class_table!(FLAG_CLASSES, FLAG_CLASS_TABLE, FLAG_CLASS_ID, "nc.examples.multi.Flag");
        class_table!(METRIC_CLASSES, METRIC_CLASS_TABLE, METRIC_CLASS_ID, "nc.examples.multi.Metric");
        class_table!(STATE_CLASSES, STATE_CLASS_TABLE, STATE_CLASS_ID, "nc.examples.multi.State");
        class_table!(EXTRA_CLASSES, EXTRA_CLASS_TABLE, EXTRA_CLASS_ID, "nc.examples.multi.Extra");

        static EMPTY_DEPENDENCIES: [NcPluginDependencyDescriptor; 0] = [];
        static FLAG_DEPENDENCIES: [NcPluginDependencyDescriptor; 1] = [NcPluginDependencyDescriptor { plugin_id: COUNTER_PLUGIN_ID }];
        static METRIC_DEPENDENCIES: [NcPluginDependencyDescriptor; 1] = [NcPluginDependencyDescriptor { plugin_id: FLAG_PLUGIN_ID }];
        static STATE_DEPENDENCIES: [NcPluginDependencyDescriptor; 1] = [NcPluginDependencyDescriptor { plugin_id: METRIC_PLUGIN_ID }];
        static EXTRA_DEPENDENCIES: [NcPluginDependencyDescriptor; 1] = [NcPluginDependencyDescriptor { plugin_id: STATE_PLUGIN_ID }];

        macro_rules! plugin {
            ($plugin:ident, $id:ident, $name:literal, $dependencies:expr, $table:ident) => {
                static $plugin: NcPluginDescriptor = NcPluginDescriptor {
                    core_abi_version: NC_COM_CORE_ABI_VERSION,
                    validation_mode: neocogi_com_core::NC_ABI_VALIDATION_MODE,
                    plugin_id: $id,
                    qualified_name: NcStaticUtf8::from_static($name),
                    plugin_version: NcStaticUtf8::from_static("1.0.0"),
                    dependency_count: $dependencies.len() as u32,
                    dependencies: $dependencies.as_ptr(),
                    class_table: &$table,
                };
            };
        }

        plugin!(
            COUNTER_PLUGIN,
            COUNTER_PLUGIN_ID,
            "nc.examples.multi.counter",
            EMPTY_DEPENDENCIES,
            COUNTER_CLASS_TABLE
        );
        plugin!(FLAG_PLUGIN, FLAG_PLUGIN_ID, "nc.examples.multi.flag", FLAG_DEPENDENCIES, FLAG_CLASS_TABLE);
        plugin!(
            METRIC_PLUGIN,
            METRIC_PLUGIN_ID,
            "nc.examples.multi.metric",
            METRIC_DEPENDENCIES,
            METRIC_CLASS_TABLE
        );
        plugin!(STATE_PLUGIN, STATE_PLUGIN_ID, "nc.examples.multi.state", STATE_DEPENDENCIES, STATE_CLASS_TABLE);
        plugin!(EXTRA_PLUGIN, EXTRA_PLUGIN_ID, "nc.examples.multi.extra", EXTRA_DEPENDENCIES, EXTRA_CLASS_TABLE);

        fn inspected(plugin_id: NcGuid, qualified_name: &str, dependencies: &[NcGuid]) -> PluginInspection {
            PluginInspection {
                core_abi_version: NC_COM_CORE_ABI_VERSION,
                plugin_id,
                qualified_name: qualified_name.to_owned(),
                plugin_version: "1.0.0".to_owned(),
                dependencies: dependencies.to_vec(),
                classes: Vec::new(),
            }
        }

        let inspected_plugins = [
            inspected(EXTRA_PLUGIN_ID, "nc.examples.multi.extra", &[STATE_PLUGIN_ID]),
            inspected(STATE_PLUGIN_ID, "nc.examples.multi.state", &[METRIC_PLUGIN_ID]),
            inspected(METRIC_PLUGIN_ID, "nc.examples.multi.metric", &[FLAG_PLUGIN_ID]),
            inspected(FLAG_PLUGIN_ID, "nc.examples.multi.flag", &[COUNTER_PLUGIN_ID]),
            inspected(COUNTER_PLUGIN_ID, "nc.examples.multi.counter", &[]),
        ];
        let expected_order = [COUNTER_PLUGIN_ID, FLAG_PLUGIN_ID, METRIC_PLUGIN_ID, STATE_PLUGIN_ID, EXTRA_PLUGIN_ID];
        assert_eq!(plan_plugin_load_order(&inspected_plugins).unwrap(), expected_order);

        let descriptors = [&COUNTER_PLUGIN, &FLAG_PLUGIN, &METRIC_PLUGIN, &STATE_PLUGIN, &EXTRA_PLUGIN];
        let runtime = RuntimeState::new();
        assert_eq!(unsafe { runtime.register_plugin_descriptor(&EXTRA_PLUGIN) }, NC_E_NOT_FOUND);
        assert!(runtime.read_catalog().plugin_registry.registered_plugin_ids().is_empty());

        for plugin_id in expected_order {
            let descriptor = descriptors.iter().copied().find(|descriptor| descriptor.plugin_id == plugin_id).unwrap();
            assert_eq!(unsafe { runtime.register_plugin_descriptor(descriptor) }, NC_OK);
        }

        assert_eq!(runtime.read_catalog().plugin_registry.registered_plugin_ids(), expected_order);
        assert_eq!(
            runtime.read_catalog().type_registry.registered_class_ids(),
            [COUNTER_CLASS_ID, FLAG_CLASS_ID, METRIC_CLASS_ID, STATE_CLASS_ID, EXTRA_CLASS_ID,]
        );
        assert_eq!(
            crate::inspection::plugin_dependency_ids(&runtime.read_catalog().plugin_registry, EXTRA_PLUGIN_ID,).unwrap(),
            vec![STATE_PLUGIN_ID]
        );
        assert_eq!(
            crate::inspection::plugin_class_ids(&runtime.read_catalog().type_registry, METRIC_PLUGIN_ID,),
            vec![METRIC_CLASS_ID]
        );
        assert_eq!(
            crate::inspection::class_interface_ids(&runtime.read_catalog().type_registry, STATE_CLASS_ID,).unwrap(),
            vec![IUnknown::INTERFACE_ID, NcInterfaceId::new(INTERFACE_ID, INTERFACE_SIGNATURE),]
        );
        assert_eq!(
            crate::inspection::interface_implementor_class_ids(&runtime.read_catalog().type_registry, NcInterfaceId::new(INTERFACE_ID, INTERFACE_SIGNATURE),)
                .unwrap(),
            vec![COUNTER_CLASS_ID, FLAG_CLASS_ID, METRIC_CLASS_ID, STATE_CLASS_ID, EXTRA_CLASS_ID,]
        );
        let class_info = crate::inspection::class_info(&runtime.read_catalog().type_registry, EXTRA_CLASS_ID).unwrap();
        assert_eq!(class_info.plugin_id, EXTRA_PLUGIN_ID);
        assert_eq!(class_info.module_id, 0);

        MULTI_PLUGIN_CONSTRUCTOR_CALLS.store(0, Ordering::SeqCst);
        let mut out = core::ptr::null_mut();
        let class = crate::construction::resolve_class(
            &runtime.read_catalog().type_registry,
            EXTRA_CLASS_ID,
            neocogi_com_core::NC_ABI_THREADING_THREAD_LOCAL,
        );
        assert_eq!(
            unsafe { crate::construction::construct_class(class, NcAbiSignature::EMPTY, core::ptr::null(), &mut out,) },
            NC_E_INVALID_ARG
        );
        assert!(out.is_null());
        assert_eq!(MULTI_PLUGIN_CONSTRUCTOR_CALLS.load(Ordering::SeqCst), 1);

        for descriptor in descriptors {
            assert_eq!(unsafe { runtime.register_plugin_descriptor(descriptor) }, NC_OK);
        }
        assert_eq!(runtime.read_catalog().plugin_registry.registered_plugin_ids().len(), 5);
        assert_eq!(runtime.read_catalog().type_registry.registered_class_ids().len(), 5);
    }

    #[test]
    fn simultaneous_exact_registration_is_blocking_and_idempotent() {
        const PLUGIN_ID: NcGuid = neocogi_com_core::guid_from_qualified_name("nc.tests.registry.concurrent_plugin");
        static EMPTY_CLASSES: NcClassTable = NcClassTable {
            class_count: 0,
            classes: core::ptr::null(),
        };
        static PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
            core_abi_version: NC_COM_CORE_ABI_VERSION,
            validation_mode: neocogi_com_core::NC_ABI_VALIDATION_MODE,
            plugin_id: PLUGIN_ID,
            qualified_name: NcStaticUtf8::from_static("nc.tests.registry.concurrent_plugin"),
            plugin_version: NcStaticUtf8::from_static("1.0.0"),
            dependency_count: 0,
            dependencies: core::ptr::null(),
            class_table: &EMPTY_CLASSES,
        };

        let runtime = Arc::new(RuntimeState::new());
        let start = Arc::new(Barrier::new(3));
        let mut threads = Vec::new();
        for _ in 0..2 {
            let runtime = Arc::clone(&runtime);
            let start = Arc::clone(&start);
            threads.push(std::thread::spawn(move || {
                start.wait();
                // SAFETY: `PLUGIN` and its empty class table are static.
                unsafe { runtime.register_plugin_descriptor(&PLUGIN) }
            }));
        }

        start.wait();
        for thread in threads {
            assert_eq!(thread.join().unwrap(), NC_OK);
        }
        assert_eq!(runtime.read_catalog().plugin_registry.registered_plugin_ids(), vec![PLUGIN_ID]);
    }

    #[test]
    fn identity_batch_requires_a_one_to_one_mapping() {
        let id_a = NcGuid::from_fields(1, 0, 0, [0; 8]);
        let id_b = NcGuid::from_fields(2, 0, 0, [0; 8]);
        let mut identities = RegistrationIdentityBatch::with_capacity(2);
        assert!(identities.push(id_a, "nc.tests.Identity").is_ok());
        assert!(identities.push(id_a, "nc.tests.Identity").is_ok());
        assert_eq!(identities.push(id_a, "nc.tests.OtherIdentity"), Err(NC_E_INVALID_ARG));
        assert_eq!(identities.push(id_b, "nc.tests.Identity"), Err(NC_E_INVALID_ARG));
    }

    #[test]
    fn malformed_direct_registration_cannot_bypass_batch_validation() {
        static PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
            core_abi_version: NC_COM_CORE_ABI_VERSION + 1,
            validation_mode: neocogi_com_core::NC_ABI_VALIDATION_MODE,
            plugin_id: NcGuid::nil(),
            qualified_name: NcStaticUtf8 { data: core::ptr::null(), len: 1 },
            plugin_version: NcStaticUtf8 { data: core::ptr::null(), len: 1 },
            dependency_count: 0,
            dependencies: core::ptr::null(),
            class_table: core::ptr::null(),
        };

        let runtime = RuntimeState::new();
        reset_registration_validation_calls();
        assert_eq!(unsafe { runtime.register_plugin_descriptor(&PLUGIN) }, NC_E_ABI_VERSION);
        assert_eq!(registration_validation_calls(), 1);
        assert!(runtime.read_catalog().plugin_registry.registered_plugin_ids().is_empty());
        assert!(runtime.read_catalog().type_registry.registered_class_ids().is_empty());
    }

    struct RegistrationBenchmarkFixture {
        _names: Vec<Rc<str>>,
        _classes: Rc<[NcClassDescriptor]>,
        _class_table: Rc<NcClassTable>,
        plugin: Box<NcPluginDescriptor>,
    }

    impl RegistrationBenchmarkFixture {
        fn new(class_count: usize) -> Self {
            static CLASS_INTERFACES: [NcClassInterfaceDescriptor; 1] = [NcClassInterfaceDescriptor {
                interface_descriptor: &IUnknown::INTERFACE_DESCRIPTOR,
            }];

            // The descriptors below retain raw pointers into this graph. Use
            // shared allocations so moving fixture fields or reallocating the
            // owner vector cannot uniquely retag the pointed-to storage and
            // invalidate those pointers under Stacked Borrows.
            let mut names: Vec<Rc<str>> = Vec::with_capacity(class_count + 1);
            names.push("nc.tests.registry.benchmark_plugin".into());
            for index in 0..class_count {
                names.push(format!("nc.tests.registry.BenchmarkClass{index}").into());
            }

            let classes: Rc<[NcClassDescriptor]> = names[1..]
                .iter()
                .map(|name| NcClassDescriptor {
                    class_id: neocogi_com_core::guid_from_qualified_name(name),
                    qualified_name: benchmark_text(name),
                    class_version: NcStaticUtf8::from_static("1.0.0"),
                    constructor_abi_signature: NcAbiSignature::EMPTY,
                    threading: neocogi_com_core::NC_ABI_THREADING_THREAD_LOCAL,
                    construct: Some(test_constructor),
                    interface_count: CLASS_INTERFACES.len() as u32,
                    interfaces: CLASS_INTERFACES.as_ptr(),
                })
                .collect::<Vec<_>>()
                .into();
            let class_table = Rc::new(NcClassTable {
                class_count: classes.len() as u32,
                classes: classes.as_ptr(),
            });
            let plugin_name = &names[0];
            let plugin = Box::new(NcPluginDescriptor {
                core_abi_version: NC_COM_CORE_ABI_VERSION,
                validation_mode: neocogi_com_core::NC_ABI_VALIDATION_MODE,
                plugin_id: neocogi_com_core::guid_from_qualified_name(plugin_name),
                qualified_name: benchmark_text(plugin_name),
                plugin_version: NcStaticUtf8::from_static("1.0.0"),
                dependency_count: 0,
                dependencies: core::ptr::null(),
                class_table: Rc::as_ptr(&class_table),
            });

            Self {
                _names: names,
                _classes: classes,
                _class_table: class_table,
                plugin,
            }
        }
    }

    fn benchmark_text(value: &Rc<str>) -> NcStaticUtf8 {
        NcStaticUtf8 {
            data: value.as_ptr(),
            len: value.len() as u32,
        }
    }

    #[test]
    fn class_collision_rejects_the_entire_registration_batch() {
        let first = RegistrationBenchmarkFixture::new(1);
        let mut conflicting = RegistrationBenchmarkFixture::new(2);
        conflicting._names.push("nc.tests.registry.benchmark_plugin_conflict".into());
        let conflicting_name = conflicting._names.last().unwrap();
        conflicting.plugin.plugin_id = neocogi_com_core::guid_from_qualified_name(conflicting_name);
        conflicting.plugin.qualified_name = benchmark_text(conflicting_name);

        let first_plugin_id = first.plugin.plugin_id;
        let conflicting_plugin_id = conflicting.plugin.plugin_id;
        let existing_class_id = first._classes[0].class_id;
        let otherwise_new_class_id = conflicting._classes[1].class_id;
        let runtime = RuntimeState::new();

        assert_eq!(unsafe { runtime.register_plugin_descriptor(first.plugin.as_ref()) }, NC_OK);
        assert_eq!(unsafe { runtime.register_plugin_descriptor(conflicting.plugin.as_ref()) }, NC_E_INVALID_ARG);
        assert_eq!(runtime.read_catalog().plugin_registry.registered_plugin_ids(), vec![first_plugin_id]);
        assert!(!runtime.read_catalog().plugin_registry.registered_plugin_ids().contains(&conflicting_plugin_id));
        assert_eq!(runtime.read_catalog().type_registry.registered_class_ids(), vec![existing_class_id]);
        assert!(!runtime.read_catalog().type_registry.registered_class_ids().contains(&otherwise_new_class_id));
    }

    #[test]
    #[ignore = "manual descriptor registration benchmark; run scripts/bench-registry.sh"]
    fn plugin_registration_scaling_benchmark() {
        use std::hint::black_box;
        use std::time::Instant;

        for class_count in [10_usize, 1_000, 10_000] {
            let fixture = RegistrationBenchmarkFixture::new(class_count);
            let runtime = RuntimeState::new();
            let total_start = Instant::now();
            let prepare_start = Instant::now();
            let batch = unsafe { build_plugin_registration_batch(black_box(fixture.plugin.as_ref()), None) }
                .unwrap_or_else(|error| panic!("batch failed with {}", error.status()));
            let prepare = prepare_start.elapsed();
            let commit_start = Instant::now();
            let status = runtime.commit_plugin_registration_batch(batch);
            let commit = commit_start.elapsed();
            let total = total_start.elapsed();
            assert_eq!(status, NC_OK);
            assert_eq!(runtime.read_catalog().type_registry.registered_class_ids().len(), class_count);
            println!("plugin_classes={class_count:>5} prepare={prepare:?} commit={commit:?} total={total:?}");
        }
    }
}
