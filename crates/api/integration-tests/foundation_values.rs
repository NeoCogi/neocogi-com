//! Shared resource accounting and admitted output exercise real generated calls.
#![recursion_limit = "512"]
use neocogi_com::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
#[path = "support/allocation_probe.rs"]
mod allocation_probe;

#[com_value(assigned_signature = b"00000000000000000000000000771001")]
#[derive(Debug, PartialEq)]
/// Dynamic sibling sequences share the enclosing record's actual-value budget.
pub struct Payload {
    /// Absent/present alternatives have different logical costs.
    pub batches: Vec<Vec<Option<u8>>>,
}

#[com_interface(auto_guid = "nc.tests.foundation.Receiver:v1", assigned_signature = b"00000000000000000000000000771002")]
/// Receiver explicitly acquires its borrowed argument through the real ABI.
trait IReceiver {
    /// Returns the acquired element count so both producer and receiver execute.
    fn retain(&mut self, value: NcValueRef<'_, Payload>) -> Result<u32, NcStatus>;
}
#[com_object(IReceiver)]
/// Entry counting distinguishes producer rejection from provider acquisition.
struct Receiver {
    /// Shared observation only; the implementation itself remains exclusively mutable.
    entries: Arc<AtomicUsize>,
}
impl IReceiver for Receiver {
    fn retain(&mut self, value: NcValueRef<'_, Payload>) -> Result<u32, NcStatus> {
        self.entries.fetch_add(1, Ordering::SeqCst);
        let owned = value.to_owned()?;
        Ok(owned.batches.iter().map(Vec::len).sum::<usize>() as u32)
    }
}

/// Every sibling permutation must consume exactly the same selected nodes.
#[test]
fn shared_nested_sequences_charge_actual_count_and_depth() {
    for value in [
        vec![vec![None, None], vec![Some(7), Some(7), Some(7)]],
        vec![vec![Some(7), Some(7), Some(7)], vec![None, None]],
    ] {
        // Outer Vec + two Vecs + five options + three payloads = eleven.
        let (wire, _storage) = value.encode_value(&mut ValueBudget::new(MAX_VALUE_BYTES, 11, 4)).unwrap();
        // SAFETY: checked producer and retained source/storage own every wire region.
        let decoded = unsafe { Vec::<Vec<Option<u8>>>::decode_value(wire, &mut ValueBudget::new(MAX_VALUE_BYTES, 11, 4)) }.unwrap();
        assert_eq!(decoded, value);
        for (count, depth) in [(10, 4), (11, 3)] {
            assert!(matches!(
                value.encode_value(&mut ValueBudget::new(MAX_VALUE_BYTES, count, depth)),
                Err(NC_E_OUT_OF_RANGE)
            ));
            // SAFETY: only resource bounds change; the same valid backing stays live.
            assert!(matches!(
                unsafe { Vec::<Vec<Option<u8>>>::decode_value(wire, &mut ValueBudget::new(MAX_VALUE_BYTES, count, depth)) },
                Err(NC_E_OUT_OF_RANGE)
            ));
        }
    }
}

/// Selected record fields and later dynamic fields share one ledger as well.
#[test]
fn mixed_record_and_sequence_permutations_retain_exact_boundaries() {
    for batches in [
        vec![vec![None, None], vec![Some(7), Some(7), Some(7)]],
        vec![vec![Some(7), Some(7), Some(7)], vec![None, None]],
    ] {
        let value = Payload { batches };
        let (wire, _storage) = value.encode_value(&mut ValueBudget::new(MAX_VALUE_BYTES, 12, 5)).unwrap();
        // SAFETY: the concrete producer/guard retain all active option payloads.
        assert_eq!(
            unsafe { Payload::decode_value(wire, &mut ValueBudget::new(MAX_VALUE_BYTES, 12, 5)) }.unwrap(),
            value
        );
        assert!(matches!(
            value.encode_value(&mut ValueBudget::new(MAX_VALUE_BYTES, 11, 5)),
            Err(NC_E_OUT_OF_RANGE)
        ));
    }
}

/// The default-limit example crosses client, thunk and explicit owned acquisition.
#[test]
fn generated_calls_accept_fitting_values_in_both_sibling_orders() {
    let entries = Arc::new(AtomicUsize::new(0));
    let mut receiver = Receiver::from_value(Receiver { entries: entries.clone() }).query::<IReceiverAbi>().unwrap();
    for reverse in [false, true] {
        let mut value = Payload {
            batches: vec![vec![None; 100_000], vec![Some(7); 400_000]],
        };
        if reverse {
            value.batches.reverse();
        }
        assert_eq!(receiver.retain(NcValueRef::new(&value).unwrap()), Ok(500_000));
        let (wire, _storage) = value.encode_value(&mut ValueBudget::new(MAX_VALUE_BYTES, 900_004, 5)).unwrap();
        // SAFETY: independently initialized raw input shares the retained producer backing.
        assert_eq!(receiver.retain(unsafe { NcValueRef::from_raw(&wire) }.unwrap()), Ok(500_000));
    }
    assert_eq!(entries.load(Ordering::SeqCst), 4);
}

/// Identity Vec producers need no wire allocation; retention still acquires ownership.
#[test]
fn identity_vec_borrows_its_source_without_allocating() {
    let source = vec![13_u32; 65_536];
    let probe = allocation_probe::Probe::start(Some(0));
    let guard = NcValueRef::new(&source);
    let report = probe.finish();
    let guard = guard.unwrap();
    assert_eq!(report.attempts, 0);
    assert_eq!(report.denied, 0);
    let (wire, _storage) = source.encode_value(&mut ValueBudget::default()).unwrap();
    assert_eq!(wire.pointer, source.as_ptr());
    let owned = guard.to_owned().unwrap();
    assert_eq!(owned, source);
    assert_ne!(owned.as_ptr(), source.as_ptr());
    let empty: Vec<u32> = Vec::new();
    assert!(NcValueRef::new(&empty).unwrap().to_owned().unwrap().is_empty());
    // Direct storage does not bypass the enclosing value-tree limits.
    assert!(matches!(
        source.encode_value(&mut ValueBudget::new(MAX_VALUE_BYTES, source.len(), 2)),
        Err(NC_E_OUT_OF_RANGE)
    ));
}

/// Restricted native values still receive converted backing and content validation.
#[test]
fn restricted_vec_retains_conversion_and_rejects_bad_wire_bits() {
    let source = vec![true, false];
    let (wire, _storage) = source.encode_value(&mut ValueBudget::default()).unwrap();
    assert_ne!(wire.pointer, source.as_ptr().cast::<u8>());
    let malformed = [1_u8, 2];
    let wire = ValueSlice { pointer: malformed.as_ptr(), len: 2 };
    // SAFETY: both initialized raw bytes are readable; the second is an invalid bool.
    assert!(matches!(
        unsafe { Vec::<bool>::decode_value(wire, &mut ValueBudget::default()) },
        Err(NC_E_INVALID_ARG)
    ));
}

/// Eight single-element arrays add eight logical levels without growing storage.
type Array8<T> = [[[[[[[[T; 1]; 1]; 1]; 1]; 1]; 1]; 1]; 1];
/// A one-byte identity value with logical depth65 exceeds ordinary conversion depth.
type Deep = Array8<Array8<Array8<Array8<Array8<Array8<Array8<Array8<u8>>>>>>>>;
/// Constructs eight array levels in ordinary initialized Rust storage.
fn array8<T>(value: T) -> Array8<T> {
    [[[[[[[[value]]]]]]]]
}
/// Constructs the deep identity value without an enormous array or stack allocation.
fn deep() -> Deep {
    array8(array8(array8(array8(array8(array8(array8(array8(7))))))))
}

#[com_interface(auto_guid = "nc.tests.foundation.Item:v1", assigned_signature = b"00000000000000000000000000771003")]
/// Concrete transferred owner used to exercise consuming output admission.
trait IItem {
    /// Confirms the same returned owner remains callable.
    fn value(&self) -> Result<u32, NcStatus>;
}
#[com_object(interfaces(IItem), ownership = exclusive)]
/// Stateless exclusive transfer fixture.
struct Item;
impl IItem for Item {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(73)
    }
}
#[com_interface(auto_guid = "nc.tests.foundation.Output:v1", assigned_signature = b"00000000000000000000000000771004")]
/// Both ordinary and consuming methods must honor the same identity admission.
trait IOutput {
    /// Produces a deep identity element through either writer entrypoint.
    fn ordinary(&self, bulk: bool, output: NcOutputWriter<'_, Deep>) -> Result<(), NcStatus>;
    /// Returns the exact owner after successful already-admitted completion.
    fn consuming(&self, owner: ComBox<IItemAbi>, output: NcOutputWriter<'_, Deep>) -> Result<ComBox<IItemAbi>, (NcStatus, ComBox<IItemAbi>)>;
}
#[com_object(IOutput)]
/// Stateless provider deliberately uses individual writes on the consuming path.
struct Output;
impl IOutput for Output {
    fn ordinary(&self, bulk: bool, mut output: NcOutputWriter<'_, Deep>) -> Result<(), NcStatus> {
        match bulk {
            true => output.extend_from_slice(&[deep()]),
            false => output.push(deep()),
        }
    }
    fn consuming(&self, owner: ComBox<IItemAbi>, mut output: NcOutputWriter<'_, Deep>) -> Result<ComBox<IItemAbi>, (NcStatus, ComBox<IItemAbi>)> {
        match output.push(deep()) {
            Ok(()) => Ok(owner),
            Err(status) => Err((status, owner)),
        }
    }
}

/// Safe generated calls and raw provider admission agree, with stack-only output.
#[test]
fn admitted_identity_push_and_bulk_complete_through_generated_calls() {
    assert!(Deep::BORROW_WIRE);
    assert_eq!(check_buffer_output::<Deep>(1), Ok(()));
    let provider = Output::from_value(Output).query::<IOutputAbi>().unwrap();
    for bulk in [false, true] {
        let mut slots = [core::mem::MaybeUninit::<Deep>::uninit(); 1];
        let mut count = 0;
        provider.ordinary(bulk, NcOutput::from_slots(0, &mut slots, &mut count).unwrap()).unwrap();
        assert_eq!(count, 1);
        // SAFETY: successful generated publication initialized precisely one element.
        assert_eq!(unsafe { slots[0].assume_init() }, deep());
    }
    let owner = Item::into_box(Item).query::<IItemAbi>().unwrap();
    let identity = owner.as_raw().object;
    let mut output = NcOutputBuffer::new();
    let owner = provider.consuming(owner, output.output(0, 1)).unwrap();
    assert_eq!(owner.as_raw().object, identity);
    assert_eq!(owner.value(), Ok(73));
    assert_eq!(output.as_slice(), &[deep()]);
    let mut raw_output = core::mem::MaybeUninit::<<Deep as ComValue>::Wire>::uninit();
    let mut written = 99;
    let raw = provider.as_raw();
    // SAFETY: a live receiver, one aligned writable wire slot, and separate count.
    assert_eq!(
        unsafe { ((*raw.vtable).ordinary)(raw.object, 0, 0, 1, raw_output.as_mut_ptr(), &mut written) },
        NC_OK
    );
    assert_eq!(written, 1);
    // Ordinary inline conversion remains budgeted: only admitted identity buffers
    // get this exemption, not arbitrary deep copied results.
    assert!(matches!(encode_inline(&deep()), Err(NC_E_OUT_OF_RANGE)));
}
