//! Shared real-ABI regressions, compiled against both macro and IDL declarations.
use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering::SeqCst},
};
use allocation_probe::Probe;

#[com_object(IItem, ownership = exclusive)]
/// Exclusive input with independently observable destruction.
struct Item {
    /// Shared only with the test, never passed over the ABI.
    drops: Arc<AtomicUsize>,
}
impl IItem for Item {
    /// Weak observers can verify rejected owners remain callable.
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(42)
    }
}
impl Drop for Item {
    /// Count concrete destruction, independently of retained weak shells.
    fn drop(&mut self) {
        self.drops.fetch_add(1, SeqCst);
    }
}

#[com_object(IComplete, ownership = exclusive)]
/// Stores accepted owners in ordinary fields; generated code cannot reclaim them.
struct Receiver {
    /// Empty until the implementation commits both owning inputs.
    children: [Option<ComBox<IItemAbi>>; 2],
    /// Counts actual dispatch independently of preparation and receiver queries.
    calls: Arc<AtomicUsize>,
}
impl IComplete for Receiver {
    /// A single commit mutates two converted outputs and stores both input owners.
    fn complete(
        &mut self,
        first: ComBox<IItemAbi>,
        second: ComBox<IItemAbi>,
        mut choices: NcCopyToBufferOffset<'_, Choice>,
        mut flags: NcCopyToBufferOffset<'_, bool>,
        reject: bool,
        deny_after_entry: bool,
    ) -> Result<Choice, (NcStatus, ComBox<IItemAbi>, ComBox<IItemAbi>)> {
        if deny_after_entry {
            allocation_probe::deny_remaining();
        }
        self.calls.fetch_add(1, SeqCst);
        choices.as_mut_slice().fill(Choice::Filled(Details { flags: [true, false] }));
        flags.as_mut_slice().fill(true);
        match reject {
            true => Err((NC_E_INVALID_ARG, first, second)),
            false => {
                self.children = [Some(first), Some(second)];
                Ok(Choice::Filled(Details { flags: [false, true] }))
            }
        }
    }
    /// Direct writable integers require neither staging nor output allocation.
    fn direct(&mut self, first: ComBox<IItemAbi>, mut values: NcCopyToBufferOffset<'_, u64>) -> Result<u32, (NcStatus, ComBox<IItemAbi>)> {
        self.calls.fetch_add(1, SeqCst);
        values.as_mut_slice().fill(9);
        self.children[0] = Some(first);
        Ok(42)
    }
    /// Even zero-sized arrays must obey the cumulative logical value limit.
    fn excessive(&mut self, first: ComBox<IItemAbi>) -> Result<Excessive, (NcStatus, ComBox<IItemAbi>)> {
        self.calls.fetch_add(1, SeqCst);
        self.children[0] = Some(first);
        Ok([[]; MAX_VALUE_COUNT])
    }
}

/// Constructs a provider plus observations before enabling allocation injection.
fn receiver() -> (ComBox<ICompleteAbi>, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let owner = Receiver::into_box(Receiver {
        children: [None, None],
        calls: calls.clone(),
    })
    .query()
    .unwrap();
    (owner, calls)
}
/// Constructs two independently identifiable owners sharing only a drop counter.
fn items() -> ([ComBox<IItemAbi>; 2], Arc<AtomicUsize>) {
    let drops = Arc::new(AtomicUsize::new(0));
    let make = || Item::into_box(Item { drops: drops.clone() }).query().unwrap();
    ([make(), make()], drops)
}

