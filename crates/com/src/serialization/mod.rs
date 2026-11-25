//! Runtime-selectable format COM objects and typed value traversal.
//!
//! The interfaces and value traits are shared with core; this facade provides
//! format API wrappers and provider thunks. Concrete codecs, owning NCB/JSON session
//! state and NCB conveniences live in `neocogi-com-serialization`. Its scoped NCB
//! objects support allocation-free fixed buffers without a host runtime.

pub use neocogi_com::serialization::*;
#[rustfmt::skip]
mod providers;
pub use providers::*;
#[cfg(feature = "alloc")]
mod error;
#[cfg(feature = "alloc")]
pub use error::Error;

/// Selects a format through its shared COM factory and bounds each new document.
#[cfg(feature = "alloc")]
pub struct Format {
    /// Factory implementation may live in another linked artifact or plugin.
    object: crate::ComPtr<ISerializationFormatAbi>,
    /// Per-document resource policy copied into each exclusive session.
    limits: Limits,
}
#[cfg(feature = "alloc")]
impl Format {
    /// Selects an arbitrary COM format provider without linking a host implementation.
    pub fn new(object: crate::ComPtr<ISerializationFormatAbi>, limits: Limits) -> Self {
        Self { object, limits }
    }
    /// Creates one independently mutable writer with provider-owned output.
    pub fn serializer(&self) -> Result<crate::ComBox<ISerializerAbi>, crate::NcStatus> {
        self.object.create_serializer(
            self.limits.bytes as u64,
            u32::try_from(self.limits.elements).map_err(|_| crate::NC_E_OUT_OF_RANGE)?,
            u32::try_from(self.limits.depth).map_err(|_| crate::NC_E_OUT_OF_RANGE)?,
            self.limits.values as u64,
        )
    }
    /// Creates an owning source; the provider copies input before returning.
    pub fn deserializer(&self, bytes: &[u8]) -> Result<crate::ComBox<IDeserializerAbi>, crate::NcStatus> {
        if bytes.len() > self.limits.bytes {
            return Err(crate::NC_E_OUT_OF_RANGE);
        }
        self.object.create_deserializer(
            crate::NcCopyFromBufferOffset::new(0, bytes),
            self.limits.bytes as u64,
            u32::try_from(self.limits.elements).map_err(|_| crate::NC_E_OUT_OF_RANGE)?,
            u32::try_from(self.limits.depth).map_err(|_| crate::NC_E_OUT_OF_RANGE)?,
            self.limits.values as u64,
        )
    }
    /// Writes a complete typed document and copies its committed bytes locally.
    pub fn serialize<T: crate::ComSerialize + ?Sized>(&self, value: &T) -> Result<crate::__alloc::vec::Vec<u8>, Error> {
        self.write_document(value, true, |status, writer| match writer {
            Some(writer) => Error::capture(status, writer),
            None => Error::from(status),
        })
    }
    /// Writes a complete document for a caller that needs only the status.
    ///
    /// Failed sessions are discarded without automatic failure reporting or
    /// copying diagnostics. The value may still report a failure explicitly.
    #[doc(hidden)]
    pub fn serialize_status<T: crate::ComSerialize + ?Sized>(&self, value: &T) -> Result<crate::__alloc::vec::Vec<u8>, crate::NcStatus> {
        self.write_document(value, false, |status, _| status)
    }
    /// Owns one session through the write, commit and checked output copy.
    fn write_document<T: crate::ComSerialize + ?Sized, E>(
        &self,
        value: &T,
        report_failure: bool,
        make_error: impl Fn(crate::NcStatus, Option<&crate::ComBox<ISerializerAbi>>) -> E,
    ) -> Result<crate::__alloc::vec::Vec<u8>, E> {
        let mut writer = self.serializer().map_err(|status| make_error(status, None))?;
        Serializer::new(writer.as_com_mut())
            .write_root(value, report_failure)
            .map_err(|status| make_error(status, Some(&writer)))?;
        self.copy_finished_output(&writer).map_err(|status| make_error(status, None))
    }
    /// Checks both output counts before returning the copied committed bytes.
    fn copy_finished_output(&self, writer: &crate::ComBox<ISerializerAbi>) -> Result<crate::__alloc::vec::Vec<u8>, crate::NcStatus> {
        let output = writer.query_ref::<ISerializationOutputAbi>()?;
        let len = usize::try_from(output.byte_len()?).map_err(|_| crate::NC_E_OUT_OF_RANGE)?;
        if len > self.limits.bytes {
            return Err(crate::NC_E_OUT_OF_RANGE);
        }
        let mut bytes = crate::NcOutputBuffer::<u8>::new();
        let written = usize::try_from(output.copy_to(bytes.output(0, len))?).map_err(|_| crate::NC_E_OUT_OF_RANGE)?;
        if written != len || bytes.len() != len {
            return Err(crate::NC_E_INVALID_ARG);
        }
        Ok(bytes.into_vec())
    }
    /// Constructs a typed value and rejects trailing input before returning it.
    pub fn deserialize<T: crate::ComDeserialize>(&self, bytes: &[u8]) -> Result<T, Error> {
        let mut reader = self.deserializer(bytes)?;
        let result = Deserializer::with_limits(reader.as_com_mut(), self.limits).deserialize();
        result.map_err(|status| Error::capture(status, &reader))
    }
}
