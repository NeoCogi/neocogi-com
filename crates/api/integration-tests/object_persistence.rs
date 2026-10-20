//! Real COM factories exercise recursive snapshots, preflight, and teardown.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use neocogi_com::{
    binary, com_object, guid_from_qualified_name, ComDeserialize, ComSerialize, ComPtr, ComRef, INcBytesAbi, IUnknown, NcBytes, NcCopyFromBufferOffset, NcGuid,
    NcStatus, NC_E_ABI_VERSION, NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE,
};
use neocogi_com::persistence::*;

/// A live exclusive persistence provider is borrowable, but not shareable.
#[com_object(interfaces(IPersistable), ownership = exclusive)]
struct ExclusiveObject {
    /// Persistence getter invocations; rejected capture must not invoke any.
    calls: Arc<AtomicUsize>,
    /// Destruction is independent of projection bookkeeping lifetime.
    drops: Arc<AtomicUsize>,
}
impl Drop for ExclusiveObject {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::Relaxed);
    }
}
impl IPersistable for ExclusiveObject {
    fn type_id(&self) -> Result<NcGuid, NcStatus> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(KIND)
    }
    fn schema_version(&self) -> Result<u32, NcStatus> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(1)
    }
    fn save_state(&self) -> Result<ComPtr<INcBytesAbi>, NcStatus> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(NcBytes::from_vec(vec![1, 2, 3]))
    }
    fn child_count(&self) -> Result<u32, NcStatus> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(0)
    }
    fn child(&self, _index: u32) -> Result<ComPtr<IUnknown>, NcStatus> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Err(NC_E_OUT_OF_RANGE)
    }
}

/// Both public snapshot paths reject a real exclusive root across the runtime ABI.
#[test]
fn exclusive_snapshot_returns_status_before_getters_and_preserves_owner() {
    let calls = Arc::new(AtomicUsize::new(0));
    let drops = Arc::new(AtomicUsize::new(0));
    let owner = ExclusiveObject::into_box(ExclusiveObject {
        calls: calls.clone(),
        drops: drops.clone(),
    })
    .query::<IPersistableAbi>()
    .unwrap();
    let identity = owner.query_weak::<IUnknown>().unwrap();
    assert_eq!(snapshot(identity.as_com_ref()), Err(NC_E_INVALID_ARG));
    assert_eq!(snapshot_with_limits(identity.as_com_ref(), Limits::default()), Err(NC_E_INVALID_ARG));
    // SAFETY: The linked runtime table is process-lived. The live owner protects
    // identity, and the callback receives one exclusive local counter for this call.
    let api = unsafe { nc_com_get_persistence_api_v1().as_ref() }.unwrap();
    let mut deliveries = 0usize;
    let status = unsafe {
        (api.snapshot)(
            identity.as_raw(),
            Limits::default().into(),
            Some(count_delivery),
            core::ptr::from_mut(&mut deliveries).cast(),
        )
    };
    assert_eq!(status, NC_E_INVALID_ARG);
    assert_eq!(deliveries, 0);
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    assert_eq!(drops.load(Ordering::Relaxed), 0);
    assert_eq!(owner.schema_version().unwrap(), 1);
    drop(owner);
    assert_eq!(drops.load(Ordering::Relaxed), 1);
    assert!(!identity.is_alive());
}

/// Counts attempted delivery; rejected capture must never reach this sink.
///
/// # Safety
/// `data` exclusively points to the live local usize installed by the test.
unsafe extern "C" fn count_delivery(data: *mut core::ffi::c_void, _bytes: *const u8, _len: usize) -> NcStatus {
    // SAFETY: The test supplies and retains the exact counter for the synchronous call.
    unsafe { *data.cast::<usize>() += 1 };
    neocogi_com::NC_OK
}

