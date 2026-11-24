//! Real runtime cycle identity, ordering, provider failures and destruction.

use neocogi_com_serialization::ncb as binary;
use std::{cell::Cell, rc::Rc};
use neocogi_com_sdk::{
    com_object, ComInterface, ComPtr, IUnknown, IUnknownVTable, INcBytesAbi, LocalCom, NcBytes, NcGuid, NcStatus, WeakComPtr, NC_E_BUSY, NC_E_INVALID_ARG,
    NC_E_NOT_FOUND, NC_E_OUT_OF_RANGE,
};
use neocogi_com_sdk::persistence::{IPersistable, IPersistableAbi, IPersistableVtblFor, Limits, SavedObject};

// A valid exact IUnknown table copy for generated common-header allocations.
// It changes dispatch storage, with identical lifecycle methods and identity.
static ALTERNATE: IUnknownVTable = IUnknownVTable {
    interface_id: &<IUnknown as ComInterface>::INTERFACE_ID,
    ..neocogi_com_sdk::lifetime::SHARED_UNKNOWN_VTABLE
};

/// An exact shared vtable that observes retention and forwards it unchanged.
static COUNTED_ROOT: IUnknownVTable = IUnknownVTable { add_ref: count_root_retain, ..ALTERNATE };

thread_local! {
    /// Number of explicit AddRef calls made through the probe on this thread.
    static ROOT_RETAINS: Cell<usize> = const { Cell::new(0) };
}

/// Observes one AddRef without changing the shared allocation's lifecycle.
///
/// # Safety
/// The object must have the live shared backing required by the original slot.
unsafe extern "C" fn count_root_retain(object: *mut core::ffi::c_void) -> u32 {
    ROOT_RETAINS.with(|count| count.set(count.get() + 1));
    // SAFETY: The probe has the exact backing precondition of the original slot.
    unsafe { (ALTERNATE.add_ref)(object) }
}

/// Legal alternate exact IUnknown view transfers the same retained allocation.
fn alternate(owner: ComPtr<IUnknown>) -> ComPtr<IUnknown> {
    let mut raw = owner.into_raw();
    assert_ne!(raw.vtable, &ALTERNATE as *const _);
    raw.vtable = &ALTERNATE;
    // SAFETY: This exact duplicate table supports the same generated allocation.
    // No ownership changed: the input's one strong reference is transferred.
    unsafe { ComPtr::from_raw_owned(raw) }.unwrap()
}

/// Exhaustive injected getter boundary; all other operations retain real behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    /// Child count getter.
    Count,
    /// State object acquisition.
    State,
    /// Indexed child acquisition, permitting failure after earlier child completion.
    Child(usize),
    /// Type identifier after child traversal.
    Type,
    /// Schema version after type identification.
    Schema,
    /// Failure while obtaining state length from its nested byte provider.
    BytesLen,
    /// Failure while copying the admitted nested provider bytes.
    BytesCopy,
    /// Foreign length content that cannot fit a Rust byte vector, despite a
    /// permissive save limits. No copy or allocation may be attempted.
    BytesOversize,
}

/// External observers use Cells because provider calls update counters through &self.
/// They observe work/drop independently of the provider and capture ownership.
#[derive(Default)]
struct Metrics {
    /// Child-count implementation entries.
    count: Cell<usize>,
    /// State acquisition entries.
    state: Cell<usize>,
    /// Indexed child implementation entries.
    child: Cell<usize>,
    /// Final type getter entries.
    type_id: Cell<usize>,
    /// Final schema getter entries.
    schema: Cell<usize>,
    /// Concrete provider destruction, independent of final weak-token release.
    drops: Cell<usize>,
}
impl Metrics {
    /// Compare observable implementation work independently of the returned status.
    fn entries(&self) -> [usize; 5] {
        [self.count.get(), self.state.get(), self.child.get(), self.type_id.get(), self.schema.get()]
    }
}

