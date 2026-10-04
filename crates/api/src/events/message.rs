//! A single owned envelope for any concrete IDL event interface.
use super::{EventOrigin, IEventAbi};
use crate::{ComBox, ComInterface, NcStatus, NcValueRef, WeakComBox, WeakComPtr, NC_E_NOINTERFACE};

/// Captured immutable fact and the producer's non-owning identity.
/// The event remains readable after origin removal; the consumer decides staleness.
#[derive(Debug)]
pub struct EventMessage {
    /// Distinguishes exclusive widget ownership from shared service lifetime.
    pub origin: EventOrigin,
    /// Owns only the event snapshot, never its emitting widget or application.
    pub event: ComBox<IEventAbi>,
}
impl EventMessage {
    /// Captures the weak identity before committing the sole event owner.
    /// Failed validation returns that same owner unchanged to the producer.
    pub fn capture(origin: NcValueRef<'_, EventOrigin>, event: ComBox<IEventAbi>) -> Result<Self, (NcStatus, ComBox<IEventAbi>)> {
        match origin.to_owned() {
            Ok(origin) => Ok(Self { origin, event }),
            Err(status) => Err((status, event)),
        }
    }

    /// Queries a concrete event interface; an unsupported type is ordinary absence.
    /// ABI mismatches and provider failures remain errors, rather than being hidden.
    /// The projection observes this message's event; it cannot become another owner.
    /// Its methods return Dropped after the event owner is destroyed.
    pub fn downcast<T: ComInterface>(&self) -> Result<Option<WeakComBox<T>>, NcStatus> {
        optional(self.event.query_weak())
    }
}
impl EventOrigin {
    /// Selects an exclusive widget interface without requiring the widget to survive.
    pub fn widget<T: ComInterface>(&self) -> Result<Option<WeakComBox<T>>, NcStatus> {
        match self {
            Self::Widget(origin) => optional(origin.query()),
            Self::Service(_) => Ok(None),
        }
    }

    /// Selects a shared service interface without temporarily extending its lifetime.
    pub fn service<T: ComInterface>(&self) -> Result<Option<WeakComPtr<T>>, NcStatus> {
        match self {
            Self::Service(origin) => optional(origin.query()),
            Self::Widget(_) => Ok(None),
        }
    }
}

/// Only a missing capability maps to None; all other COM failures stay visible.
fn optional<T>(result: Result<T, NcStatus>) -> Result<Option<T>, NcStatus> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(NC_E_NOINTERFACE) => Ok(None),
        Err(status) => Err(status),
    }
}
