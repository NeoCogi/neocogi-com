//! Ordinary object persistence regressions own one embedded runtime.

// Only this final root retains the implementation; shared qualification imports it.
extern crate neocogi_com_host as _;
include!("support/object_persistence_cases.rs");

#[path = "support/allocation_probe.rs"]
mod allocation_probe;

/// Refusing host storage must return a status without allocating diagnostics.
#[test]
fn save_reservations_fail_cleanly_and_allow_retry() {
    use std::alloc::Layout;
    use std::mem::MaybeUninit;
    use neocogi_com_sdk::NC_E_OUT_OF_MEMORY;

    let runtime = crate::host_runtime();
    let (registry, _) = registry();
    // Match a single-frame Vec growth layout without assuming std's capacities.
    // A frame has one typed owner followed by two u32 counters.
    type Frame = (ComPtr<IPersistableAbi>, u32, u32);
    let mut probe_stack = Vec::<MaybeUninit<Frame>>::new();
    probe_stack.try_reserve(1).unwrap();
    let initial = probe_stack.capacity();
    probe_stack.resize_with(initial, MaybeUninit::uninit);
    probe_stack.try_reserve(1).unwrap();
    let later = probe_stack.capacity();
    let mut tree = record(&"x".repeat(111), vec![]);
    for _ in 0..initial {
        tree = record("parent", vec![tree]);
    }
    let expected = binary::to_vec(&tree).unwrap();
    let root = registry.restore_object_tree(&expected).unwrap();

    for layout in [
        Layout::array::<Frame>(initial).unwrap(),
        Layout::array::<Frame>(later).unwrap(),
        Layout::array::<u8>(expected.len()).unwrap(),
    ] {
        let probe = allocation_probe::Probe::matching(layout, Some(1));
        let result = save_object_tree(&runtime, &root);
        let report = probe.finish();
        assert_eq!(report.denied, 1, "{layout:?}");
        assert_eq!(result, Err(NC_E_OUT_OF_MEMORY), "{layout:?}");
    }
    assert_eq!(save_object_tree(&runtime, &root).unwrap(), expected);
}

/// Pending siblings count before their storage or any recipe callback.
#[test]
fn restore_checks_declared_child_slots_before_descending() {
    let first = record("first", vec![record("left", vec![]), record("right", vec![])]);
    let sibling = record("sibling", vec![]);
    let root = record("root", vec![first, sibling]);
    let bytes = binary::to_vec(&root).unwrap();
    let (registry, counts) = registry();
    // NCB has no close framing. Omit the two grandchildren and the pending
    // sibling, leaving the first child's admitted length as the final count.
    let header_size = binary::to_vec(&()).unwrap().len();
    let wire_len = |value: &SavedObject| binary::to_vec(value).unwrap().len() - header_size;
    let unread_bytes = root.children[0].children.iter().map(wire_len).sum::<usize>() + wire_len(&root.children[1]);
    let prefix = &bytes[..bytes.len() - unread_bytes];
    assert!(prefix.ends_with(&2u32.to_le_bytes()));

    let exact = Limits {
        objects: 5,
        depth: 3,
        ..Limits::default()
    };
    for limits in [Limits { objects: 4, ..exact }, Limits { depth: 2, ..exact }] {
        for slots in [1, 2] {
            let layout = std::alloc::Layout::array::<SavedObject>(slots).unwrap();
            let probe = allocation_probe::Probe::matching(layout, None);
            let outcome = registry.restore_object_tree_with_limits(prefix, limits);
            let report = probe.finish();
            assert_eq!(outcome.err(), Some(NC_E_OUT_OF_RANGE));
            assert_eq!(report.attempts, 0, "no completed child should request storage");
        }
    }
    assert_eq!(registry.restore_object_tree_with_limits(prefix, exact).err(), Some(NC_E_INVALID_ARG));
    for limits in [Limits { objects: 0, ..exact }, Limits { depth: 0, ..exact }] {
        assert_eq!(registry.restore_object_tree_with_limits(&bytes, limits).err(), Some(NC_E_OUT_OF_RANGE));
        assert_eq!(registry.restore_object_tree_with_limits(b"NCB", limits).err(), Some(NC_E_INVALID_ARG));
    }
    assert_eq!(counts.validated.load(Ordering::Relaxed), 0);
    assert_eq!(counts.created.load(Ordering::Relaxed), 0);

    let restored = registry.restore_object_tree_with_limits(&bytes, exact).unwrap();
    assert_eq!(save_object_tree(&crate::host_runtime(), &restored).unwrap(), bytes);
    assert_eq!(counts.created.load(Ordering::Relaxed), 5);
    // SAFETY: The selected linked host publishes process-lived service tables.
    let api = unsafe { neocogi_com_host::initialize_process().persistence.as_ref() }.unwrap();
    let mut context = core::ptr::null_mut();
    // SAFETY: The writable local slot receives one thread-local context.
    assert_eq!(unsafe { (api.create_context)(&mut context) }, neocogi_com_sdk::NC_OK);
    let mut out = restored.as_raw();
    // SAFETY: The context is live on its creating thread, the prefix is readable,
    // and the output can be written without touching the borrowed sentinel owner.
    let status = unsafe { (api.restore_object_tree)(context, prefix.as_ptr(), prefix.len(), Limits { objects: 4, ..exact }.into(), &mut out) };
    // SAFETY: This test still owns the created context and no call is active.
    unsafe { (api.destroy_context)(context) };
    assert_eq!(status, NC_E_OUT_OF_RANGE);
    assert!(out.object.is_null() && out.vtable.is_null());
    drop(restored);
    assert_eq!(counts.live.load(Ordering::Relaxed), 0);
}

/// Binary admission can reject the same count before the object policy sees it.
#[test]
fn restore_preserves_binary_count_rejection() {
    let mut bytes = binary::to_vec(&SavedObject {
        type_id: KIND.to_uuid_bytes(),
        schema_version: 1,
        state: vec![],
        children: vec![],
    })
    .unwrap();
    let count_start = bytes.len() - 4;
    bytes[count_start..].copy_from_slice(&17u32.to_le_bytes());
    let (registry, counts) = registry();
    let mut limits = Limits { objects: 1, ..Limits::default() };
    assert_eq!(registry.restore_object_tree_with_limits(&bytes, limits).err(), Some(NC_E_OUT_OF_RANGE));
    limits.binary.elements = 16;
    assert_eq!(registry.restore_object_tree_with_limits(&bytes, limits).err(), Some(NC_E_INVALID_ARG));
    assert_eq!(counts.validated.load(Ordering::Relaxed), 0);
}

/// Local child reservation refusal returns through the real host table.
#[test]
fn restore_reports_host_child_storage_allocation_refusal() {
    let bytes = binary::to_vec(&record("root", vec![record("leaf", vec![])])).unwrap();
    let (registry, counts) = registry();
    let probe = allocation_probe::Probe::matching(std::alloc::Layout::new::<SavedObject>(), Some(1));
    let outcome = registry.restore_object_tree(&bytes);
    let report = probe.finish();
    assert_eq!(outcome.err(), Some(neocogi_com_sdk::NC_E_OUT_OF_MEMORY));
    assert_eq!(report.attempts, 1);
    assert_eq!(report.denied, 1);
    assert_eq!(counts.validated.load(Ordering::Relaxed), 0);
    assert_eq!(counts.created.load(Ordering::Relaxed), 0);
    assert!(registry.restore_object_tree(&bytes).is_ok());
}