/// A raw registration caller can lend an exclusive projection without owning a shared handle.
#[com_object(interfaces(IPersistenceFactory), ownership = exclusive)]
struct ExclusiveFactory;
impl IPersistenceFactory for ExclusiveFactory {
    fn validate(&self, _version: u32, _state: NcCopyFromBufferOffset<'_, u8>, _count: u32) -> Result<(), NcStatus> {
        Ok(())
    }
    fn restore(&self, _version: u32, _state: NcCopyFromBufferOffset<'_, u8>, _children: ComRef<'_, IObjectChildrenAbi>) -> Result<ComPtr<IUnknown>, NcStatus> {
        Err(NC_E_INVALID_ARG)
    }
}

unsafe extern "C" {
    /// Linked runtime getter; the facade's runtime feature selects its implementation.
    fn nc_com_get_persistence_api_v1() -> *const neocogi_com::NcPersistenceApiV1;
}

/// Rejection across the raw table leaves the registry empty and usable for shared registration.
#[test]
fn raw_factory_registration_rejects_exclusive_projection_without_mutation() {
    let owner = ExclusiveFactory::into_box(ExclusiveFactory).query::<IPersistenceFactoryAbi>().unwrap();
    // SAFETY: The linked runtime lends an immutable table for the process lifetime.
    let api = unsafe { nc_com_get_persistence_api_v1().as_ref() }.unwrap();
    let mut context = core::ptr::null_mut();
    // SAFETY: Fresh local output; all context operations remain sequential on this thread.
    assert_eq!(unsafe { (api.create_context)(&mut context) }, neocogi_com::NC_OK);
    // SAFETY: Exclusive access to the new context and a valid, live COM projection.
    assert_eq!(unsafe { (api.register_factory)(context, KIND, owner.as_raw()) }, NC_E_INVALID_ARG);
    assert!(owner.downgrade().is_alive());
    owner.validate(1, NcCopyFromBufferOffset::new(0, &[]), 0).unwrap();
    let counts = Arc::new(Counts::default());
    let factory = Factory::from_value(Factory { counts: counts.clone() })
        .query::<IPersistenceFactoryAbi>()
        .unwrap();
    // SAFETY: Same exclusive context access and a live shared provider; the rejected
    // registration must not reserve KIND or leave an invalid retained reference.
    assert_eq!(unsafe { (api.register_factory)(context, KIND, factory.as_raw()) }, neocogi_com::NC_OK);
    drop(factory);
    assert_eq!(counts.factories_dropped.load(Ordering::Relaxed), 0);
    // SAFETY: Return the sole context owner once, with no active table calls.
    unsafe { (api.destroy_context)(context) };
    assert_eq!(counts.factories_dropped.load(Ordering::Relaxed), 1);
    let weak = owner.downgrade();
    drop(owner);
    assert!(!weak.is_alive());
}

/// Shareable objects without persistence still report ordinary capability absence.
#[test]
fn shared_root_without_persistence_returns_nointerface() {
    let owner = Factory::from_value(Factory { counts: Arc::new(Counts::default()) });
    assert_eq!(snapshot(owner.as_com_ref()), Err(neocogi_com::NC_E_NOINTERFACE));
}

/// Explicit recipe identity, unrelated to individual object addresses.
const KIND: NcGuid = guid_from_qualified_name("test.persistence.group");
/// Complete mutable configuration selected for persistence.
#[derive(Clone, Debug, PartialEq, ComSerialize, ComDeserialize)]
struct State {
    /// Author-visible label.
    label: String,
    /// An ordinary copied property.
    enabled: bool,
}
/// Per-test construction/destruction observations, shared only for assertions.
#[derive(Default)]
struct Counts {
    /// Constructors actually invoked after preflight.
    created: AtomicUsize,
    /// Live fixture objects not yet destroyed.
    live: AtomicUsize,
    /// Factory destructors observed after registry ownership has been released.
    factories_dropped: AtomicUsize,
}

