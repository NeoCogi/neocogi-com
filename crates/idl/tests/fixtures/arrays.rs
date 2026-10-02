//! Exercises intrinsic arrays through copied, nested, and buffer ABI paths.
#[cfg(test)]
mod tests {
    use super::*;
    use neocogi_com::{com_object, ComPtr, NcStatus, NcCopyFromBufferOffset, NcOutputWriter, ValueArray, NC_E_INVALID_ARG};

    /// Counts accepted input calls, so validation failures cannot hide dispatch.
    #[com_object(IArrays)]
    struct Arrays {
        /// Calls whose inputs passed the generated wire decoder.
        accepted: usize,
    }
    impl IArrays for Arrays {
        /// Returns the complete copied array unchanged.
        fn echo(&mut self, value: [bool; 3]) -> Result<[bool; 3], NcStatus> {
            self.accepted += 1;
            Ok(value)
        }
        /// Preserves arrays inside copied structs as well as direct arrays.
        fn record(&mut self, value: Flags) -> Result<Flags, NcStatus> { Ok(value) }
        /// Decodes each array element through the normal checked input buffer.
        fn input(&mut self, values: NcCopyFromBufferOffset<'_, [bool; 3]>) -> Result<u32, NcStatus> {
            let decoded = values.as_slice();
            self.accepted += decoded.len();
            Ok(decoded.len() as u32)
        }
        /// Writes owned inline elements through the checked output buffer.
        fn output(&mut self, mut values: NcOutputWriter<'_, [bool; 3]>) -> Result<(), NcStatus> {
            values.extend_from_slice(&[[true, false, true]])?;
            Ok(())
        }
    }

    /// Real generated thunks must reject an invalid nested boolean before dispatch.
    #[test]
    fn copied_arrays_round_trip_and_validate_raw_elements() {
        let local = Arrays::into_local(Arrays { accepted: 0 });
        let mut arrays: ComPtr<IArraysAbi> = local.query().unwrap();
        assert_eq!(arrays.echo([false, true, false]).unwrap(), [false, true, false]);
        assert_eq!(arrays.record(Flags { bits: [[true, false], [false, true]] }).unwrap().bits, [[true, false], [false, true]]);
        assert_eq!(arrays.input(NcCopyFromBufferOffset::new(0, &[[true; 3]])).unwrap(), 1);
        let mut destination = neocogi_com::NcOutputBuffer::new();
        arrays.output(destination.output(0, 1)).unwrap();
        assert_eq!(destination.as_slice(), [[true, false, true]]);
        let raw = arrays.as_raw();
        let bad = ValueArray { elements: [0u8, 2, 1] };
        let mut out = core::mem::MaybeUninit::uninit();
        let mut count = 99u32;
        // SAFETY: all wire storage and the object/table pair are live. The bad
        // byte is legal u8 storage and must never become an unchecked Rust bool.
        unsafe {
            assert_eq!(((*raw.vtable).echo)(raw.object, bad, out.as_mut_ptr()), NC_E_INVALID_ARG);
            assert_eq!(((*raw.vtable).input)(raw.object, 0, 1, &bad, &mut count), NC_E_INVALID_ARG);
        }
        assert_eq!(local.try_borrow().unwrap().accepted, 2);
    }
}