/// Weak graph edge avoids fixture-owned cycles except in explicit strong-cycle cases.
struct Edge {
    /// Header-retaining weak reference to the target allocation.
    target: WeakComPtr<IUnknown>,
    /// Return the exact alternate IUnknown dispatch table for the same allocation.
    alternate: bool,
}
/// Concrete provider graph; LocalCom owns mutation and enforces real call gates.
#[com_object(interfaces(IPersistable))]
struct Node {
    /// Ordered child graph exposed by getters.
    edges: Vec<Edge>,
    /// One exact injected boundary; every other operation is real.
    fail: Option<Stage>,
    /// Selected state byte count for resource admission tests.
    state_len: usize,
    /// External observers outlive this concrete provider.
    metrics: Rc<Metrics>,
    /// Distinct state/type identity for decoded-tree controls.
    tag: u8,
    /// Explicit provider-owned strong cycles, broken only by fixture teardown.
    keepalive: Vec<ComPtr<IUnknown>>,
}
impl Node {
    /// Record the actual provider entry before selecting its injected error.
    fn enter(&self, stage: Stage, counter: &Cell<usize>) -> Result<(), NcStatus> {
        counter.set(counter.get() + 1);
        if self.fail == Some(stage) { Err(NC_E_NOT_FOUND) } else { Ok(()) }
    }
}
impl Drop for Node {
    fn drop(&mut self) {
        self.metrics.drops.set(self.metrics.drops.get() + 1);
    }
}
impl IPersistable for Node {
    fn type_id(&self) -> Result<NcGuid, NcStatus> {
        self.enter(Stage::Type, &self.metrics.type_id)?;
        Ok(NcGuid::from_uuid_bytes([self.tag; 16]))
    }
    fn schema_version(&self) -> Result<u32, NcStatus> {
        self.enter(Stage::Schema, &self.metrics.schema)?;
        Ok(1)
    }
    fn save_state(&self) -> Result<ComPtr<INcBytesAbi>, NcStatus> {
        self.enter(Stage::State, &self.metrics.state)?;
        let bytes = NcBytes::from_vec(vec![self.tag; self.state_len]);
        Ok(match self.fail {
            Some(Stage::BytesLen) => failing_bytes(bytes, 0),
            Some(Stage::BytesCopy) => failing_bytes(bytes, 1),
            Some(Stage::BytesOversize) => failing_bytes(bytes, 2),
            _ => bytes,
        })
    }
    fn child_count(&self) -> Result<u32, NcStatus> {
        self.enter(Stage::Count, &self.metrics.count)?;
        Ok(self.edges.len() as u32)
    }
    fn child(&self, index: u32) -> Result<ComPtr<IUnknown>, NcStatus> {
        self.enter(Stage::Child(index as usize), &self.metrics.child)?;
        let edge = self.edges.get(index as usize).ok_or(NC_E_OUT_OF_RANGE)?;
        let owner = edge.target.upgrade()?.ok_or(NC_E_NOT_FOUND)?;
        Ok(if edge.alternate { alternate(owner) } else { owner })
    }
}

// These immutable alternate tables retain the real byte provider's allocation,
// identity and lifecycle functions. The selected operation fails or reports
// oversized length content; backing storage and every pointer remain valid.
// They outlive every returned byte owner on this test thread; no registry or
// production implementation participates in this test-only dispatch substitution.
thread_local! {
    /// Thread-confined immutable tables; each capture releases byte owners before
    /// thread teardown. OnceCell supplies stable addresses without a Send promise.
    static BYTE_TABLES: std::cell::OnceCell<[neocogi_com_sdk::INcBytesVTable; 3]> = const { std::cell::OnceCell::new() };
    /// Synchronous capture stays on this thread; independent tests cannot alter counts.
    static BYTE_ERRORS: Cell<[usize; 2]> = const { Cell::new([0, 0]) };
}

/// Length rejection must stop before runtime allocates/copies provider state.
/// # Safety
/// Same live byte allocation contract as INcBytesVTable::len; no output is written.
unsafe extern "C" fn reject_length(_: *mut core::ffi::c_void, _: *mut u64) -> NcStatus {
    BYTE_ERRORS.with(|value| {
        let mut counts = value.get();
        counts[0] += 1;
        value.set(counts);
    });
    NC_E_NOT_FOUND
}