#[test]
fn every_generated_preparation_failure_returns_both_original_owners() {
    // Four allocations per side: two vectors for each of two converted buffers.
    for ordinal in 1..=8 {
        let (mut receiver, calls) = receiver();
        let ([first, second], drops) = items();
        let identities = [first.as_raw().object, second.as_raw().object];
        let mut choices = [Choice::Idle; 2];
        let mut flags = [false; 2];
        let probe = Probe::start(Some(ordinal));
        let result = receiver.complete(
            first,
            second,
            NcCopyToBufferOffset::new(0, &mut choices),
            NcCopyToBufferOffset::new(0, &mut flags),
            false,
            false,
        );
        let report = probe.finish();
        let (status, first, second) = result.unwrap_err();
        assert_eq!(status, NC_E_OUT_OF_MEMORY, "allocation {ordinal}");
        assert_eq!(report.denied, 1);
        assert_eq!([first.as_raw().object, second.as_raw().object], identities);
        assert_eq!(calls.load(SeqCst), 0);
        assert_eq!(drops.load(SeqCst), 0);
        assert_eq!(first.value(), Ok(42));
        assert_eq!(second.value(), Ok(42));
        assert_eq!(choices, [Choice::Idle; 2]);
        assert_eq!(flags, [false; 2]);
        drop((first, second));
        assert_eq!(drops.load(SeqCst), 2);
    }
}

#[test]
fn generated_commit_completes_with_all_further_allocation_denied() {
    let (mut receiver, calls) = receiver();
    let ([first, second], drops) = items();
    let weak = first.downgrade();
    let mut choices = [Choice::Idle; 2];
    let mut flags = [false; 2];
    let probe = Probe::start(None);
    let result = receiver.complete(
        first,
        second,
        NcCopyToBufferOffset::new(0, &mut choices),
        NcCopyToBufferOffset::new(0, &mut flags),
        false,
        true,
    );
    let report = probe.finish();
    assert_eq!(result.unwrap(), Choice::Filled(Details { flags: [false, true] }));
    assert_eq!(report.attempts, 8);
    assert_eq!(report.denied, 0);
    assert_eq!(choices, [Choice::Filled(Details { flags: [true, false] }); 2]);
    assert_eq!(flags, [true; 2]);
    assert_eq!(calls.load(SeqCst), 1);
    assert_eq!(drops.load(SeqCst), 0);
    assert_eq!(weak.value(), Ok(42));
    drop(receiver);
    assert_eq!(drops.load(SeqCst), 2);
    assert_eq!(weak.value(), Err(NC_E_CLOSED));
}

#[test]
fn implementation_rejection_restores_owners_without_publishing_outputs() {
    let (mut receiver, calls) = receiver();
    let ([first, second], drops) = items();
    let identities = [first.as_raw().object, second.as_raw().object];
    let mut choices = [Choice::Idle];
    let mut flags = [false];
    let probe = Probe::start(None);
    let result = receiver.complete(
        first,
        second,
        NcCopyToBufferOffset::new(0, &mut choices),
        NcCopyToBufferOffset::new(0, &mut flags),
        true,
        true,
    );
    let report = probe.finish();
    let (status, first, second) = result.unwrap_err();
    assert_eq!(status, NC_E_INVALID_ARG);
    assert_eq!([first.as_raw().object, second.as_raw().object], identities);
    assert_eq!(report.denied, 0);
    assert_eq!(calls.load(SeqCst), 1);
    assert_eq!(choices, [Choice::Idle]);
    assert_eq!(flags, [false]);
    drop((first, second));
    assert_eq!(drops.load(SeqCst), 2);
}

