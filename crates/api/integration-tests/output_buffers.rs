//! One-way output primitives before and after a synchronous raw provider call.
//! Generated-call and independent-provider tests additionally cover the binding layer.
use neocogi_com::*;
use std::mem::MaybeUninit;
#[path = "support/allocation_probe.rs"]
mod allocation_probe;
use allocation_probe::Probe;

#[com_struct(assigned_signature = b"00000000000000000000000000000201")]
#[derive(Debug, PartialEq)]
/// Restricted record proves output acquisition recursively checks raw fields.
struct Flag {
    /// Native bool is a raw byte until the complete output validates.
    enabled: bool,
}

#[com_enum(tag = u8, assigned_signature = b"00000000000000000000000000000202")]
#[derive(Clone, Copy, Debug, PartialEq)]
/// Both the selected tag and its nested restricted payload require validation.
enum Choice {
    /// Empty payload must not be inspected as the other alternative.
    Empty,
    /// Active payload recursively validates its bool byte.
    Value(Flag),
}

/// Produces two booleans through the real raw pointer/capacity/count protocol.
unsafe extern "C" fn flags(pointer: *mut u8, capacity: u64, written: *mut u64) -> NcStatus {
    let operation = || -> Result<(), NcStatus> {
        // SAFETY: fixture caller grants aligned, distinct raw output/count storage.
        let mut output = unsafe { NcOutputWriter::<bool>::from_raw(9, pointer, capacity, &mut *written) }?;
        assert_eq!(output.offset(), 9);
        output.push(true)?;
        output.push(false)?;
        Ok(())
    };
    match operation() { Ok(()) => NC_OK, Err(status) => status }
}

#[test]
fn raw_output_initializes_only_reported_prefix_and_reuses_capacity() {
    let mut buffer = NcOutputBuffer::<bool>::new();
    for round in 0..3 {
        let mut prepared = buffer.output(9, 4).prepare().unwrap();
        let probe = Probe::start(Some(0));
        // SAFETY: prepared storage and its distinct count remain live for this call.
        assert_eq!(unsafe { flags(prepared.as_mut_ptr(), prepared.capacity(), prepared.written_ptr()) }, NC_OK);
        // SAFETY: the provider initialized exactly the reported prefix.
        unsafe { prepared.convert_output() }.unwrap();
        prepared.publish();
        assert_eq!(probe.finish().attempts, 0, "round {round}");
        assert_eq!(buffer.as_slice(), &[true, false]);
        let probe = Probe::start(Some(0));
        drop(buffer.output(0, 4).prepare().unwrap());
        assert_eq!(probe.finish().attempts, 0);
        assert!(buffer.is_empty());
    }
}

#[test]
fn either_preparation_allocation_can_fail_before_any_provider_runs() {
    for ordinal in 1..=2 {
        let mut buffer = NcOutputBuffer::<bool>::new();
        let probe = Probe::start(Some(ordinal));
        let result = buffer.output(0, 4).prepare();
        assert!(matches!(result, Err(NC_E_OUT_OF_MEMORY)));
        drop(result);
        assert_eq!(probe.finish().denied, 1);
        assert!(buffer.is_empty());
    }
}

#[test]
fn direct_numeric_output_has_no_wire_allocation_and_retains_its_native_pointer() {
    let mut buffer = NcOutputBuffer::from_vec(vec![0_u32; 3]);
    let mut prepared = buffer.output(0, 3).prepare().unwrap();
    let pointer = prepared.as_mut_ptr();
    let probe = Probe::start(Some(0));
    let mut written = 0;
    // SAFETY: prepared grants exclusive writable wire slots and a distinct local count.
    let mut output = unsafe { NcOutputWriter::<u32>::from_raw(0, pointer, 3, &mut written) }.unwrap();
    output.extend_from_slice(&[4, 5]).unwrap();
    drop(output);
    unsafe { *prepared.written_ptr() = written; prepared.convert_output().unwrap(); }
    prepared.publish();
    assert_eq!(probe.finish().attempts, 0);
    assert_eq!(buffer.as_slice(), &[4, 5]);
    assert_eq!(buffer.as_slice().as_ptr(), pointer);
}

