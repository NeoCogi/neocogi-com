//! Buffer completion reuses storage reserved before any implementation can commit.
#![recursion_limit = "256"]
use neocogi_com::*;
#[path = "support/allocation_probe.rs"]
mod allocation_probe;

extern crate self as fixture_output_completion;
neocogi_com_test_contracts::output_completion!();

/// A copied zero-sized return can still exceed the logical traversal limit.
type Excessive = [[bool; 0]; MAX_VALUE_COUNT];

#[path = "support/output_completion_fixture.rs"]
mod generated_calls;

#[com_object(IOrdinaryOutputs)]
/// Deliberately produces valid native outputs exceeding ordinary conversion limits.
struct OrdinaryOutputs;
impl IOrdinaryOutputs for OrdinaryOutputs {
    /// Each current Idle is cheap, but Filled may exceed the combined output count.
    fn buffers(&mut self, mut flags: NcOutputWriter<'_, bool>, mut choices: NcOutputWriter<'_, Choice>) -> Result<(), NcStatus> {
        for _ in 0..flags.capacity() {
            flags.push(true)?;
        }
        for _ in 0..choices.capacity() {
            choices.push(Choice::Filled(Details { flags: [true; 2] }))?;
        }
        Ok(())
    }
    /// Returns zero physical bytes with too many logical array values.
    fn result(&mut self, mut flags: NcOutputWriter<'_, bool>) -> Result<Excessive, NcStatus> {
        for _ in 0..flags.capacity() {
            flags.push(true)?;
        }
        Ok([[]; MAX_VALUE_COUNT])
    }
}

#[test]
fn provider_validates_all_converted_outputs_before_publishing_any() {
    let provider = OrdinaryOutputs::from_value(OrdinaryOutputs).query::<IOrdinaryOutputsAbi>().unwrap();
    let raw = provider.as_raw();
    let mut flags = [0_u8];
    let mut flags_written = 0;
    let mut choices_written = 0;
    let mut choices = vec![encode_inline(&Choice::Idle).unwrap(); MAX_VALUE_COUNT / 5 + 1];
    // SAFETY: live initialized writable wire buffers and a valid provider object.
    let status = unsafe {
        ((*raw.vtable).buffers)(
            raw.object,
            0,
            1,
            flags.as_mut_ptr(),
            &mut flags_written,
            0,
            choices.len() as u64,
            choices.as_mut_ptr(),
            &mut choices_written,
        )
    };
    assert_eq!(status, NC_E_OUT_OF_RANGE);
    assert_eq!(flags, [0]);
    assert!(choices.iter().all(|wire| unsafe { decode_inline::<Choice>(*wire) }.unwrap() == Choice::Idle));
    let mut out = std::mem::MaybeUninit::uninit();
    // SAFETY: output storage matches the copied wire, including its zero size.
    let status = unsafe { ((*raw.vtable).result)(raw.object, 0, 1, flags.as_mut_ptr(), &mut flags_written, out.as_mut_ptr()) };
    assert_eq!(status, NC_E_OUT_OF_RANGE);
    assert_eq!(flags_written, 0);
}

/// Simulates a foreign provider violating the boolean output representation.
/// The raw wire remains initialized u8 storage; no invalid Rust bool is created.
unsafe extern "C" fn invalid_later_buffer(
    _this: *mut std::ffi::c_void,
    _flags_offset: u64,
    flags_len: u64,
    flags: *mut u8,
    flags_written: *mut u64,
    _choices_offset: u64,
    choices_len: u64,
    choices: *mut <Choice as ComValue>::Wire,
    choices_written: *mut u64,
) -> NcStatus {
    assert_eq!((flags_len, choices_len), (1, 1));
    let mut malformed = encode_inline(&Choice::Filled(Details { flags: [true; 2] })).unwrap();
    // The generated wire exposes the raw enum tag independently of its payload.
    malformed.tag = 99;
    // SAFETY: this test's caller supplies two distinct writable single-element buffers.
    unsafe {
        *flags = 1;
        *choices = malformed;
        *flags_written = 1;
        *choices_written = 1;
    }
    NC_OK
}

