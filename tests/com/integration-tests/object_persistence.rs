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

/// Every host-owned restore allocation can fail without publishing a partial root.
#[test]
fn restore_allocations_fail_cleanly_and_allow_same_context_retry() {
    use std::alloc::Layout;
    use std::sync::atomic::AtomicU32;
    use neocogi_com_sdk::{ComRaw, NC_E_OUT_OF_MEMORY, NC_OK};

    let runtime = crate::host_runtime();
    // Measure the actual Vec growth request used for collected child owners.
    let mut capacity = Vec::<std::mem::MaybeUninit<ComPtr<IUnknown>>>::new();
    capacity.try_reserve(1).unwrap();
    let first_capacity = capacity.capacity();
    capacity.resize_with(first_capacity, std::mem::MaybeUninit::uninit);
    capacity.try_reserve(1).unwrap();
    let next_capacity = capacity.capacity();
    let siblings = first_capacity + 1;
    assert!(siblings <= 8, "fixture recipe admits up to eight children");

    let small = record(&"s".repeat(733), vec![]);
    let large = record(&"l".repeat(6001), vec![]);
    let parent = record("parent", (0..siblings).map(|_| record("leaf", vec![])).collect());
    // The private owner-local list test verifies this mirror's allocation shape.
    let list_layout = Layout::new::<(AtomicU32, Vec<ComPtr<IUnknown>>)>();
    let list_ordinal = {
        // Codec controls may share the header layout. On this healthy path the
        // parent's list is the last matching request; counts below verify it.
        let (registry, counts) = registry();
        let bytes = binary::to_vec(&parent).unwrap();
        let probe = allocation_probe::Probe::matching(list_layout, None);
        let root = registry.restore_object_tree(&bytes);
        let report = probe.finish();
        assert_eq!(counts.created.load(Ordering::Relaxed), siblings + 1);
        drop(root.unwrap());
        assert_eq!(counts.live.load(Ordering::Relaxed), 0);
        report.attempts
    };
    for (record, layout, ordinal, expected_created, expected_status) in [
        (&small, Layout::array::<u8>(small.state.len()).unwrap(), 1, 0, NC_E_OUT_OF_MEMORY),
        (&large, Layout::array::<u8>(4096).unwrap(), 1, 0, NC_E_OUT_OF_MEMORY),
        (&large, Layout::array::<u8>(8192).unwrap(), 1, 0, NC_E_OUT_OF_MEMORY),
        (&parent, Layout::new::<SavedObject>(), 1, 0, NC_E_OUT_OF_MEMORY),
        (
            &parent,
            Layout::array::<ComPtr<IUnknown>>(next_capacity).unwrap(),
            1,
            siblings,
            NC_E_OUT_OF_MEMORY,
        ),
        (&parent, list_layout, list_ordinal, siblings, NC_E_OUT_OF_MEMORY),
        // The format's own input copy refusal keeps its codec-error translation.
        (
            &small,
            Layout::array::<u8>(binary::to_vec(&small).unwrap().len()).unwrap(),
            1,
            0,
            NC_E_INVALID_ARG,
        ),
    ] {
        let bytes = binary::to_vec(record).unwrap();
        let counts = Arc::new(Counts::default());
        let factory = Factory::from_value(Factory { counts: counts.clone() })
            .query::<IPersistenceFactoryAbi>()
            .unwrap();
        // SAFETY: The linked host owns a process-lived table.
        let api = unsafe { &*neocogi_com_host::initialize_process().persistence };
        let mut context = core::ptr::null_mut();
        // SAFETY: The local context slot accepts the new exclusive context owner.
        assert_eq!(unsafe { (api.create_context)(&mut context) }, NC_OK);
        // SAFETY: This exact factory is shared-borrowed for registration.
        assert_eq!(unsafe { (api.register_factory)(context, KIND, factory.as_raw()) }, NC_OK);
        let mut out = ComRaw {
            object: core::ptr::dangling_mut(),
            vtable: core::ptr::dangling(),
        };
        let probe = allocation_probe::Probe::matching(layout, Some(ordinal));
        // SAFETY: Context, input and exclusive output live for this synchronous call.
        let status = unsafe { (api.restore_object_tree)(context, bytes.as_ptr(), bytes.len(), Limits::default().into(), &mut out) };
        let report = probe.finish();
        assert_eq!(report.denied, 1, "{layout:?}, request {ordinal}");
        assert_eq!(status, expected_status, "{layout:?}, request {ordinal}");
        assert!(out.is_null());
        assert_eq!(counts.created.load(Ordering::Relaxed), expected_created, "{layout:?}, request {ordinal}");
        assert_eq!(counts.live.load(Ordering::Relaxed), 0);
        if expected_created == 0 {
            assert_eq!(counts.validated.load(Ordering::Relaxed), 0);
        }
        // SAFETY: Retry uses the same live context after restoring allocation.
        assert_eq!(
            unsafe { (api.restore_object_tree)(context, bytes.as_ptr(), bytes.len(), Limits::default().into(), &mut out) },
            NC_OK
        );
        // SAFETY: Successful restoration transfers exactly this root owner.
        let root = unsafe { ComPtr::from_raw_owned(out) }.unwrap();
        assert_eq!(save_object_tree(&runtime, &root).unwrap(), bytes);
        drop(root);
        assert_eq!(counts.live.load(Ordering::Relaxed), 0);
        // SAFETY: Return the unique context after all calls have completed.
        unsafe { (api.destroy_context)(context) };
    }
}
