//! Typed, single-consumer stream ownership independent of widgets and queues.
//!
//! Only the generated sink interface crosses COM. The behavioral parent owns
//! the receiving queue and supplies an adapter that maps E into its own message.
//! A stream never retains the parent's Rust model or allows receiver replacement.

use crate::{ComInterface, ComValue, ComPtr, ComRef, NcStatus, ThreadLocal, NC_E_CLOSED};

/// Operations supplied by a closed IDL `IMessageSink<T>` specialization.
pub trait MessageSinkInterface: ComInterface<Threading = ThreadLocal> + Sized + 'static {
    /// The concrete owned value acquired before the foreign call returns.
    type Message: ComValue + 'static;
    /// Posts one borrowed value through the specialization's checked ABI.
    fn post(destination: ComRef<'_, Self>, message: &Self::Message) -> Result<(), NcStatus>;
    /// Allocates an application-local receiver holding only this typed callback.
    fn receiver<F: FnMut(Self::Message) -> Result<(), NcStatus> + 'static>(consume: F) -> Result<ComPtr<Self>, NcStatus>;
}

/// A constructed producer whose factory accepts exactly one typed event receiver.
/// This is generated Rust binding metadata, not another COM projection or a
/// capability to install a receiver after construction.
pub trait EventSource: ComInterface<Threading = ThreadLocal> + Sized + 'static {
    /// Exact constructor receiver associated with the producer's complete event enum.
    type Sink: MessageSinkInterface;
}

/// Captured event type of a concrete producer, without an additional wrapper enum.
pub type EventOf<C> = <<C as EventSource>::Sink as MessageSinkInterface>::Message;

/// Exact typed receiver used when constructing a concrete producer.
pub type SinkOf<C> = <C as EventSource>::Sink;

/// Irreversible stream lifetime; COM reference clones never create another slot.
enum Phase<S: MessageSinkInterface> {
    /// Construction selected its final receiver; None permanently suppresses delivery.
    Open(Option<ComPtr<S>>),
    /// Logical teardown permanently rejects emission; no installation operation exists.
    Closed,
}

/// Private producer storage for one stream. All mutation uses exclusive access.
pub struct EventPort<S: MessageSinkInterface> {
    /// One state and receiver shared by every projection of the producer object.
    phase: Phase<S>,
}

impl<S: MessageSinkInterface> Default for EventPort<S> {
    /// Creates a permanently unobserved producer without a receiver allocation.
    fn default() -> Self {
        Self::new()
    }
}

impl<S: MessageSinkInterface> EventPort<S> {
    /// Creates a permanently unobserved producer; no later installation operation exists.
    pub const fn new() -> Self {
        Self { phase: Phase::Open(None) }
    }

    /// Constructs the final delivery policy before the producer is published.
    /// A missing sink is permanently unobserved, never an available attach slot.
    pub fn with_sink(sink: Option<ComPtr<S>>) -> Self {
        Self { phase: Phase::Open(sink) }
    }

    /// Reports whether snapshots need to be constructed for a live consumer.
    pub fn is_bound(&self) -> bool {
        matches!(self.phase, Phase::Open(Some(_)))
    }

    /// Posts a call-scoped snapshot. Unobserved previews intentionally discard events.
    /// Receiver decoding owns all variable data before the producer can release it.
    pub fn post(&mut self, event: &S::Message) -> Result<(), NcStatus> {
        match &mut self.phase {
            Phase::Open(None) => Ok(()),
            Phase::Open(Some(receiver)) => S::post(receiver.as_com_ref(), event),
            Phase::Closed => Err(NC_E_CLOSED),
        }
    }

    /// Drops the receiver and permanently retires the stream; repeated closure is harmless.
    pub fn close(&mut self) {
        self.phase = Phase::Closed;
    }
}

/// Maps typed events into an application-local queue before producer creation.
/// The resulting COM object owns the sender and closure; neither Rust value nor
/// the application message type crosses the factory ABI. Failed construction
/// drops ordinary COM references and releases captures through their owner.
#[cfg(feature = "std")]
pub fn mapped_sink<S: MessageSinkInterface, M: 'static>(
    sender: std::sync::mpsc::Sender<M>,
    mut map: impl FnMut(S::Message) -> M + 'static,
) -> Result<ComPtr<S>, NcStatus> {
    S::receiver(move |event| {
        // A disconnected application inbox does not undo a committed UI edit.
        let _ = sender.send(map(event));
        Ok(())
    })
}
