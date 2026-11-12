//! Runtime-selectable format COM objects and typed value traversal.
//!
//! The interfaces and value traits are shared with core. NCB objects own their
//! buffers; binary convenience functions also support nonretainable scoped
//! objects for allocation-free fixed buffers. No host runtime is required.

pub use neocogi_com_core::serialization::*;
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
        let writer = self.serializer()?;
        Serializer::new(writer.as_com_ref())
            .serialize(value)
            .map_err(|status| Error::capture(status, &writer))?;
        let output = writer.query_ref::<ISerializationOutputAbi>()?;
        let len = usize::try_from(output.byte_len()?).map_err(|_| crate::NC_E_OUT_OF_RANGE)?;
        if len > self.limits.bytes {
            return Err(crate::NC_E_OUT_OF_RANGE.into());
        }
        let mut bytes = crate::NcOutputBuffer::<u8>::new();
        let written = usize::try_from(output.copy_to(bytes.output(0, len))?).map_err(|_| crate::NC_E_OUT_OF_RANGE)?;
        if written != len || bytes.len() != len {
            return Err(crate::NC_E_INVALID_ARG.into());
        }
        Ok(bytes.into_vec())
    }
    /// Constructs a typed value and rejects trailing input before returning it.
    pub fn deserialize<T: crate::ComDeserialize>(&self, bytes: &[u8]) -> Result<T, Error> {
        let reader = self.deserializer(bytes)?;
        Deserializer::with_limits(reader.as_com_ref(), self.limits)
            .deserialize()
            .map_err(|status| Error::capture(status, &reader))
    }
}