#[test]
fn bad_later_enum_or_nested_bool_discards_the_entire_result() {
    for malformed_tag in [true, false] {
        let mut buffer = NcOutputBuffer::<Choice>::new();
        let mut prepared = buffer.output(0, 2).prepare().unwrap();
        let good = encode_inline(&Choice::Empty).unwrap();
        let mut bad = encode_inline(&Choice::Value(Flag { enabled: true })).unwrap();
        match malformed_tag {
            true => bad.tag = 99,
            false => bad.payload.case_1.field_0.enabled = 7,
        }
        // SAFETY: valid prepared storage receives two initialized wire records;
        // only restricted bits are malformed, not pointer/lifetime obligations.
        unsafe {
            prepared.as_mut_ptr().write(good);
            prepared.as_mut_ptr().add(1).write(bad);
            *prepared.written_ptr() = 2;
        }
        assert_eq!(unsafe { prepared.convert_output() }, Err(match malformed_tag { true => NC_E_INVALID_TAG, false => NC_E_INVALID_ARG }));
        drop(prepared);
        assert!(buffer.is_empty());
    }
}

#[test]
fn acquired_first_output_is_discarded_when_a_later_output_fails() {
    let mut first = NcOutputBuffer::<bool>::new();
    let mut second = NcOutputBuffer::<bool>::new();
    let mut a = first.output(0, 1).prepare().unwrap();
    let mut b = second.output(0, 1).prepare().unwrap();
    // SAFETY: all slots are live initialized raw bytes; 7 is deliberately invalid.
    unsafe { a.as_mut_ptr().write(1); *a.written_ptr() = 1; b.as_mut_ptr().write(7); *b.written_ptr() = 1; }
    unsafe { a.convert_output() }.unwrap();
    assert_eq!(unsafe { b.convert_output() }, Err(NC_E_INVALID_ARG));
    drop((a, b));
    assert!(first.is_empty() && second.is_empty());
}

#[test]
fn count_shape_capacity_and_budget_fail_without_exposing_output() {
    // Identity outputs still reject unrepresentable regions before trying allocation.
    let mut numeric = NcOutputBuffer::<u64>::new();
    assert_eq!(numeric.output(0, usize::MAX).prepare().err().unwrap(), NC_E_OUT_OF_RANGE);
    let mut written = 3;
    // SAFETY: empty storage permits null; nonempty null is rejected before access.
    let empty = unsafe { NcOutputWriter::<bool>::from_raw(0, std::ptr::null_mut(), 0, &mut written) }.unwrap();
    assert_eq!(empty.capacity(), 0);
    drop(empty);
    assert_eq!(written, 0);
    assert!(matches!(unsafe { NcOutputWriter::<bool>::from_raw(0, std::ptr::null_mut(), 1, &mut written) }, Err(NC_E_POINTER)));
    assert!(matches!(unsafe { NcOutputWriter::<bool>::from_raw(0, std::ptr::null_mut(), u64::MAX, &mut written) }, Err(NC_E_OUT_OF_RANGE)));
    let mut slots = [MaybeUninit::<u32>::uninit(); 1];
    let mut writer = unsafe { NcOutputWriter::<u32>::from_raw(0, slots.as_mut_ptr().cast(), 1, &mut written) }.unwrap();
    assert_eq!(writer.extend_from_slice(&[1, 2]), Err(NC_E_OUT_OF_RANGE));
    assert_eq!(writer.written(), 0);
    writer.push(4).unwrap();
    assert_eq!(writer.push(5), Err(NC_E_OUT_OF_RANGE));
    drop(writer);
    let mut buffer = NcOutputBuffer::<bool>::new();
    let mut prepared = buffer.output(0, 1).prepare().unwrap();
    unsafe { *prepared.written_ptr() = 2; }
    assert_eq!(unsafe { prepared.convert_output() }, Err(NC_E_OUT_OF_RANGE));
    drop(prepared);
    assert!(buffer.is_empty());
    let mut prepared = buffer.output(0, 0).prepare().unwrap();
    unsafe { prepared.convert_output() }.unwrap();
    prepared.publish();
    assert!(buffer.is_empty());
}
