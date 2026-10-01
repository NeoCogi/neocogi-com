//! Buffer completion reuses storage reserved before any implementation can commit.
#![recursion_limit = "256"]
use neocogi_com::*;
#[path = "support/allocation_probe.rs"]
mod allocation_probe;
use allocation_probe::Probe;

#[test]
fn converted_guards_allocate_only_during_preparation() {
    let mut target = [false; 3];
    let mut caller = EncodedOutput::new(NcCopyToBufferOffset::new(0, &mut target)).unwrap();
    // SAFETY: caller retains the initialized exclusive wire region until completion.
    let mut provider = unsafe { DecodedOutput::<bool>::from_raw(caller.as_mut_ptr(), caller.len()) }.unwrap();
    provider.as_mut_slice().fill(true);
    let probe = Probe::start(Some(0));
    let sent = provider.finish();
    let received = caller.finish();
    let report = probe.finish();
    assert_eq!(sent, Ok(()));
    assert_eq!(received, Ok(()));
    assert_eq!(report.attempts, 0);
    assert_eq!(target, [true; 3]);
}

#[test]
fn every_guard_allocation_is_fallible_and_leaves_destination_unchanged() {
    // Two vectors per side: initialized input and reserved completion scratch.
    for ordinal in 1..=5 {
        let mut target = [false; 3];
        let probe = Probe::start(Some(ordinal));
        let result = (|| -> Result<(), NcStatus> {
            let mut caller = EncodedOutput::new(NcCopyToBufferOffset::new(0, &mut target))?;
            // SAFETY: caller owns live initialized storage and grants exclusive access.
            let _provider = unsafe { DecodedOutput::<bool>::from_raw(caller.as_mut_ptr(), caller.len()) }?;
            Ok(())
        })();
        let report = probe.finish();
        match ordinal {
            1..=4 => {
                assert_eq!(result, Err(NC_E_OUT_OF_MEMORY), "allocation {ordinal}");
                assert_eq!(report.denied, 1);
            }
            _ => {
                assert_eq!(result, Ok(()));
                assert_eq!(report.attempts, 4);
            }
        }
        assert_eq!(target, [false; 3]);
    }
}

#[test]
fn malformed_output_does_not_publish_a_valid_prefix() {
    let mut target = [false; 3];
    let mut caller = EncodedOutput::new(NcCopyToBufferOffset::new(0, &mut target)).unwrap();
    // SAFETY: raw u8 wires accept all bit patterns; bool validity is checked later.
    unsafe { caller.as_mut_ptr().copy_from_nonoverlapping([1, 1, 7].as_ptr(), 3) };
    let probe = Probe::start(Some(0));
    let result = caller.finish();
    let report = probe.finish();
    assert_eq!(result, Err(NC_E_INVALID_ARG));
    assert_eq!(report.attempts, 0);
    assert_eq!(target, [false; 3]);
}

#[test]
fn direct_and_empty_buffers_need_no_scratch() {
    let mut target = [2_u32; 3];
    let address = target.as_mut_ptr();
    let probe = Probe::start(Some(0));
    let result = (|| -> Result<_, NcStatus> {
        let mut caller = EncodedOutput::new(NcCopyToBufferOffset::new(0, &mut target))?;
        let direct = caller.as_mut_ptr() == address;
        // SAFETY: caller owns the original exclusive numeric slice.
        let mut provider = unsafe { DecodedOutput::<u32>::from_raw(caller.as_mut_ptr(), caller.len()) }?;
        provider.as_mut_slice().fill(9);
        provider.finish()?;
        caller.finish()?;
        let mut empty = [];
        let mut caller = EncodedOutput::<bool>::new(NcCopyToBufferOffset::new(0, &mut empty))?;
        // SAFETY: an empty initialized buffer needs no addressable element.
        let provider = unsafe { DecodedOutput::<bool>::from_raw(caller.as_mut_ptr(), 0) }?;
        provider.finish()?;
        caller.finish()?;
        Ok(direct)
    })();
    let report = probe.finish();
    assert_eq!(result, Ok(true));
    assert_eq!(report.attempts, 0);
    assert_eq!(target, [9; 3]);
}

