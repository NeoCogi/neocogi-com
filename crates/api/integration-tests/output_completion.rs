//! Buffer completion reuses storage reserved before any implementation can commit.
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