/// Concrete persistent object; private runtime instrumentation is not serialized.
#[com_object(interfaces(IPersistable))]
struct Object {
    /// Authored settings owned by this instance.
    state: State,
    /// Sole owned child list; no parent references or cycles.
    children: Vec<ComPtr<IUnknown>>,
    /// Test instrumentation, deliberately excluded from the saved state.
    counts: Arc<Counts>,
}
impl Drop for Object {
    /// Construction failures must release every partially constructed descendant.
    fn drop(&mut self) {
        self.counts.live.fetch_sub(1, Ordering::Relaxed);
    }
}
impl IPersistable for Object {
    /// Returns the recipe identity used by this factory.
    fn type_id(&self) -> Result<NcGuid, NcStatus> {
        Ok(KIND)
    }
    /// Revision one describes State in declaration order.
    fn schema_version(&self) -> Result<u32, NcStatus> {
        Ok(1)
    }
    /// Export observes the current configuration, not its original input bytes.
    fn save_state(&self) -> Result<ComPtr<INcBytesAbi>, NcStatus> {
        Ok(NcBytes::from_vec(binary::to_vec(&self.state).map_err(|_| NC_E_INVALID_ARG)?))
    }
    /// Child order belongs to the owning object.
    fn child_count(&self) -> Result<u32, NcStatus> {
        Ok(self.children.len() as u32)
    }
    /// Exports one borrowed tree edge as an owned COM reference.
    fn child(&self, index: u32) -> Result<ComPtr<IUnknown>, NcStatus> {
        self.children.get(index as usize).cloned().ok_or(NC_E_OUT_OF_RANGE)
    }
}

/// A context-bound provider may carry runtime services without persisting them.
#[com_object(interfaces(IPersistenceFactory))]
struct Factory {
    /// Observations shared with constructed objects.
    counts: Arc<Counts>,
}

impl Drop for Factory {
    /// Allows the tests to distinguish retained factories from leaked references.
    fn drop(&mut self) {
        self.counts.factories_dropped.fetch_add(1, Ordering::Relaxed);
    }
}

/// Contexts remain independent and retain/release factories through the runtime ABI.
#[test]
fn registry_ownership_is_local_and_releases_retained_factories() {
    let (first, counts) = registry();
    let second = FactoryRegistry::new().unwrap();
    let bytes = binary::to_vec(&record("root", vec![])).unwrap();
    // A recipe in one context must never become visible in another context.
    assert_eq!(second.restore(&bytes).err(), Some(neocogi_com::NC_E_NOT_FOUND));
    let root = first.restore(&bytes).unwrap();
    assert_eq!(counts.factories_dropped.load(Ordering::Relaxed), 0);
    drop(first);
    assert_eq!(counts.factories_dropped.load(Ordering::Relaxed), 1);
    // Releasing a registry does not invalidate objects it previously created.
    assert_eq!(snapshot(root.as_com_ref()).unwrap(), bytes);
    drop(root);
    assert_eq!(counts.live.load(Ordering::Relaxed), 0);
}
/// Validates a complete initialization without constructing an object.
fn decode(version: u32, state: NcCopyFromBufferOffset<'_, u8>) -> Result<State, NcStatus> {
    if version != 1 {
        return Err(NC_E_ABI_VERSION);
    }
    if state.offset() != 0 {
        return Err(NC_E_INVALID_ARG);
    }
    binary::from_slice(state.as_slice()).map_err(|_| NC_E_INVALID_ARG)
}
impl IPersistenceFactory for Factory {
    /// Invalid states and revisions fail during whole-tree preflight.
    fn validate(&self, version: u32, state: NcCopyFromBufferOffset<'_, u8>, child_count: u32) -> Result<(), NcStatus> {
        let state = decode(version, state)?;
        if state.label.is_empty() || child_count > 8 {
            return Err(NC_E_INVALID_ARG);
        }
        Ok(())
    }
    /// The simulated runtime failure occurs after children exist, testing cleanup.
    fn restore(&self, version: u32, state: NcCopyFromBufferOffset<'_, u8>, children: ComRef<'_, IObjectChildrenAbi>) -> Result<ComPtr<IUnknown>, NcStatus> {
        self.validate(version, state, children.count()?)?;
        let state = decode(version, state)?;
        if state.label == "runtime failure" {
            return Err(NC_E_INVALID_ARG);
        }
        let children = (0..children.count()?).map(|index| children.get(index)).collect::<Result<Vec<_>, _>>()?;
        self.counts.created.fetch_add(1, Ordering::Relaxed);
        self.counts.live.fetch_add(1, Ordering::Relaxed);
        Ok(Object::from_value(Object {
            state,
            children,
            counts: self.counts.clone(),
        }))
    }
}