/// Copy rejection must discard private runtime storage and deliver no document.
/// # Safety
/// Same admitted initialized destination contract as INcBytesVTable::copy_to.
unsafe extern "C" fn reject_copy(_: *mut core::ffi::c_void, _: u64, _: u64, _: *mut u8) -> NcStatus {
    BYTE_ERRORS.with(|value| {
        let mut counts = value.get();
        counts[1] += 1;
        value.set(counts);
    });
    NC_E_NOT_FOUND
}

/// Reports an unrepresentable foreign length through valid output storage.
/// The test never grants a read/copy of that fictitious byte range: admission
/// must reject the length before touching the real provider's byte payload.
/// # Safety
/// `out` is writable for one u64, as required by the byte length slot.
unsafe extern "C" fn oversized_length(_: *mut core::ffi::c_void, out: *mut u64) -> NcStatus {
    BYTE_ERRORS.with(|value| {
        let mut counts = value.get();
        counts[0] += 1;
        value.set(counts);
    });
    // SAFETY: The checked COM caller lends a live, writable length output.
    unsafe { out.write(isize::MAX as u64 + 1) };
    neocogi_com_sdk::NC_OK
}

/// Transfers one actual NcBytes owner through a table with one selected probe.
fn failing_bytes(owner: ComPtr<INcBytesAbi>, stage: usize) -> ComPtr<INcBytesAbi> {
    let mut raw = owner.into_raw();
    raw.vtable = BYTE_TABLES.with(|cell| {
        let tables = cell.get_or_init(|| {
            // SAFETY: this owned actual byte object protects its immutable static table.
            let original = unsafe { &*raw.vtable };
            let table = |len, copy_to| neocogi_com_sdk::INcBytesVTable {
                unknown: IUnknownVTable {
                    interface_id: original.unknown.interface_id,
                    query_interface: original.unknown.query_interface,
                    add_ref: original.unknown.add_ref,
                    release: original.unknown.release,
                    downgrade: original.unknown.downgrade,
                },
                len,
                byte_at: original.byte_at,
                copy_to,
            };
            [
                table(reject_length, original.copy_to),
                table(original.len, reject_copy),
                table(oversized_length, reject_copy),
            ]
        });
        &tables[stage] as *const _
    });
    // SAFETY: same allocation/identity/lifecycle and one transferred strong owner;
    // only the selected byte operation fails or returns malformed length content.
    unsafe { ComPtr::from_raw_owned(raw) }.unwrap()
}

/// Both real runtime entrypoints use the SDK byte-copy policy. Empty state still
/// queries length but never calls copy_to; nonempty failures publish no document.
/// Even permissive limits reject Rust-vector-unrepresentable lengths as range
/// errors before allocation/copy. Provider owners are released on every outcome.
#[test]
fn snapshot_state_uses_the_sdk_bounded_copy_contract() {
    let defaults = Limits::default();
    let permissive = Limits {
        binary: binary::Limits {
            bytes: usize::MAX,
            elements: usize::MAX,
            ..defaults.binary
        },
        ..defaults
    };
    for which in [Entry::Safe, Entry::Raw] {
        for (fail, state_len, limits, expected, byte_calls) in [
            (None, 2, defaults, None, [0, 0]),
            (Some(Stage::BytesCopy), 0, defaults, None, [0, 0]),
            (Some(Stage::BytesLen), 0, defaults, Some(NC_E_NOT_FOUND), [1, 0]),
            (Some(Stage::BytesCopy), 2, defaults, Some(NC_E_NOT_FOUND), [0, 1]),
            (Some(Stage::BytesOversize), 0, permissive, Some(NC_E_OUT_OF_RANGE), [1, 0]),
        ] {
            let metrics = Rc::new(Metrics::default());
            let local = Node::into_local(Node {
                edges: vec![],
                fail,
                state_len,
                metrics: metrics.clone(),
                tag: 7,
                keepalive: vec![],
            });
            let root = local.query::<IUnknown>().unwrap();
            let before = BYTE_ERRORS.with(Cell::get);
            let result = which.capture(&root, limits);
            let after = BYTE_ERRORS.with(Cell::get);
            assert_eq!(result.as_ref().err().copied(), expected, "{which:?} {fail:?} state={state_len}");
            assert_eq!([after[0] - before[0], after[1] - before[1]], byte_calls);
            let completed = usize::from(expected.is_none());
            assert_eq!(metrics.entries(), [1, 1, 0, completed, completed]);
            if let Ok(bytes) = result {
                let record: SavedObject = binary::from_slice(&bytes).unwrap();
                assert_eq!(record.state, vec![7; state_len]);
                assert!(record.children.is_empty());
            }
            drop(root);
            drop(local);
            assert_eq!(metrics.drops.get(), 1);
        }
    }
}

