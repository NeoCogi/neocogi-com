//! One shared plugin root selects independently compiled format implementations.
use neocogi_com_sdk::{ComPtr, NcStatus, NcCopyFromBufferOffset, NC_E_NOINTERFACE, NC_E_INVALID_ARG};
use neocogi_com_sdk::serialization::*;

/// Stateless selection service; it owns no document sessions or application values.
#[neocogi_com_sdk::com_object(ISerializationPlugin)]
struct Formats;

impl ISerializationPlugin for Formats {
    /// NCB is unconditional, including when optional JSON is disabled.
    fn default_format(&self) -> Result<ComPtr<ISerializationFormatAbi>, NcStatus> {
        Ok(crate::ncb::factory())
    }
    /// Disabled and unknown formats report absence without falling back silently.
    fn format(&self, name: NcCopyFromBufferOffset<'_, u8>) -> Result<ComPtr<ISerializationFormatAbi>, NcStatus> {
        if name.offset() != 0 {
            return Err(NC_E_INVALID_ARG);
        }
        match core::str::from_utf8(name.as_slice()).map_err(|_| NC_E_INVALID_ARG)? {
            "ncb" => Ok(crate::ncb::factory()),
            #[cfg(feature = "json")]
            "json" => Ok(crate::json::factory()),
            _ => Err(NC_E_NOINTERFACE),
        }
    }
}

/// Acquires the same shared root directly in a statically linked application.
pub fn factory() -> ComPtr<ISerializationPluginAbi> {
    Formats::from_value(Formats)
        .query()
        .unwrap_or_else(|_| unreachable!("format selection projection"))
}

/// Static registration and native loading share this exact acquisition contract.
#[neocogi_com_sdk::plugin(name = "nc.serialization")]
fn acquire(_runtime: neocogi_com_sdk::RuntimeHandle) -> Result<ComPtr<ISerializationPluginAbi>, NcStatus> {
    Ok(factory())
}