/// Constructs typed initialization independently from live objects.
fn record(label: &str, children: Vec<ObjectRecord>) -> ObjectRecord {
    ObjectRecord {
        type_id: KIND.to_uuid_bytes(),
        schema_version: 1,
        state: binary::to_vec(&State { label: label.into(), enabled: true }).unwrap(),
        children,
    }
}
/// Registers the one explicitly selected provider and returns per-test counters.
fn registry() -> (FactoryRegistry, Arc<Counts>) {
    let counts = Arc::new(Counts::default());
    let factory = Factory::from_value(Factory { counts: counts.clone() }).query().unwrap();
    let mut registry = FactoryRegistry::new().unwrap();
    registry.register(KIND, factory).unwrap();
    (registry, counts)
}

/// Generated persistence projections share the root lifetime and keep their
/// exact interface usable after promotion, without keeping destroyed values alive.
#[test]
fn persistence_projections_share_weak_ownership_with_the_root() {
    let (registry, counts) = registry();
    let bytes = binary::to_vec(&record("weak persistence", vec![])).unwrap();
    let root = registry.restore(&bytes).unwrap();
    let projected = root.query::<IPersistableAbi>().unwrap();
    let weak_root = root.downgrade().unwrap();
    let weak_projection = projected.downgrade().unwrap();
    assert!(weak_root.same_object(&weak_projection));

    // Promotion preserves the requested persistence projection, not just IUnknown.
    let promoted = weak_projection.upgrade().unwrap().unwrap();
    assert_eq!(promoted.type_id().unwrap(), KIND);
    assert_eq!(promoted.schema_version().unwrap(), 1);
    drop(root);
    drop(projected);
    assert_eq!(counts.live.load(Ordering::Relaxed), 1);
    drop(promoted);
    assert_eq!(counts.live.load(Ordering::Relaxed), 0);
    assert!(weak_projection.upgrade().unwrap().is_none());
    assert!(weak_root.upgrade().unwrap().is_none());

    // Factories use separately generated bindings and must obey the same protocol.
    let factory_root = Factory::from_value(Factory { counts: counts.clone() });
    let factory = factory_root.query::<IPersistenceFactoryAbi>().unwrap();
    let weak_factory = factory.downgrade().unwrap();
    assert!(weak_factory.same_object(&factory_root.downgrade().unwrap()));
    let promoted_factory = weak_factory.upgrade().unwrap().unwrap();
    drop(factory_root);
    drop(factory);
    assert_eq!(counts.factories_dropped.load(Ordering::Relaxed), 0);
    drop(promoted_factory);
    assert_eq!(counts.factories_dropped.load(Ordering::Relaxed), 1);
    assert!(weak_factory.upgrade().unwrap().is_none());
}

/// Both persistence directions use real interface calls and preserve child order.
#[test]
fn recursive_objects_round_trip_through_their_factories() {
    let original = record("root", vec![record("first", vec![]), record("second", vec![record("leaf", vec![])])]);
    let bytes = binary::to_vec(&original).unwrap();
    let (registry, counts) = registry();
    let root = registry.restore(&bytes).unwrap();
    assert_eq!(counts.live.load(Ordering::Relaxed), 4);
    assert_eq!(snapshot(root.as_com_ref()).unwrap(), bytes);
    let copy = registry.restore(&snapshot(root.as_com_ref()).unwrap()).unwrap();
    drop(root);
    drop(copy);
    assert_eq!(counts.live.load(Ordering::Relaxed), 0);
}