/// Both entrypoints reach the actual linked runtime, never a copied algorithm.
#[derive(Clone, Copy, Debug)]
enum Entry {
    /// Facade method validates and owns the complete returned document.
    Safe,
    /// Fixed ABI table publishes bytes through a synchronous typed sink.
    Raw,
}
impl Entry {
    /// Raw delivery stays untouched on capture failure and matches safe bytes on success.
    fn capture(self, root: &ComPtr<IUnknown>, limits: Limits) -> Result<Vec<u8>, NcStatus> {
        match self {
            Self::Safe => neocogi_com_sdk::persistence::save_object_tree_with_limits(&crate::host_runtime(), root, limits),
            Self::Raw => {
                let mut sink = Sink { bytes: Vec::new(), calls: 0 };
                // SAFETY: linked table is static; root and exclusive sink outlive the call.
                let status = unsafe {
                    ((*neocogi_com_host::initialize_process().persistence).save_object_tree)(
                        root.as_raw(),
                        limits.into(),
                        Some(deliver),
                        core::ptr::from_mut(&mut sink).cast(),
                    )
                };
                if status.is_err() {
                    assert_eq!(sink.calls, 0);
                    Err(status)
                } else {
                    assert_eq!(sink.calls, 1);
                    Ok(sink.bytes)
                }
            }
        }
    }
}

/// Zero depth or object budget wins before the runtime attempts root acquisition.
#[test]
fn snapshot_checks_root_budget_before_retaining() {
    let local = Node::into_local(Node {
        edges: vec![],
        fail: None,
        state_len: 0,
        metrics: Rc::new(Metrics::default()),
        tag: 0,
        keepalive: Vec::new(),
    });
    let mut raw = local.query::<IUnknown>().unwrap().into_raw();
    raw.vtable = &COUNTED_ROOT;
    // SAFETY: The shared allocation transfers one live owner and the alternate
    // table preserves identity and forwards the original lifecycle operations.
    let root = unsafe { ComPtr::from_raw_owned(raw) }.unwrap();
    let limits = Limits::default();
    for entry in [Entry::Safe, Entry::Raw] {
        ROOT_RETAINS.with(|count| count.set(0));
        assert_eq!(entry.capture(&root, Limits { objects: 0, ..limits }), Err(NC_E_OUT_OF_RANGE));
        assert_eq!(entry.capture(&root, Limits { depth: 0, ..limits }), Err(NC_E_OUT_OF_RANGE));
        assert_eq!(ROOT_RETAINS.with(Cell::get), 0);
        assert!(entry.capture(&root, limits).is_ok());
        assert_eq!(ROOT_RETAINS.with(Cell::get), 1);
    }
}

/// Caller-owned synchronous byte sink; runtime never retains its address.
struct Sink {
    /// Owned copy of runtime-published bytes.
    bytes: Vec<u8>,
    /// Zero until capture and final encoding have both succeeded.
    calls: usize,
}
/// Copies published bytes only while the callback's source and destination are live.
/// # Safety
/// Context exclusively points to Sink; bytes is readable for length during this call.
unsafe extern "C" fn deliver(context: *mut core::ffi::c_void, bytes: *const u8, length: usize) -> NcStatus {
    // SAFETY: the runtime and Entry::capture establish the stated storage obligations.
    let sink = unsafe { &mut *context.cast::<Sink>() };
    sink.calls += 1;
    sink.bytes.extend_from_slice(unsafe { core::slice::from_raw_parts(bytes, length) });
    neocogi_com_sdk::NC_OK
}

