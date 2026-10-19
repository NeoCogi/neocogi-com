//! Explicit failure reports use the ordinary exclusive event and immutable string contracts.
use super::{IErrorEvent, IEvent, IEventAbi};
use crate::{ComBox, ComPtr, INcStringAbi, NcStatus, NcString, NC_E_INVALID_ARG};

#[crate::com_object(interfaces(crate::events::IErrorEvent), ownership = exclusive)]
/// Captured failure with no producer reference, mutable error state or delivery policy.
struct ErrorEvent {
    /// Validated negative code; domain-specific codes are preserved verbatim.
    status: NcStatus,
    /// One immutable string allocation shared by repeatable getter results.
    message: ComPtr<INcStringAbi>,
}
impl IEvent for ErrorEvent {}
impl IErrorEvent for ErrorEvent {
    /// Reads captured data without consulting a source widget or the current origin stack.
    fn status(&self) -> Result<i32, NcStatus> {
        Ok(self.status.code())
    }
    /// Acquires another reference to the same immutable string, without copying its contents.
    fn message(&self) -> Result<ComPtr<INcStringAbi>, NcStatus> {
        Ok(self.message.clone())
    }
}

/// Captures a reported failure as the sole event owner, without a source reference.
///
/// Nonnegative statuses return `NC_E_INVALID_ARG` before allocating. Unknown negative
/// domain codes are valid. The UTF-8 message is copied once and may be empty.
/// Post through the producer's ordinary destination to attach its weak origin.
/// This constructor neither queues the report nor converts delivery failures into
/// further reports; callers propagate transport errors directly.
///
/// ```
/// use neocogi_com::{events::{error, IErrorEventAbi}, INcStringComPtrExt, NC_E_NOT_FOUND};
/// let event = error(NC_E_NOT_FOUND, "Requested item is unavailable")?;
/// let report = event.query_ref::<IErrorEventAbi>()?;
/// assert_eq!(report.status()?, NC_E_NOT_FOUND.code());
/// assert_eq!(report.message()?.to_string()?, "Requested item is unavailable");
/// # Ok::<(), neocogi_com::NcStatus>(())
/// ```
pub fn error(status: NcStatus, message: &str) -> Result<ComBox<IEventAbi>, NcStatus> {
    if status.is_ok() {
        return Err(NC_E_INVALID_ARG);
    }
    ErrorEvent::into_box(ErrorEvent {
        status,
        message: NcString::from_utf8_str(message),
    })
    .query()
    .map_err(|(status, _unpublished)| status)
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::{events::IErrorEventAbi, INcStringComPtrExt, NC_E_DROPPED};

    /// Actual generated getters preserve unknown errors and copied UTF-8 independently of owners.
    #[test]
    fn errors_capture_negative_status_and_share_one_immutable_message() {
        for code in [-1, crate::NC_E_CLOSED.code(), -987_654, i32::MIN] {
            for initial in ["", "failure α\0🦀"] {
                let mut input = initial.to_owned();
                let event = error(NcStatus::from_code(code), &input).unwrap();
                input.clear();
                let report = event.query_ref::<IErrorEventAbi>().unwrap();
                assert_eq!(report.status(), Ok(code));
                let first = report.message().unwrap();
                let second = report.message().unwrap();
                assert_eq!(first.as_raw().object, second.as_raw().object);
                assert_eq!(first.to_string().unwrap(), initial);
                let observer = event.query_weak::<IErrorEventAbi>().unwrap();
                drop(event);
                assert!(!observer.is_alive());
                assert_eq!(observer.status(), Err(NC_E_DROPPED));
                assert_eq!(first.to_string().unwrap(), initial, "getter result owns its immutable data");
            }
        }
    }

    /// A success status cannot be packaged as an error, including extension success codes.
    #[test]
    fn error_constructor_rejects_nonnegative_statuses() {
        for code in [0, 1, i32::MAX] {
            assert_eq!(error(NcStatus::from_code(code), "invalid").unwrap_err(), NC_E_INVALID_ARG);
        }
    }
}
