//! Construction-bound common receivers and shared-service delivery lifetime.
use super::{EventOrigin, EventPayload, IEventReceiverAbi};
use crate::{ComInterface, ComPtr, ComValue, NcStatus, NcValueRef, WeakComPtr, NC_E_CLOSED, NC_E_DROPPED};

/// Constructor-derived association with an optional native event representation.
/// The wire transport always transfers a `ComBox<IEventAbi>` to `IEventReceiver`.
pub trait EventSource: ComInterface + Sized + 'static {
    /// Complete native payload, useful to compiled handlers and authoring metadata.
    type Event: ComValue + 'static;
}
/// Native event representation associated with a producer's constructors.
pub type EventOf<C> = <C as EventSource>::Event;

/// Delivery changes only from a constructor-selected open policy to terminal closure.
enum Phase<C: EventSource> {
    /// No receiver exists and one can never be installed later.
    Unobserved,
    /// Neither the typed observer nor the captured origin owns the producer value.
    Observed {
        /// Typed observer preserves the shared-service liveness check before posting.
        source: WeakComPtr<C, C::Threading>,
        /// Cached wire identity avoids a query on every emitted event.
        origin: EventOrigin,
        /// The one strong receiver capability released by terminal closure.
        queue: ComPtr<IEventReceiverAbi>,
    },
    /// Logical teardown permanently rejects emission and retains no queue or origin.
    Closed,
}
/// Producer-local state for shared services. Widgets store exclusive weak origins.
pub struct EventPort<C: EventSource> {
    /// Sole delivery and lifecycle authority for this producer stream.
    phase: Phase<C>,
}
impl<C: EventSource> Default for EventPort<C> {
    fn default() -> Self {
        Self::new()
    }
}
impl<C: EventSource> EventPort<C> {
    /// Permanently disables delivery, including for unpublished designer previews.
    pub const fn new() -> Self {
        Self { phase: Phase::Unobserved }
    }
    /// Fixes the common receiver and public origin before publishing the producer.
    pub fn with_queue(queue: Option<ComPtr<IEventReceiverAbi>>, source: WeakComPtr<C, C::Threading>) -> Self {
        let phase = match queue {
            Some(queue) => {
                let origin = EventOrigin::Service(source.clone().into_unknown().into_thread_local());
                Phase::Observed { source, origin, queue }
            }
            None => Phase::Unobserved,
        };
        Self { phase }
    }
    /// Reports whether a consumer needs a captured snapshot.
    pub fn is_bound(&self) -> bool {
        matches!(self.phase, Phase::Observed { .. })
    }
    /// Captures one exclusive event and transfers it to the receiver.
    /// A failed post drops the restored snapshot; already committed UI state is unchanged.
    pub fn post(&mut self, event: C::Event) -> Result<(), NcStatus>
    where
        C::Event: EventPayload,
    {
        match &mut self.phase {
            Phase::Unobserved => Ok(()),
            Phase::Observed { source, origin, queue } => {
                let _pin = source.upgrade()?.ok_or(NC_E_DROPPED)?;
                let origin = NcValueRef::new(origin)?;
                queue.post(origin, event.into_event()?).map_err(|(status, _event)| status)
            }
            Phase::Closed => Err(NC_E_CLOSED),
        }
    }
    /// Drops the receiver and weak origin exactly once; repeated close is harmless.
    pub fn close(&mut self) {
        self.phase = Phase::Closed;
    }
}