#[com_struct(assigned_signature = b"00000000000000000000000000000811")]
#[derive(Debug, PartialEq)]
/// Nested restricted leaves require validation in both conversion directions.
pub struct Details {
    /// A fixed array changes both the maximum count and maximum depth.
    pub flags: [bool; 2],
}
#[com_enum(tag = u8, assigned_signature = b"00000000000000000000000000000812")]
#[derive(Clone, Copy, Debug, PartialEq)]
/// Unequal alternatives make input-only budget checks insufficient.
pub enum Choice {
    /// Cheap current value does not bound a provider's possible replacement.
    Idle,
    /// More expensive but still allocation-free output.
    Filled(Details),
}
/// A copied zero-sized return can still exceed the logical traversal limit.
type Excessive = [[bool; 0]; MAX_VALUE_COUNT];
#[com_interface(auto_guid = "nc.tests.completion.Item:v1", assigned_signature = b"00000000000000000000000000000813")]
/// Small exclusive input observed through a weak reference after transfer.
trait IItem {
    /// Confirms liveness without owning the object.
    fn value(&self) -> Result<u32, NcStatus>;
}
#[com_interface(auto_guid = "nc.tests.completion.Complete:v1", assigned_signature = b"00000000000000000000000000000814")]
/// Real generated ownership and completion paths, including multiple outputs.
trait IComplete {
    /// Accepts both owners or returns both original identities, with checked outputs.
    fn complete(
        &mut self,
        first: ComBox<IItemAbi>,
        second: ComBox<IItemAbi>,
        choices: NcCopyToBufferOffset<'_, Choice>,
        flags: NcCopyToBufferOffset<'_, bool>,
        reject: bool,
        deny_after_entry: bool,
    ) -> Result<Choice, (NcStatus, ComBox<IItemAbi>, ComBox<IItemAbi>)>;
    /// Direct numeric buffers retain their existing allocation-free identity path.
    fn direct(&mut self, first: ComBox<IItemAbi>, values: NcCopyToBufferOffset<'_, u64>) -> Result<u32, (NcStatus, ComBox<IItemAbi>)>;
    /// Known but excessive maximum must reject before implementation entry.
    fn excessive(&mut self, first: ComBox<IItemAbi>) -> Result<Excessive, (NcStatus, ComBox<IItemAbi>)>;
}
#[path = "support/output_completion_fixture.rs"]
mod generated_calls;

#[com_interface(auto_guid = "nc.tests.completion.Ordinary:v1", assigned_signature = b"00000000000000000000000000000831")]
/// Ordinary fallible outputs still convert completely before any publication.
trait IOrdinaryOutputs {
    /// A later converted buffer may exceed the actual-value budget.
    fn buffers(&mut self, flags: NcCopyToBufferOffset<'_, bool>, choices: NcCopyToBufferOffset<'_, Choice>) -> Result<(), NcStatus>;
    /// An invalid copied return must not publish earlier converted buffers either.
    fn result(&mut self, flags: NcCopyToBufferOffset<'_, bool>) -> Result<Excessive, NcStatus>;
}
#[com_object(IOrdinaryOutputs)]
/// Deliberately produces valid native outputs exceeding ordinary conversion limits.
struct OrdinaryOutputs;
impl IOrdinaryOutputs for OrdinaryOutputs {
    /// Each current Idle is cheap, but Filled may exceed the combined output count.
    fn buffers(&mut self, mut flags: NcCopyToBufferOffset<'_, bool>, mut choices: NcCopyToBufferOffset<'_, Choice>) -> Result<(), NcStatus> {
        flags.as_mut_slice().fill(true);
        choices.as_mut_slice().fill(Choice::Filled(Details { flags: [true; 2] }));
        Ok(())
    }
    /// Returns zero physical bytes with too many logical array values.
    fn result(&mut self, mut flags: NcCopyToBufferOffset<'_, bool>) -> Result<Excessive, NcStatus> {
        flags.as_mut_slice().fill(true);
        Ok([[]; MAX_VALUE_COUNT])
    }
}

#[test]
fn provider_validates_all_converted_outputs_before_publishing_any() {
    let provider = OrdinaryOutputs::from_value(OrdinaryOutputs).query::<IOrdinaryOutputsAbi>().unwrap();
    let raw = provider.as_raw();
    let mut flags = [0_u8];
    let mut choices = vec![encode_inline(&Choice::Idle).unwrap(); MAX_VALUE_COUNT / 5 + 1];
    // SAFETY: live initialized writable wire buffers and a valid provider object.
    let status = unsafe { ((*raw.vtable).buffers)(raw.object, 0, 1, flags.as_mut_ptr(), 0, choices.len() as u64, choices.as_mut_ptr()) };
    assert_eq!(status, NC_E_OUT_OF_RANGE);
    assert_eq!(flags, [0]);
    assert!(choices.iter().all(|wire| unsafe { decode_inline::<Choice>(*wire) }.unwrap() == Choice::Idle));
    let mut out = std::mem::MaybeUninit::uninit();
    // SAFETY: output storage matches the copied wire, including its zero size.
    let status = unsafe { ((*raw.vtable).result)(raw.object, 0, 1, flags.as_mut_ptr(), out.as_mut_ptr()) };
    assert_eq!(status, NC_E_OUT_OF_RANGE);
    assert_eq!(flags, [0]);
}

/// Simulates a foreign provider violating the boolean output representation.
/// The raw wire remains initialized u8 storage; no invalid Rust bool is created.
unsafe extern "C" fn invalid_later_buffer(
    _this: *mut std::ffi::c_void,
    _flags_offset: u64,
    flags_len: u64,
    flags: *mut u8,
    _choices_offset: u64,
    choices_len: u64,
    choices: *mut <Choice as ComValue>::Wire,
) -> NcStatus {
    assert_eq!((flags_len, choices_len), (1, 1));
    let mut malformed = encode_inline(&Choice::Filled(Details { flags: [true; 2] })).unwrap();
    // The generated wire exposes the raw enum tag independently of its payload.
    malformed.tag = 99;
    // SAFETY: this test's caller supplies two distinct writable single-element buffers.
    unsafe {
        *flags = 1;
        *choices = malformed;
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
    let mut flags = [false];
    let mut choices = [Choice::Idle];
    let status = client.buffers(NcCopyToBufferOffset::new(0, &mut flags), NcCopyToBufferOffset::new(0, &mut choices));
    assert_eq!(status, Err(NC_E_INVALID_TAG));
    assert_eq!(flags, [false]);
    assert_eq!(choices, [Choice::Idle]);
}

#[test]
fn consuming_output_admission_checks_bytes_count_depth_and_exact_boundaries() {
    // bool is restricted, whereas u64 remains a direct in-place buffer.
    assert_eq!(check_buffer_output::<bool>(MAX_VALUE_COUNT as u64), Ok(()));
    assert_eq!(check_buffer_output::<bool>(MAX_VALUE_COUNT as u64 + 1), Err(NC_E_OUT_OF_RANGE));
    assert_eq!(check_buffer_output::<u64>(u64::MAX), Ok(()));
    // Existing raw pointer checks still reject invalid lengths for direct buffers;
    // output admission alone has no conversion budget to impose on this path.
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