#[test]
fn maximum_variant_cost_is_admitted_before_dispatch_on_both_sides() {
    // Choice::Filled costs five values: enum, record, array, and two bools.
    // The current Idle variant is cheap; the possible output determines admission.
    let count = MAX_VALUE_COUNT / 5 + 1;
    let (mut receiver, calls) = receiver();
    let ([first, second], drops) = items();
    let identities = [first.as_raw().object, second.as_raw().object];
    let mut choices = vec![Choice::Idle; count];
    let mut flags = [];
    let probe = Probe::start(Some(0));
    let result = receiver.complete(
        first,
        second,
        NcCopyToBufferOffset::new(0, &mut choices),
        NcCopyToBufferOffset::new(0, &mut flags),
        false,
        false,
    );
    let report = probe.finish();
    let (status, first, second) = result.unwrap_err();
    assert_eq!(status, NC_E_OUT_OF_RANGE);
    assert_eq!(report.attempts, 0);
    assert_eq!([first.as_raw().object, second.as_raw().object], identities);

    // Bypass the safe caller and prove the provider enforces the same contract.
    let mut wires = vec![encode_inline(&Choice::Idle).unwrap(); count];
    let mut first_raw = first.into_raw();
    let mut second_raw = second.into_raw();
    let raw = receiver.as_raw();
    let mut output = std::mem::MaybeUninit::uninit();
    let probe = Probe::start(Some(0));
    // SAFETY: owners remain in their unique transfer slots, buffers are initialized
    // and exclusively writable, and output storage has the generated wire type.
    let status = unsafe {
        ((*raw.vtable).complete)(
            raw.object,
            &mut first_raw,
            &mut second_raw,
            0,
            count as u64,
            wires.as_mut_ptr(),
            0,
            0,
            std::ptr::null_mut(),
            0,
            0,
            output.as_mut_ptr(),
        )
    };
    let report = probe.finish();
    assert_eq!(status, NC_E_OUT_OF_RANGE);
    assert_eq!(report.attempts, 0);
    assert_eq!([first_raw.object, second_raw.object], identities);
    assert_eq!(calls.load(SeqCst), 0);
    // SAFETY: rejection left both original ownership tokens in their caller slots.
    unsafe {
        drop((
            ComBox::<IItemAbi>::from_raw_owned(first_raw).unwrap(),
            ComBox::<IItemAbi>::from_raw_owned(second_raw).unwrap(),
        ));
    }
    assert_eq!(drops.load(SeqCst), 2);
}

#[test]
fn raw_provider_preparation_failures_never_adopt_an_owner() {
    for ordinal in 1..=4 {
        let (receiver, calls) = receiver();
        let ([first, second], drops) = items();
        let mut first = first.into_raw();
        let mut second = second.into_raw();
        let identities = [first.object, second.object];
        let mut choice = encode_inline(&Choice::Idle).unwrap();
        let mut flag = 0_u8;
        let raw = receiver.as_raw();
        let mut output = std::mem::MaybeUninit::uninit();
        let probe = Probe::start(Some(ordinal));
        // SAFETY: independent live transfer slots, initialized single-element
        // wires, and distinct output storage satisfy the generated ABI contract.
        let status = unsafe {
            ((*raw.vtable).complete)(
                raw.object,
                &mut first,
                &mut second,
                0,
                1,
                &mut choice,
                0,
                1,
                &mut flag,
                0,
                0,
                output.as_mut_ptr(),
            )
        };
        let report = probe.finish();
        assert_eq!(status, NC_E_OUT_OF_MEMORY);
        assert_eq!(report.denied, 1);
        assert_eq!([first.object, second.object], identities);
        assert_eq!(calls.load(SeqCst), 0);
        assert_eq!(flag, 0);
        assert_eq!(unsafe { decode_inline::<Choice>(choice) }.unwrap(), Choice::Idle);
        // SAFETY: the failed provider left both tokens unconsumed.
        unsafe {
            drop((
                ComBox::<IItemAbi>::from_raw_owned(first).unwrap(),
                ComBox::<IItemAbi>::from_raw_owned(second).unwrap(),
            ));
        }
        assert_eq!(drops.load(SeqCst), 2);
    }
}

#[test]
fn copied_return_bound_rejects_before_acceptance_even_if_actual_result_is_small() {
    let (mut receiver, calls) = receiver();
    let ([first, second], drops) = items();
    let identity = first.as_raw().object;
    let probe = Probe::start(Some(0));
    let result = receiver.excessive(first);
    let report = probe.finish();
    let (status, first) = result.unwrap_err();
    assert_eq!(status, NC_E_OUT_OF_RANGE);
    assert_eq!(report.attempts, 0);
    assert_eq!(first.as_raw().object, identity);
    let mut slot = first.into_raw();
    let raw = receiver.as_raw();
    let mut output = std::mem::MaybeUninit::uninit();
    let probe = Probe::start(Some(0));
    // SAFETY: the original token occupies one unique live slot; the copied
    // zero-sized result still has a valid, aligned generated output address.
    let status = unsafe { ((*raw.vtable).excessive)(raw.object, &mut slot, output.as_mut_ptr()) };
    let report = probe.finish();
    assert_eq!(status, NC_E_OUT_OF_RANGE);
    assert_eq!(slot.object, identity);
    assert_eq!(report.attempts, 0);
    assert_eq!(calls.load(SeqCst), 0);
    // SAFETY: rejection left the original owner unconsumed in its caller slot.
    let first = unsafe { ComBox::<IItemAbi>::from_raw_owned(slot) }.unwrap();
    drop((first, second));
    assert_eq!(drops.load(SeqCst), 2);
}

