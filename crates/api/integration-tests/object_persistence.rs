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
        // A resource rejection must precede factory activation, preserving the
        // already-live root and preventing construction of a partial copy.
        assert_eq!(counts.created.load(Ordering::Relaxed), created);
        assert_eq!(counts.live.load(Ordering::Relaxed), 1);
    }
    drop(root);
    assert_eq!(counts.live.load(Ordering::Relaxed), 0);
}
