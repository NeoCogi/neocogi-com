//! Construction-bound shared-service delivery with explicit terminal closure.
use super::{EventOrigin, IEventAbi, IEventReceiverAbi};
use crate::{ComBox, ComInterface, ComPtr, NcStatus, NcValueRef, WeakComPtr, NC_E_CLOSED, NC_E_DROPPED};

/// Delivery changes only from its constructor-selected policy to terminal closure.
enum Phase {
    /// No receiver exists and one can never be installed later.
    Unobserved,
    /// The weak origin owns no producer value; delivery temporarily pins shared lifetime.
    Observed {
        /// Canonical identity captured once, without duplicate typed weak storage.
        origin: EventOrigin,
        /// The strong receiver capability released by terminal closure.
        queue: ComPtr<IEventReceiverAbi>,
    },
    /// Logical teardown permanently rejects emission and retains no receiver or source.
    Closed,
}
/// Producer-owned delivery state for a shared service, independent of event type.
/// Factories can initially allocate an object with an unobserved port, obtain
/// its weak identity, then replace the port with `with_queue` before exposing
/// the object. The factory-selected destination remains fixed afterward.
pub struct EventPort {
    /// Sole authority for this producer's immutable delivery policy and closure.
    phase: Phase,
}
impl Default for EventPort {
    fn default() -> Self {
        Self::new()
    }
}
impl EventPort {
    /// Selects permanent non-delivery; closed ports remain distinguishable.
    pub const fn new() -> Self {
        Self { phase: Phase::Unobserved }
    }
    /// Fixes the common receiver and weak public identity before publication.
    pub fn with_queue<C: ComInterface>(queue: Option<ComPtr<IEventReceiverAbi>>, source: WeakComPtr<C, C::Threading>) -> Self {
        Self {
            phase: match queue {
                Some(queue) => Phase::Observed {
                    origin: EventOrigin::Service(source.into_unknown().into_thread_local()),
                    queue,
                },
                None => Phase::Unobserved,
            },
        }
    }
    /// Allows a producer to avoid allocating snapshots for unobserved services.
    pub fn is_bound(&self) -> bool {
        matches!(self.phase, Phase::Observed { .. })
    }
    /// Transfers the exclusive event; failed delivery destroys the returned snapshot.
    /// Committed producer state is never rolled back by a delivery error.
    pub fn post(&mut self, event: ComBox<IEventAbi>) -> Result<(), NcStatus> {
        match &mut self.phase {
            Phase::Unobserved => Ok(()),
            Phase::Observed { origin, queue } => {
                let EventOrigin::Service(source) = origin else {
                    unreachable!("service port origin")
                };
                let _pin = source.upgrade()?.ok_or(NC_E_DROPPED)?;
                queue.post(NcValueRef::new(origin)?, event).map_err(|(status, _event)| status)
            }
            Phase::Closed => Err(NC_E_CLOSED),
        }
    }
    /// Releases receiver and weak origin once; closure is terminal and idempotent.
    pub fn close(&mut self) {
        self.phase = Phase::Closed;
    }
}