#[test]
fn caller_validates_later_output_before_publishing_earlier_buffer() {
    let provider = OrdinaryOutputs::from_value(OrdinaryOutputs).query::<IOrdinaryOutputsAbi>().unwrap();
    let mut raw = provider.as_raw();
    // SAFETY: the generated vtable contains only function pointers/static metadata;
    // duplicating it locally does not transfer ownership or change provider storage.
    let mut table = unsafe { std::ptr::read(raw.vtable) };
    table.buffers = invalid_later_buffer;
    raw.vtable = &table;
    // SAFETY: the provider and substituted C-shaped table remain live throughout
    // this scoped borrow. Only the output-writing slot is deliberately malformed.
    let mut client = unsafe { ComRef::<IOrdinaryOutputsAbi>::from_raw_borrowed(raw) }.unwrap();
    let mut flags = generated_calls::seeded_output(true);
    let mut choices = generated_calls::seeded_output(Choice::Idle);
    let status = client.buffers(flags.output(0, 1), choices.output(0, 1));
    assert_eq!(status, Err(NC_E_INVALID_TAG));
    assert!(flags.is_empty());
    assert!(choices.is_empty());
}

/// Ordinary callers have the same freshness rule even when an earlier output never prepares.
#[test]
fn ordinary_admission_rejection_discards_seeded_outputs_in_either_order() {
    for reject_first in [true, false] {
        let mut provider = OrdinaryOutputs::from_value(OrdinaryOutputs).query::<IOrdinaryOutputsAbi>().unwrap();
        let mut flags = generated_calls::seeded_output(true);
        let mut choices = generated_calls::seeded_output(Choice::Idle);
        let result = provider.buffers(
            flags.output(0, if reject_first { usize::MAX } else { 1 }),
            choices.output(0, if reject_first { 1 } else { usize::MAX }),
        );
        assert_eq!(result, Err(NC_E_OUT_OF_RANGE));
        assert!(flags.is_empty() && choices.is_empty());
    }
}

/// A successful empty replacement publishes an empty group without retaining old values.
#[test]
fn ordinary_empty_success_replaces_seeded_outputs() {
    let mut provider = OrdinaryOutputs::from_value(OrdinaryOutputs).query::<IOrdinaryOutputsAbi>().unwrap();
    let mut flags = generated_calls::seeded_output(true);
    let mut choices = generated_calls::seeded_output(Choice::Idle);
    assert_eq!(provider.buffers(flags.output(0, 0), choices.output(0, 0)), Ok(()));
    assert!(flags.is_empty() && choices.is_empty());
}

#[test]
fn consuming_output_admission_checks_bytes_count_depth_and_exact_boundaries() {
    // bool is restricted, whereas u64 remains a direct in-place buffer.
    assert_eq!(check_buffer_output::<bool>(MAX_VALUE_COUNT as u64), Ok(()));
    assert_eq!(check_buffer_output::<bool>(MAX_VALUE_COUNT as u64 + 1), Err(NC_E_OUT_OF_RANGE));
    assert_eq!(check_buffer_output::<u64>(u64::MAX), Err(NC_E_OUT_OF_RANGE));
    // Identity output has no conversion budget, but its region must still fit
    // pointer arithmetic before allocation or a raw provider call.
    assert_eq!(check_buffer_output::<Choice>((MAX_VALUE_COUNT / 5) as u64), Ok(()));
    assert_eq!(check_buffer_output::<Choice>((MAX_VALUE_COUNT / 5 + 1) as u64), Err(NC_E_OUT_OF_RANGE));
    assert_eq!(check_buffer_output::<Choice>(u64::MAX), Err(NC_E_OUT_OF_RANGE));
    assert_eq!(check_buffer_output::<[u64; 2]>((MAX_VALUE_BYTES / 16 + 1) as u64), Ok(()));
    // This test codec deliberately pads wire and native storage differently, as
    // an independent provider can. Padding adds bytes, not logical bool values.
    #[derive(Clone, Copy)]
    #[repr(align(64))]
    struct Padded(bool);
    // SAFETY: initialized raw bytes are checked as a boolean plus inert padding.
    // The one-value maximum and both prechecked converters allocate nothing.
    unsafe impl ComValue for Padded {
        const FIXED_COST: Option<ValueCost> = Some(ValueCost::SCALAR);
        type Wire = [u8; 128];
        type Storage = ();
        fn encode_value(&self, budget: &mut ValueBudget) -> Result<(Self::Wire, ()), NcStatus> {
            budget.value(|_| unsafe { self.encode_prechecked() })
        }
        unsafe fn decode_value(wire: Self::Wire, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
            budget.value(|_| unsafe { Self::decode_prechecked(wire) })
        }
        unsafe fn encode_prechecked(&self) -> Result<(Self::Wire, ()), NcStatus> {
            let mut raw = [0; 128];
            raw[0] = u8::from(self.0);
            Ok((raw, ()))
        }
        unsafe fn decode_prechecked(wire: Self::Wire) -> Result<Self, NcStatus> {
            // SAFETY: the first initialized wire byte has the checked bool format.
            unsafe { bool::decode_prechecked(wire[0]) }.map(Self)
        }
    }
    // SAFETY: the initialized wire contains no references or owned resources.
    unsafe impl ComInlineValue for Padded {}
    assert_eq!(check_buffer_output::<Padded>((MAX_VALUE_BYTES / 128) as u64), Ok(()));
    assert_eq!(check_buffer_output::<Padded>((MAX_VALUE_BYTES / 128 + 1) as u64), Err(NC_E_OUT_OF_RANGE));
    // Option nesting tests admission without materializing a recursive ABI layout.
    type TooDeep = Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<bool>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>;
    assert_eq!(check_inline_output::<TooDeep>(), Err(NC_E_OUT_OF_RANGE));
    assert_eq!(check_buffer_output::<TooDeep>(1), Err(NC_E_OUT_OF_RANGE));
    assert_eq!(check_buffer_output::<TooDeep>(0), Ok(()));
}

