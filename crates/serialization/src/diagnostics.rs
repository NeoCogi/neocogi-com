//! Shared read-only diagnostic projection. Strings are copied only when a caller
//! asks for them; successful document traversal allocates no diagnostic objects.
use neocogi_com_sdk::{NcStatus, NcOutputWriter, NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE};

/// Borrowed first-failure data, never transferred as a Rust value through COM.
pub(crate) struct Details<'a> {
    /// Original operation status, independent of the diagnostic query's status.
    pub status: NcStatus,
    /// Stable provider-owned UTF-8 until the session is destroyed.
    pub message: &'a str,
    /// Logical path or `$` when only a byte cursor is available.
    pub path: &'a str,
    /// Source byte/cursor, with u64::MAX representing unavailable coordinates.
    pub offset: u64,
    /// One-based line, or zero for formats/operations without line information.
    pub line: u32,
    /// One-based column, or zero when unavailable.
    pub column: u32,
}
/// Copies one whole diagnostic string without publishing a short prefix.
pub(crate) fn copy(value: &str, mut output: NcOutputWriter<'_, u8>) -> Result<u32, NcStatus> {
    if output.offset() != 0 {
        return Err(NC_E_INVALID_ARG);
    }
    let length = u32::try_from(value.len()).map_err(|_| NC_E_OUT_OF_RANGE)?;
    output.extend_from_slice(value.as_bytes())?;
    Ok(length)
}
/// Derives the identical read-only COM surface for each concrete session's
/// borrowed `details` method, without retaining or erasing the implementation.
macro_rules! diagnostic_projection {
    ($object:ty) => {
        impl neocogi_com_sdk::serialization::ISerializationDiagnostics for $object {
            fn failure_status(&self) -> Result<neocogi_com_sdk::NcStatus, neocogi_com_sdk::NcStatus> {
                Ok(self.details()?.status)
            }
            fn byte_offset(&self) -> Result<u64, neocogi_com_sdk::NcStatus> {
                Ok(self.details()?.offset)
            }
            fn line(&self) -> Result<u32, neocogi_com_sdk::NcStatus> {
                Ok(self.details()?.line)
            }
            fn column(&self) -> Result<u32, neocogi_com_sdk::NcStatus> {
                Ok(self.details()?.column)
            }
            fn message_len(&self) -> Result<u32, neocogi_com_sdk::NcStatus> {
                u32::try_from(self.details()?.message.len()).map_err(|_| neocogi_com_sdk::NC_E_OUT_OF_RANGE)
            }
            fn copy_message(&self, output: neocogi_com_sdk::NcOutputWriter<'_, u8>) -> Result<u32, neocogi_com_sdk::NcStatus> {
                $crate::diagnostics::copy(self.details()?.message, output)
            }
            fn path_len(&self) -> Result<u32, neocogi_com_sdk::NcStatus> {
                u32::try_from(self.details()?.path.len()).map_err(|_| neocogi_com_sdk::NC_E_OUT_OF_RANGE)
            }
            fn copy_path(&self, output: neocogi_com_sdk::NcOutputWriter<'_, u8>) -> Result<u32, neocogi_com_sdk::NcStatus> {
                $crate::diagnostics::copy(self.details()?.path, output)
            }
        }
    };
}
