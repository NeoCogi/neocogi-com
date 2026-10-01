//! One construction-bound event queue per producer, with a typed weak origin.
//!
//! Applications may implement several generated queue interfaces on one concrete
//! COM object containing their channel sender. Factories accept that concrete
//! interface; neither IDL nor COM generates or stores an application mapping closure.
use crate::{ComInterface, ComPtr, ComRef, ComValue, NcStatus, ThreadLocal, WeakComPtr, NC_E_CLOSED, NC_E_DROPPED};

/// Rust access to one closed IDL `IEventQueue<Object, Event>` contract.
/// Implementing the generated COM trait directly avoids callback adapters entirely.
pub trait EventQueueInterface: ComInterface<Threading = ThreadLocal> + Sized + 'static {
    /// Exact public producer projection borrowed during enqueueing.
    type Object: ComInterface + Sized + 'static;
    /// Complete event acquired before its producer's call-scoped storage expires.
    type Event: ComValue + 'static;
    /// Calls the receiver with a borrowed origin and checked borrowed payload.
    fn post(
        destination: ComRef<'_, Self>,
        origin: ComRef<'_, Self::Object, <Self::Object as ComInterface>::Threading>,
        event: &Self::Event,
    ) -> Result<(), NcStatus>;
}

/// Constructor-derived association; no event interface or attach method exists on the producer.
pub trait EventSource: ComInterface + Sized + 'static {
    /// Exactly one queue contract for this product's complete event enum.
    type Queue: EventQueueInterface<Object = Self>;
}
/// Complete event type of a product's constructor queue.
pub type EventOf<C> = <<C as EventSource>::Queue as EventQueueInterface>::Event;
/// Exact receiver accepted by that product's constructors.
pub type QueueOf<C> = <C as EventSource>::Queue;

/// Delivery changes only from a constructor-selected open policy to terminal closure.
enum Phase<Q: EventQueueInterface> {
    /// No receiver exists and one can never be installed later.
    Unobserved,
    /// The public producer is weak, so storing its own origin cannot create a cycle.
    Observed {
        /// Public identity fixed before publication; never owns the producer value.
        origin: WeakComPtr<Q::Object, <Q::Object as ComInterface>::Threading>,
        /// The one strong receiver capability released by terminal closure.
        queue: ComPtr<Q>,
    },
    /// Logical teardown permanently rejects emission and retains no queue or origin.
    Closed,
}
/// Producer-local storage; mutable operations require exclusive access.
/// A COM clone cannot create a second consumer or replace the constructor's queue.
pub struct EventPort<Q: EventQueueInterface> {
    /// Sole delivery and lifecycle authority for this producer stream.
    phase: Phase<Q>,
}
impl<Q: EventQueueInterface> Default for EventPort<Q> {
    /// An unobserved producer needs no queue or weak allocation reference.
    fn default() -> Self {
        Self::new()
    }
}
impl<Q: EventQueueInterface> EventPort<Q> {
    /// Permanently disables delivery, including for unpublished designer previews.
    pub const fn new() -> Self {
        Self { phase: Phase::Unobserved }
    }
    /// Fixes both queue and public origin before publishing the producer.
    /// Use the COM class's cyclic initializer to acquire the unpublished weak origin.
    pub fn with_queue(queue: Option<ComPtr<Q>>, origin: WeakComPtr<Q::Object, <Q::Object as ComInterface>::Threading>) -> Self {
        let phase = match queue {
            Some(queue) => Phase::Observed { origin, queue },
            None => Phase::Unobserved,
        };
        Self { phase }
    }
    /// Reports whether a live consumer needs a captured snapshot.
    pub fn is_bound(&self) -> bool {
        matches!(self.phase, Phase::Observed { .. })
    }
    /// Pins the public origin only for the synchronous enqueue call.
    /// Queues acquire weak origins and owned payloads before returning; handlers
    /// run later, after the producer's exclusive COM call gate is released.
    pub fn post(&mut self, event: &Q::Event) -> Result<(), NcStatus> {
        match &mut self.phase {
            Phase::Unobserved => Ok(()),
            Phase::Observed { origin, queue } => {
                let origin = origin.upgrade()?.ok_or(NC_E_DROPPED)?;
                Q::post(queue.as_com_ref(), origin.as_com_ref(), event)
            }
            Phase::Closed => Err(NC_E_CLOSED),
        }
    }
    /// Drops the queue and weak origin exactly once; subsequent close calls are harmless.
    pub fn close(&mut self) {
        self.phase = Phase::Closed;
    }
}