/// A foreign provider can violate content validity even after resource admission.
/// This test consumes both owners correctly, then writes an invalid raw bool byte.
unsafe extern "C" fn invalid_committed_output(
    _this: *mut std::ffi::c_void,
    first: *mut ComRaw<IItemAbi>,
    second: *mut ComRaw<IItemAbi>,
    _choices_offset: u64,
    choices_len: u64,
    choices: *mut <Choice as ComValue>::Wire,
    _flags_offset: u64,
    flags_len: u64,
    flags: *mut u8,
    _reject: u8,
    _deny: u8,
    result: *mut <Choice as ComValue>::Wire,
) -> NcStatus {
    assert_eq!((choices_len, flags_len), (1, 1));
    // SAFETY: the fixture supplies unique live tokens in distinct writable slots.
    // Clearing each slot implements successful consumption; releasing the adopted
    // owner is this deliberately adversarial provider's chosen implementation.
    unsafe {
        drop(ComBox::<IItemAbi>::from_raw_owned(std::mem::replace(&mut *first, ComRaw::null())).unwrap());
        drop(ComBox::<IItemAbi>::from_raw_owned(std::mem::replace(&mut *second, ComRaw::null())).unwrap());
        *choices = encode_inline(&Choice::Filled(Details { flags: [true; 2] })).unwrap();
        *flags = 7;
        *result = encode_inline(&Choice::Idle).unwrap();
    }
    NC_OK
}

#[test]
fn invalid_committed_output_is_a_contract_violation_not_an_owner_rejection() {
    let (receiver, _) = receiver();
    let ([first, second], drops) = items();
    let mut raw = receiver.as_raw();
    // SAFETY: vtable entries are copyable static metadata and C function pointers.
    let mut table = unsafe { std::ptr::read(raw.vtable) };
    table.complete = invalid_committed_output;
    raw.vtable = &table;
    // SAFETY: receiver and substituted table outlive this borrowed test client.
    let mut client = unsafe { ComRef::<ICompleteAbi>::from_raw_borrowed(raw) }.unwrap();
    let mut choices = [Choice::Idle];
    let mut flags = [false];
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.complete(
            first,
            second,
            NcCopyToBufferOffset::new(0, &mut choices),
            NcCopyToBufferOffset::new(0, &mut flags),
            false,
            false,
        )
    }));
    assert!(failure.is_err(), "a consumed owner cannot be reconstructed on invalid successful output");
    assert_eq!(drops.load(SeqCst), 2);
    assert_eq!(choices, [Choice::Idle]);
    assert_eq!(flags, [false]);
}

#[test]
fn generated_direct_buffers_and_empty_converted_buffers_need_no_allocation() {
    let (mut receiver, calls) = receiver();
    let ([first, second], drops) = items();
    let mut values = [0; 3];
    let probe = Probe::start(Some(0));
    let result = receiver.direct(first, NcCopyToBufferOffset::new(0, &mut values));
    let report = probe.finish();
    assert_eq!(result.unwrap(), 42);
    assert_eq!(report.attempts, 0);
    assert_eq!(values, [9; 3]);
    assert_eq!(calls.load(SeqCst), 1);
    drop((receiver, second));
    assert_eq!(drops.load(SeqCst), 2);

    let (mut receiver, _) = self::receiver();
    let ([first, second], drops) = items();
    let mut choices = [];
    let mut flags = [];
    let probe = Probe::start(Some(0));
    let result = receiver.complete(
        first,
        second,
        NcCopyToBufferOffset::new(0, &mut choices),
        NcCopyToBufferOffset::new(0, &mut flags),
        false,
        true,
    );
    let report = probe.finish();
    assert!(result.is_ok());
    assert_eq!(report.attempts, 0);
    drop(receiver);
    assert_eq!(drops.load(SeqCst), 2);
}
