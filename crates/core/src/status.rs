//! Stable status-code representation and built-in neocogi-com error categories.
//!
//! ABI calls use non-negative values for success and negative values for
//! failure. [`NcStatus`] preserves unknown extension codes, supplies symbolic
//! names for the built-ins, and normalizes malformed `Result::Err(success)`
//! values before generated Rust thunks publish them to foreign callers.

/// Status returned by COM ABI calls.
///
/// Non-negative values are considered success. Negative values are errors.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcStatus(i32);

impl NcStatus {
    /// Creates a status from a raw ABI status code.
    pub const fn from_code(code: i32) -> Self {
        Self(code)
    }

    /// Returns the raw ABI status code.
    pub const fn code(self) -> i32 {
        self.0
    }

    /// Returns `true` when the status is successful.
    pub const fn is_ok(self) -> bool {
        self.0 >= 0
    }

    /// Returns `true` when the status is an error.
    pub const fn is_err(self) -> bool {
        !self.is_ok()
    }

    /// Normalizes a status used as the error variant of a Rust [`Result`].
    ///
    /// Negative statuses are preserved. Non-negative statuses cannot represent
    /// `Result::Err` at the ABI boundary and are mapped to
    /// [`NC_E_INVALID_ARG`]. This keeps a malformed `Err(success)` from being
    /// observed by an ABI caller as proof that an output slot was initialized.
    pub const fn normalize_error(self) -> Self {
        if self.is_err() { self } else { NC_E_INVALID_ARG }
    }

    /// Returns `true` when this status is one of the built-in `neocogi-com` codes.
    pub const fn is_known(self) -> bool {
        matches!(
            self,
            NC_OK
                | NC_E_NOINTERFACE
                | NC_E_POINTER
                | NC_E_INVALID_TAG
                | NC_E_INVALID_ARG
                | NC_E_ABI_VERSION
                | NC_E_OUT_OF_RANGE
                | NC_E_NOT_FOUND
                | NC_E_INSUFFICIENT_BUFFER
                | NC_E_LOAD_FAILED
                | NC_E_OUT_OF_MEMORY
                | NC_E_BUSY
                | NC_E_ALREADY_BOUND
                | NC_E_CLOSED
                | NC_E_DROPPED
        )
    }

    /// Returns the symbolic name for known statuses.
    pub const fn name(self) -> &'static str {
        match self {
            NC_OK => "NC_OK",
            NC_E_NOINTERFACE => "NC_E_NOINTERFACE",
            NC_E_POINTER => "NC_E_POINTER",
            NC_E_INVALID_TAG => "NC_E_INVALID_TAG",
            NC_E_INVALID_ARG => "NC_E_INVALID_ARG",
            NC_E_ABI_VERSION => "NC_E_ABI_VERSION",
            NC_E_OUT_OF_RANGE => "NC_E_OUT_OF_RANGE",
            NC_E_NOT_FOUND => "NC_E_NOT_FOUND",
            NC_E_INSUFFICIENT_BUFFER => "NC_E_INSUFFICIENT_BUFFER",
            NC_E_LOAD_FAILED => "NC_E_LOAD_FAILED",
            NC_E_OUT_OF_MEMORY => "NC_E_OUT_OF_MEMORY",
            NC_E_BUSY => "NC_E_BUSY",
            NC_E_ALREADY_BOUND => "NC_E_ALREADY_BOUND",
            NC_E_CLOSED => "NC_E_CLOSED",
            NC_E_DROPPED => "NC_E_DROPPED",
            _ => "NC_STATUS_UNKNOWN",
        }
    }
}