/// Run an identified graph through each real API and assert status/work/final cleanup.
fn run_graph(
    name: &str,
    edges: &[&[(usize, bool)]],
    fail: Option<(usize, Stage)>,
    state_len: usize,
    limits: Limits,
    expected: Option<NcStatus>,
    expected_entries: &[[usize; 5]],
) {
    for which in [Entry::Safe, Entry::Raw] {
        let metrics: Vec<_> = edges.iter().map(|_| Rc::new(Metrics::default())).collect();
        let locals: Vec<LocalCom<Node>> = metrics
            .iter()
            .enumerate()
            .map(|(index, metrics)| {
                Node::into_local(Node {
                    edges: Vec::new(),
                    fail: fail.filter(|(target, _)| *target == index).map(|(_, stage)| stage),
                    state_len,
                    metrics: metrics.clone(),
                    tag: index as u8,
                    keepalive: Vec::new(),
                })
            })
            .collect();
        let owners: Vec<_> = locals.iter().map(|node| node.query::<IUnknown>().unwrap()).collect();
        let weak: Vec<_> = owners.iter().map(|owner| owner.downgrade().unwrap()).collect();
        for (index, targets) in edges.iter().enumerate() {
            locals[index].try_borrow_mut().unwrap().edges = targets
                .iter()
                .map(|(target, alt)| Edge {
                    target: weak[*target].clone(),
                    alternate: *alt,
                })
                .collect();
            if name.starts_with("strong cycle") {
                locals[index].try_borrow_mut().unwrap().keepalive = targets.iter().map(|(target, _)| owners[*target].clone()).collect();
            }
        }
        // Real projection queries have one allocation identity. Exact query of
        // the alternate table deliberately preserves the selected table address.
        let persistence = owners[0].query::<IPersistableAbi>().unwrap();
        assert!(owners[0].as_com_ref().same_object(persistence.as_com_ref()));
        drop(persistence);
        let alias = alternate(owners[0].clone());
        assert!(alias.as_com_ref().same_object(owners[0].as_com_ref()));
        assert_eq!(alias.query::<IUnknown>().unwrap().as_raw(), alias.as_raw());
        drop(alias);

        let root = if name.starts_with("alternate root") {
            alternate(owners[0].clone())
        } else {
            owners[0].clone()
        };
        let before_bytes = BYTE_ERRORS.with(Cell::get);
        let result = which.capture(&root, limits);
        let after_bytes = BYTE_ERRORS.with(Cell::get);
        if let Some((_, stage)) = fail {
            match stage {
                Stage::BytesLen => assert_eq!(after_bytes, [before_bytes[0] + 1, before_bytes[1]]),
                Stage::BytesCopy => assert_eq!(after_bytes, [before_bytes[0], before_bytes[1] + 1]),
                _ => assert_eq!(after_bytes, before_bytes),
            }
        }
        drop(root);
        assert_eq!(result.as_ref().err().copied(), expected, "{name} {which:?}");
        if let Ok(bytes) = &result {
            let record: SavedObject = binary::from_slice(bytes).unwrap();
            assert_eq!(record.children.len(), edges[0].len());
            if name == "repeated sibling" {
                assert_eq!(record.children[0], record.children[1]);
            }
        }
        let observed: Vec<_> = metrics.iter().map(|m| m.entries()).collect();
        assert_eq!(observed, expected_entries, "{name} {which:?} provider entries");
        println!("{name} {which:?}: status={expected:?} entries={observed:?}");
        // All temporary capture owners must have gone away on success or error.
        // This explicit fixture teardown breaks provider-owned strong cycles;
        // A failed save leaves ownership with the application.
        for local in &locals {
            local.try_borrow_mut().unwrap().keepalive.clear();
        }
        drop(owners);
        drop(locals);
        assert!(weak.iter().all(|w| w.upgrade().unwrap().is_none()), "{name} {which:?} leaked strong ownership");
        assert!(metrics.iter().all(|m| m.drops.get() == 1), "{name} {which:?} final destruction");
    }
}

