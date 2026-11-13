//! Fault injection wraps a real owning NCB COM session; data/framing stays real.
use neocogi_com::{ComBox, NcStatus, NcCopyFromBufferOffset, NcOutputWriter, NC_E_INVALID_ARG, NC_E_BUSY, NC_E_CLOSED, NC_E_NOT_FOUND, NC_E_OUT_OF_RANGE};
use neocogi_com::serialization::{IDeserializer, IDeserializerAbi, Limits};

/// One controlled failure boundary; the first failure remains terminal.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Stage {
    /// No fault; inspect cardinality errors and successful completion.
    None,
    /// Fail before returning the collection count.
    Begin,
    /// Fail after all sequence children exist.
    End,
    /// Fail when leaving the complete root, not individual children.
    Leave,
    /// Fail after successful root acquisition.
    Finish,
}
impl Stage {
    /// Distinct statuses expose replacement by a later schema failure.
    pub(super) fn status(self) -> NcStatus {
        match self {
            Self::None => NC_E_INVALID_ARG,
            Self::Begin => NC_E_BUSY,
            Self::End => NC_E_CLOSED,
            Self::Leave => NC_E_NOT_FOUND,
            Self::Finish => NC_E_OUT_OF_RANGE,
        }
    }
}
/// Concrete decorator invoked through the ordinary scoped COM vtable.
pub(super) struct FaultSource {
    /// Real provider owns input, framing, budgets and normal diagnostics.
    inner: ComBox<IDeserializerAbi>,
    /// Only this boundary receives an injected error.
    stage: Stage,
    /// Value nesting identifies root leave without failing child completion.
    depth: usize,
    /// Successful unit acquisitions; inspect after recovering the scoped provider.
    pub(super) units: usize,
    /// First status prevents all later operations from succeeding.
    failed: Option<NcStatus>,
}
impl FaultSource {
    /// Admits real bytes through the production owning NCB factory.
    pub(super) fn new(bytes: Vec<u8>, stage: Stage) -> Self {
        Self {
            inner: neocogi_com_serialization::ncb::NcbDeserializer::create(bytes, Limits::default()).unwrap(),
            stage,
            depth: 0,
            units: 0,
            failed: None,
        }
    }
    /// Preserves a terminal status without changing successful provider behavior.
    fn admit(&self) -> Result<(), NcStatus> {
        match self.failed {
            Some(status) => Err(status),
            None => Ok(()),
        }
    }
    /// Records the selected boundary failure once.
    fn inject(&mut self, stage: Stage) -> Result<(), NcStatus> {
        self.admit()?;
        if self.stage == stage {
            let status = stage.status();
            self.failed = Some(status);
            Err(status)
        } else {
            Ok(())
        }
    }
}
impl IDeserializer for FaultSource {
    /// Delegates to the real NCB session after failure admission.
    fn enter_value(&mut self) -> Result<(), NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().enter_value()?;
        self.depth += 1;
        Ok(())
    }
    /// Delegates to the real NCB session after failure admission.
    fn leave_value(&mut self) -> Result<(), NcStatus> {
        self.admit()?;
        if self.depth == 1 {
            self.inject(Stage::Leave)?;
        }
        self.inner.as_com_ref().leave_value()?;
        self.depth -= 1;
        Ok(())
    }
    /// Delegates to the real NCB session after failure admission.
    fn deserialize_unit(&mut self) -> Result<(), NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().deserialize_unit()?;
        self.units += 1;
        Ok(())
    }
    /// Delegates to the real NCB session after failure admission.
    fn deserialize_bool(&mut self) -> Result<bool, NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().deserialize_bool()
    }
    /// Delegates to the real NCB session after failure admission.
    fn deserialize_u8(&mut self) -> Result<u8, NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().deserialize_u8()
    }
    /// Delegates to the real NCB session after failure admission.
    fn deserialize_u16(&mut self) -> Result<u16, NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().deserialize_u16()
    }
    /// Delegates to the real NCB session after failure admission.
    fn deserialize_u32(&mut self) -> Result<u32, NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().deserialize_u32()
    }
    /// Delegates to the real NCB session after failure admission.
    fn deserialize_u64(&mut self) -> Result<u64, NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().deserialize_u64()
    }
    /// Delegates to the real NCB session after failure admission.
    fn deserialize_i8(&mut self) -> Result<i8, NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().deserialize_i8()
    }
    /// Delegates to the real NCB session after failure admission.
    fn deserialize_i16(&mut self) -> Result<i16, NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().deserialize_i16()
    }
    /// Delegates to the real NCB session after failure admission.
    fn deserialize_i32(&mut self) -> Result<i32, NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().deserialize_i32()
    }
    /// Delegates to the real NCB session after failure admission.
    fn deserialize_i64(&mut self) -> Result<i64, NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().deserialize_i64()
    }
    /// Delegates to the real NCB session after failure admission.
    fn deserialize_f32(&mut self) -> Result<f32, NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().deserialize_f32()
    }
    /// Delegates to the real NCB session after failure admission.
    fn deserialize_f64(&mut self) -> Result<f64, NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().deserialize_f64()
    }
    /// Delegates to the real NCB session after failure admission.
    fn string_len(&mut self) -> Result<u32, NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().string_len()
    }
    /// Delegates to the real NCB session after failure admission.
    fn copy_string(&mut self, output: NcOutputWriter<'_, u8>) -> Result<u32, NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().copy_string(output.forward()?)
    }
    /// Delegates to the real NCB session after failure admission.
    fn option_present(&mut self) -> Result<bool, NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().option_present()
    }
    /// Delegates to the real NCB session after failure admission.
    fn begin_record(&mut self, maximum: u32) -> Result<u32, NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().begin_record(maximum)
    }
    /// Delegates to the real NCB session after failure admission.
    fn field(&mut self, name: NcCopyFromBufferOffset<'_, u8>) -> Result<bool, NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().field(name)
    }
    /// Delegates to the real NCB session after failure admission.
    fn end_record(&mut self) -> Result<(), NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().end_record()
    }
    /// Delegates to the real NCB session after failure admission.
    fn begin_sequence(&mut self) -> Result<u32, NcStatus> {
        self.inject(Stage::Begin)?;
        self.inner.as_com_ref().begin_sequence()
    }
    /// Delegates to the real NCB session after failure admission.
    fn end_sequence(&mut self) -> Result<(), NcStatus> {
        self.inject(Stage::End)?;
        self.inner.as_com_ref().end_sequence()
    }
    /// Delegates to the real NCB session after failure admission.
    fn begin_map(&mut self) -> Result<u32, NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().begin_map()
    }
    /// Delegates to the real NCB session after failure admission.
    fn end_map(&mut self) -> Result<(), NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().end_map()
    }
    /// Delegates to the real NCB session after failure admission.
    fn variant_is(&mut self, index: u32, name: NcCopyFromBufferOffset<'_, u8>) -> Result<bool, NcStatus> {
        self.admit()?;
        self.inner.as_com_ref().variant_is(index, name)
    }
    /// Delegates to the real NCB session after failure admission.
    fn fail(&mut self, message: NcCopyFromBufferOffset<'_, u8>) -> Result<(), NcStatus> {
        if let Some(status) = self.failed {
            return Err(status);
        }
        let status = self.inner.as_com_ref().fail(message).err().unwrap_or(NC_E_INVALID_ARG);
        self.failed = Some(status);
        Err(status)
    }
    /// Delegates to the real NCB session after failure admission.
    fn finish(&mut self) -> Result<(), NcStatus> {
        self.inject(Stage::Finish)?;
        self.inner.as_com_ref().finish()
    }
}
