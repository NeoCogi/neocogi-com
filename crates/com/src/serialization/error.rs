//! Caller-owned diagnostic values copied through COM; no Rust error crosses an ABI.
use super::ISerializationDiagnosticsAbi;
use crate::{ComRef, ComInterface, NcStatus};
use crate::__alloc::string::{String, ToString};

/// A failed document operation with its provider's preserved first diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    /// Original operation status; diagnostic-query failures never replace it.
    pub status: NcStatus,
    /// Human-readable format, syntax or typed-schema failure.
    pub message: String,
    /// Logical field/index path rooted at `$`, when supplied by the provider.
    pub path: String,
    /// Zero-based format cursor or offending source byte, when known.
    pub byte_offset: Option<u64>,
    /// One-based source line, when the parser supplies one.
    pub line: Option<u32>,
    /// One-based source column, when the parser supplies one.
    pub column: Option<u32>,
}
impl Error {
    /// Creates a diagnostic before any session/location exists.
    pub fn new(status: NcStatus, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            path: "$".to_string(),
            byte_offset: None,
            line: None,
            column: None,
        }
    }
    /// Copies immutable details while the caller still retains the failed session.
    /// Broken or absent diagnostic projections cannot hide the original status.
    pub fn capture<I: ComInterface>(status: NcStatus, source: &crate::ComBox<I>) -> Self {
        let mut result = Self::from(status);
        let Ok(details) = source.query_ref::<ISerializationDiagnosticsAbi>() else {
            return result;
        };
        if let Some(message) = copied(details, true) {
            result.message = message;
        }
        if let Some(path) = copied(details, false) {
            result.path = path;
        }
        result.byte_offset = details.byte_offset().ok().filter(|value| *value != u64::MAX);
        result.line = details.line().ok().filter(|value| *value != 0);
        result.column = details.column().ok().filter(|value| *value != 0);
        result
    }
}
/// Admits a bounded foreign diagnostic before allocating local UTF-8 storage.
fn copied(details: ComRef<'_, ISerializationDiagnosticsAbi>, message: bool) -> Option<String> {
    let len = if message { details.message_len() } else { details.path_len() }.ok()? as usize;
    if len > 65_536 {
        return None;
    }
    let mut bytes = crate::NcOutputBuffer::<u8>::new();
    let written = if message {
        details.copy_message(bytes.output(0, len))
    } else {
        details.copy_path(bytes.output(0, len))
    }
    .ok()? as usize;
    if written != len || bytes.len() != len {
        return None;
    }
    String::from_utf8(bytes.into_vec()).ok()
}
impl From<NcStatus> for Error {
    /// Retains failures that precede a session or lack a usable diagnostic projection.
    fn from(status: NcStatus) -> Self {
        Self::new(status, "serialization operation failed")
    }
}
impl core::fmt::Display for Error {
    /// Displays logical context and source coordinates without requiring raw ABI access.
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(out, "{} at {}", self.message, self.path)?;
        if let (Some(line), Some(column)) = (self.line, self.column) {
            write!(out, " (line {line}, column {column})")?;
        } else if let Some(offset) = self.byte_offset {
            write!(out, " (byte {offset})")?;
        }
        write!(out, ": status {}", self.status.code())
    }
}
impl core::error::Error for Error {}