/// Exact cycle/budget/error semantics, provider work and owner destruction.
#[test]
fn retained_allocation_identity_through_actual_safe_and_raw_runtime() {
    let defaults = Limits::default();
    let completed_leaf = [1, 1, 0, 1, 1];
    run_graph("leaf", &[&[]], None, 2, Limits { objects: 1, depth: 1, ..defaults }, None, &[completed_leaf]);
    run_graph(
        "repeated sibling",
        &[&[(1, false), (1, true)], &[]],
        None,
        2,
        Limits { objects: 3, depth: 2, ..defaults },
        None,
        &[[1, 1, 2, 1, 1], [2, 2, 0, 2, 2]],
    );
    run_graph(
        "diamond DAG",
        &[&[(1, false), (2, false)], &[(3, true)], &[(3, false)], &[]],
        None,
        2,
        Limits { objects: 5, depth: 3, ..defaults },
        None,
        &[[1, 1, 2, 1, 1], [1, 1, 1, 1, 1], [1, 1, 1, 1, 1], [2, 2, 0, 2, 2]],
    );
    for depth in [0, 1, 2, 3] {
        let limits = Limits { depth, ..defaults };
        let status = Some(if depth < 2 { NC_E_OUT_OF_RANGE } else { NC_E_INVALID_ARG });
        let entries = match depth {
            0 => [0; 5],
            1 => [1, 0, 0, 0, 0],
            _ => [1, 1, 1, 0, 0],
        };
        run_graph(&format!("same-table self depth={depth}"), &[&[(0, false)]], None, 2, limits, status, &[entries]);
        run_graph(
            &format!("duplicate-table self depth={depth}"),
            &[&[(0, true)]],
            None,
            2,
            limits,
            status,
            &[entries],
        );
    }
    run_graph(
        "indirect cycle",
        &[&[(1, true)], &[(0, true)]],
        None,
        2,
        defaults,
        Some(NC_E_INVALID_ARG),
        &[[1, 1, 1, 0, 0]; 2],
    );
    run_graph(
        "strong cycle self",
        &[&[(0, true)]],
        None,
        2,
        defaults,
        Some(NC_E_INVALID_ARG),
        &[[1, 1, 1, 0, 0]],
    );
    run_graph(
        "strong cycle indirect",
        &[&[(1, true)], &[(0, true)]],
        None,
        2,
        defaults,
        Some(NC_E_INVALID_ARG),
        &[[1, 1, 1, 0, 0]; 2],
    );
    run_graph(
        "alternate root canonical child",
        &[&[(0, false)]],
        None,
        2,
        Limits { depth: 2, ..defaults },
        Some(NC_E_INVALID_ARG),
        &[[1, 1, 1, 0, 0]],
    );
    for objects in [0, 1, 2] {
        let limits = Limits { objects, ..defaults };
        let (status, entries) = match objects {
            0 => (NC_E_OUT_OF_RANGE, [0; 5]),
            1 => (NC_E_OUT_OF_RANGE, [1, 0, 0, 0, 0]),
            _ => (NC_E_INVALID_ARG, [1, 1, 1, 0, 0]),
        };
        run_graph(
            &format!("duplicate-table self objects={objects}"),
            &[&[(0, true)]],
            None,
            2,
            limits,
            Some(status),
            &[entries],
        );
    }
    for (stage, entries) in [
        (Stage::Count, [[1, 0, 0, 0, 0], [0; 5], [0; 5]]),
        (Stage::State, [[1, 1, 0, 0, 0], [0; 5], [0; 5]]),
        (Stage::Child(1), [[1, 1, 2, 0, 0], completed_leaf, [0; 5]]),
        (Stage::Type, [[1, 1, 2, 1, 0], completed_leaf, completed_leaf]),
        (Stage::Schema, [[1, 1, 2, 1, 1], completed_leaf, completed_leaf]),
        (Stage::BytesLen, [[1, 1, 0, 0, 0], [0; 5], [0; 5]]),
        (Stage::BytesCopy, [[1, 1, 0, 0, 0], [0; 5], [0; 5]]),
    ] {
        run_graph(
            &format!("provider error {stage:?}"),
            &[&[(1, false), (2, true)], &[], &[]],
            Some((0, stage)),
            2,
            defaults,
            Some(NC_E_NOT_FOUND),
            &entries,
        );
    }
    run_graph(
        "child provider error",
        &[&[(1, false)], &[]],
        Some((1, Stage::State)),
        2,
        defaults,
        Some(NC_E_NOT_FOUND),
        &[[1, 1, 1, 0, 0], [1, 1, 0, 0, 0]],
    );
    run_graph(
        "object budget before provider error",
        &[&[]],
        Some((0, Stage::Count)),
        2,
        Limits { objects: 0, ..defaults },
        Some(NC_E_OUT_OF_RANGE),
        &[[0; 5]],
    );
    run_graph(
        "depth budget before provider error",
        &[&[]],
        Some((0, Stage::Count)),
        2,
        Limits { depth: 0, ..defaults },
        Some(NC_E_OUT_OF_RANGE),
        &[[0; 5]],
    );
    run_graph(
        "count beyond object budget",
        &[&[(1, false), (1, true)], &[]],
        None,
        2,
        Limits { objects: 2, ..defaults },
        Some(NC_E_OUT_OF_RANGE),
        &[[1, 0, 0, 0, 0], [0; 5]],
    );
    run_graph(
        "count beyond element budget",
        &[&[(1, false), (1, true)], &[]],
        None,
        2,
        Limits {
            binary: binary::Limits { elements: 1, ..defaults.binary },
            ..defaults
        },
        Some(NC_E_OUT_OF_RANGE),
        &[[1, 0, 0, 0, 0], [0; 5]],
    );
    run_graph(
        "child depth before failing getter",
        &[&[(1, false)], &[]],
        Some((0, Stage::Child(0))),
        2,
        Limits { objects: 2, depth: 1, ..defaults },
        Some(NC_E_OUT_OF_RANGE),
        &[[1, 0, 0, 0, 0], [0; 5]],
    );
    run_graph(
        "getter failure at admitted depth and count",
        &[&[(1, false)], &[]],
        Some((0, Stage::Child(0))),
        2,
        Limits { objects: 2, depth: 2, ..defaults },
        Some(NC_E_NOT_FOUND),
        &[[1, 1, 1, 0, 0], [0; 5]],
    );
    // After the first child advertises its grandchild, the root's pending second
    // child already accounts for the last slot. Neither getter may run.
    let with_grandchild: &[&[(usize, bool)]] = &[&[(1, false), (2, false)], &[(3, false)], &[], &[]];
    run_graph(
        "grandchild exceeds count with pending sibling",
        with_grandchild,
        Some((0, Stage::Child(1))),
        2,
        Limits { objects: 3, depth: 3, ..defaults },
        Some(NC_E_OUT_OF_RANGE),
        &[[1, 1, 1, 0, 0], [1, 0, 0, 0, 0], [0; 5], [0; 5]],
    );
    run_graph(
        "grandchild and pending sibling exactly fit",
        with_grandchild,
        None,
        2,
        Limits { objects: 4, depth: 3, ..defaults },
        None,
        &[[1, 1, 2, 1, 1], [1, 1, 1, 1, 1], completed_leaf, completed_leaf],
    );
    run_graph(
        "provider-state bytes",
        &[&[]],
        None,
        100,
        Limits {
            binary: binary::Limits { bytes: 32, ..defaults.binary },
            ..defaults
        },
        Some(NC_E_OUT_OF_RANGE),
        &[[1, 1, 0, 0, 0]],
    );
    run_graph(
        "final codec budget",
        &[&[]],
        None,
        2,
        Limits {
            binary: binary::Limits { values: 1, ..defaults.binary },
            ..defaults
        },
        Some(NC_E_INVALID_ARG),
        &[completed_leaf],
    );
    for which in [Entry::Safe, Entry::Raw] {
        let local = Node::into_local(Node {
            edges: vec![],
            fail: None,
            state_len: 2,
            metrics: Rc::new(Metrics::default()),
            tag: 0,
            keepalive: Vec::new(),
        });
        let root = local.query::<IUnknown>().unwrap();
        let mutable = local.try_borrow_mut().unwrap();
        assert_eq!(which.capture(&root, defaults), Err(NC_E_BUSY));
        drop(mutable);
        assert!(which.capture(&root, defaults).is_ok());
        println!("local borrow {which:?}: BUSY while mutable, success after release");
    }
}