impl core::fmt::Display for NcStatus {
    /// Formats the symbolic status name followed by its raw numeric code.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}({})", self.name(), self.code())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NcStatus {}

/// Successful ABI status.
pub const NC_OK: NcStatus = NcStatus::from_code(0);

/// Returned when an object does not support the requested interface ID.
pub const NC_E_NOINTERFACE: NcStatus = NcStatus::from_code(-1);

/// Returned when a required raw pointer argument is null.
pub const NC_E_POINTER: NcStatus = NcStatus::from_code(-2);

/// Returned when a tagged ABI enum contains an unknown tag value.
pub const NC_E_INVALID_TAG: NcStatus = NcStatus::from_code(-3);

/// Returned when a value is well-formed at the pointer level but invalid for the
/// requested operation.
pub const NC_E_INVALID_ARG: NcStatus = NcStatus::from_code(-4);

/// Returned when a shared library exposes an incompatible `neocogi-com` ABI version
/// or an object supports an interface ID with a different ABI signature.
pub const NC_E_ABI_VERSION: NcStatus = NcStatus::from_code(-5);

/// Returned when an index or length is outside its valid range or a fixed
/// runtime resource capacity is exhausted.
pub const NC_E_OUT_OF_RANGE: NcStatus = NcStatus::from_code(-6);

/// Returned when a requested registry entity does not exist.
pub const NC_E_NOT_FOUND: NcStatus = NcStatus::from_code(-7);

/// Returned when caller-owned output storage is too small for the current result.
pub const NC_E_INSUFFICIENT_BUFFER: NcStatus = NcStatus::from_code(-8);

/// Returned when the platform loader cannot load or inspect a plugin library.
pub const NC_E_LOAD_FAILED: NcStatus = NcStatus::from_code(-9);

/// Returned when a valid, policy-compliant allocation request cannot be
/// satisfied by the current allocator.
pub const NC_E_OUT_OF_MEMORY: NcStatus = NcStatus::from_code(-10);

/// Returned when a call requires exclusive object access but another call is active.
pub const NC_E_BUSY: NcStatus = NcStatus::from_code(-11);

/// A single-consumer capability already has its lifetime destination.
pub const NC_E_ALREADY_BOUND: NcStatus = NcStatus::from_code(-12);

/// A service, session or channel has closed and rejects this operation.
/// The COM object representing that capability may still exist.
pub const NC_E_CLOSED: NcStatus = NcStatus::from_code(-13);

/// The target has no published live owner; new access or promotion is rejected.
/// Usually ownership ended through Drop. A cyclic-construction weak reference
/// also has no callable owner until publication; it reports the same status.
/// Weak bookkeeping may remain allocated. Concurrent calls admitted before Drop
/// may still be finishing while the dropping thread waits for destruction.
pub const NC_E_DROPPED: NcStatus = NcStatus::from_code(-14);

#[cfg(test)]
mod tests {
    use super::{
        NC_E_ABI_VERSION, NC_E_BUSY, NC_E_INSUFFICIENT_BUFFER, NC_E_INVALID_ARG, NC_E_INVALID_TAG, NC_E_LOAD_FAILED, NC_E_NOINTERFACE, NC_E_NOT_FOUND,
        NC_E_OUT_OF_MEMORY, NC_E_OUT_OF_RANGE, NC_E_POINTER, NC_OK, NcStatus,
    };

    #[test]
    fn builtin_status_codes_are_stable() {
        assert_eq!(NC_OK.code(), 0);
        assert_eq!(NC_E_NOINTERFACE.code(), -1);
        assert_eq!(NC_E_POINTER.code(), -2);
        assert_eq!(NC_E_INVALID_TAG.code(), -3);
        assert_eq!(NC_E_INVALID_ARG.code(), -4);
        assert_eq!(NC_E_ABI_VERSION.code(), -5);
        assert_eq!(NC_E_OUT_OF_RANGE.code(), -6);
        assert_eq!(NC_E_NOT_FOUND.code(), -7);
        assert_eq!(NC_E_INSUFFICIENT_BUFFER.code(), -8);
        assert_eq!(NC_E_LOAD_FAILED.code(), -9);
        assert_eq!(NC_E_OUT_OF_MEMORY.code(), -10);
        assert_eq!(NC_E_BUSY.code(), -11);
        // These codes express distinct contracts across independently built COM callers.
        for (status, code, name) in [
            (super::NC_E_CLOSED, -13, "NC_E_CLOSED"),
            (super::NC_E_DROPPED, -14, "NC_E_DROPPED"),
        ] {
            assert_eq!(status.code(), code);
            assert_eq!(status.name(), name);
            assert!(status.is_known());
            assert_eq!(status.normalize_error(), status);
        }
    }

    #[test]
    fn result_error_normalization_preserves_errors_and_rejects_successes() {
        let unknown_error = NcStatus::from_code(-77);

        assert_eq!(unknown_error.normalize_error(), unknown_error);
        assert_eq!(NC_OK.normalize_error(), NC_E_INVALID_ARG);
        assert_eq!(NcStatus::from_code(1).normalize_error(), NC_E_INVALID_ARG);
    }
}