#[com_object(IOneWay)]
/// Source implementation writes a short prefix and can reject after production.
struct OneWay;
impl IOneWay for OneWay {
    /// Failure deliberately leaves raw writes, but no prefix may be published.
    fn get(&self, reject: bool, mut output: NcOutputWriter<'_, Choice>) -> Result<u32, NcStatus> {
        output.push(Choice::Filled(Details { flags: [true, false] }))?;
        match reject {
            true => Err(NC_E_INVALID_ARG),
            false => Ok(37), // Required total is independent of initialized prefix.
        }
    }
}
#[com_object(IOneWay)]
/// Real COM delegation lends the same raw wire slots and requires no scratch storage.
struct Forwarded {
    /// Owning shared handle keeps only the provider alive, not caller storage.
    source: ComPtr<IOneWayAbi>,
}
impl IOneWay for Forwarded {
    /// The nested generated client validates its prefix before the outer thunk publishes.
    fn get(&self, reject: bool, output: NcOutputWriter<'_, Choice>) -> Result<u32, NcStatus> {
        self.source.get(reject, output.forward()?)
    }
}

/// Warm direct and delegated getters need no allocation and publish only initialized slots.
#[test]
fn generated_forwarding_preserves_short_prefix_and_discards_failed_output() {
    use allocation_probe::Probe;
    let source = OneWay::from_value(OneWay).query::<IOneWayAbi>().unwrap();
    let forwarded = Forwarded::from_value(Forwarded { source }).query::<IOneWayAbi>().unwrap();
    let mut output = NcOutputBuffer::new();
    assert_eq!(forwarded.get(false, output.output(0, 4)), Ok(37));
    assert_eq!(output.as_slice(), [Choice::Filled(Details { flags: [true, false] })]);
    // The complete nested call uses already reserved capacity on both sides.
    let probe = Probe::start(Some(0));
    let result = forwarded.get(false, output.output(0, 4));
    let rejected = forwarded.get(true, output.output(0, 4));
    let attempts = probe.finish().attempts;
    assert_eq!(result, Ok(37));
    assert_eq!(rejected, Err(NC_E_INVALID_ARG));
    assert!(output.is_empty());
    assert_eq!(attempts, 0);
}

/// A writer that has already produced values cannot relend its destination as fresh output.
#[test]
fn forwarding_rejects_an_already_written_prefix() {
    let mut slots = [core::mem::MaybeUninit::<u32>::uninit(); 2];
    let mut written = 0;
    // SAFETY: aligned stack storage and count are distinct, live and exclusive.
    let mut output = unsafe { NcOutputWriter::<u32>::from_raw(0, slots.as_mut_ptr().cast(), 2, &mut written) }.unwrap();
    output.push(9).unwrap();
    assert_eq!(output.forward().err().unwrap(), NC_E_INVALID_ARG);
    assert_eq!(written, 1);
}