/// Unknown recipes, malformed state and unsupported revisions construct nothing.
#[test]
fn preflight_checks_the_complete_tree_before_construction() {
    for case in 0..3 {
        let mut bad = record("bad", vec![]);
        match case {
            0 => bad.type_id = [0; 16],
            1 => bad.schema_version = 2,
            _ => bad.state.push(9),
        }
        let root = record("root", vec![record("valid sibling", vec![]), bad]);
        let (registry, counts) = registry();
        assert!(registry.restore(&binary::to_vec(&root).unwrap()).is_err());
        assert_eq!(counts.created.load(Ordering::Relaxed), 0);
    }
}

/// A failed parent releases already initialized children and their own descendants.
#[test]
fn runtime_construction_failure_releases_partial_tree() {
    let root = record("runtime failure", vec![record("child", vec![record("leaf", vec![])])]);
    let (registry, counts) = registry();
    assert!(registry.restore(&binary::to_vec(&root).unwrap()).is_err());
    assert_eq!(counts.created.load(Ordering::Relaxed), 2);
    assert_eq!(counts.live.load(Ordering::Relaxed), 0);
}

/// Explicit tree limits apply to restoring and saving live trees alike.
#[test]
fn tree_limits_and_duplicate_registration_preserve_existing_state() {
    let (mut registry, counts) = registry();
    let duplicate = Factory::from_value(Factory { counts: counts.clone() }).query().unwrap();
    assert!(registry.register(KIND, duplicate).is_err());
    let bytes = binary::to_vec(&record("parent", vec![record("child", vec![])])).unwrap();
    assert!(registry.restore_with_limits(&bytes, Limits { objects: 1, ..Limits::default() }).is_err());
    assert_eq!(counts.created.load(Ordering::Relaxed), 0);
    let root = registry.restore(&bytes).unwrap();
    assert!(snapshot_with_limits(root.as_com_ref(), Limits { depth: 1, ..Limits::default() }).is_err());
    drop(root);
    assert_eq!(counts.live.load(Ordering::Relaxed), 0);
}

/// Snapshot and restore expose the same binary limits through actual COM calls,
/// including the value cost of the envelope's ordinary byte-vector state.
#[test]
fn binary_limits_agree_across_snapshot_and_factory_restore() {
    let original = record("persistent label", vec![]);
    let bytes = binary::to_vec(&original).unwrap();
    let (registry, counts) = registry();
    let root = registry.restore(&bytes).unwrap();
    // This leaf envelope contains 21 values besides state bytes: one record,
    // one UUID array plus 16 bytes, one schema value, and two vector containers.
    let exact = Limits {
        binary: binary::Limits {
            bytes: bytes.len(),
            elements: original.state.len(),
            depth: 3,
            values: original.state.len() + 21,
        },
        objects: 1,
        depth: 1,
    };
    assert_eq!(snapshot_with_limits(root.as_com_ref(), exact).unwrap(), bytes);
    let restored = registry.restore_with_limits(&bytes, exact).unwrap();
    drop(restored);
    let created = counts.created.load(Ordering::Relaxed);

    for binary in [
        binary::Limits {
            bytes: exact.binary.bytes - 1,
            ..exact.binary
        },
        binary::Limits {
            elements: exact.binary.elements - 1,
            ..exact.binary
        },
        binary::Limits {
            depth: exact.binary.depth - 1,
            ..exact.binary
        },
        binary::Limits {
            values: exact.binary.values - 1,
            ..exact.binary
        },
    ] {
        let insufficient = Limits { binary, ..exact };
        assert!(snapshot_with_limits(root.as_com_ref(), insufficient).is_err());
        assert!(registry.restore_with_limits(&bytes, insufficient).is_err());
        // A resource rejection must precede factory acquisition, preserving the
        // already-live root and preventing construction of a partial copy.
        assert_eq!(counts.created.load(Ordering::Relaxed), created);
        assert_eq!(counts.live.load(Ordering::Relaxed), 1);
    }
    drop(root);
    assert_eq!(counts.live.load(Ordering::Relaxed), 0);
}

// This final test executable owns its isolated process runtime.
extern crate nc_com as _;
