//! Typed, single-consumer stream ownership independent of widgets and queues.
//!
//! Only the generated sink interface crosses COM. The behavioral parent owns
//! the receiving queue and supplies an adapter that maps E into its own message.
//! A stream never retains the parent's Rust model or allows receiver replacement.

use crate::{ComInterface, ComValue, ComPtr, ComRef, NcStatus, ThreadLocal, NC_E_ALREADY_BOUND, NC_E_CLOSED};

/// Operations supplied by a closed IDL `IMessageSink<T>` specialization.
pub trait MessageSinkInterface: ComInterface<Threading = ThreadLocal> + Sized + 'static {
    /// The concrete owned value acquired before the foreign call returns.
    type Message: ComValue + 'static;
    /// Posts one borrowed value through the specialization's checked ABI.
    fn post(destination: ComRef<'_, Self>, message: &Self::Message) -> Result<(), NcStatus>;
    /// Allocates an application-local receiver holding only this typed callback.
    fn receiver<F: FnMut(Self::Message) -> Result<(), NcStatus> + 'static>(consume: F) -> Result<ComPtr<Self>, NcStatus>;
}

/// Operations supplied by a closed IDL `IEventStream<E>` specialization.
pub trait EventStreamInterface: ComInterface<Threading = ThreadLocal> + Sized + 'static {
    /// Exact sink required by this stream's event type and ABI revision.
    type Sink: MessageSinkInterface;
    /// Attempts the one lifetime attachment, preserving an existing consumer on failure.
    fn attach(source: ComRef<'_, Self>, consumer: ComRef<'_, Self::Sink>) -> Result<(), NcStatus>;
}

/// A concrete class or interface whose resolved projection set has exactly one stream.
pub trait EventSource: ComInterface<Threading = ThreadLocal> + Sized + 'static {
    /// The complete typed stream, including a stream inherited through another interface.
    type Stream: EventStreamInterface;
}

/// Captured event type of one concrete source, without an additional wrapper enum.
pub type EventOf<C> = <<<C as EventSource>::Stream as EventStreamInterface>::Sink as MessageSinkInterface>::Message;

/// Irreversible stream lifetime; COM reference clones never create another slot.
enum Phase<S: MessageSinkInterface> {
    /// The behavioral parent has not completed construction-time wiring yet.
    Unbound,
    /// One parent-owned receiver consumes every event variant.
    Bound(ComPtr<S>),
    /// Logical teardown permanently rejects future attachment and emission.
    Closed,
}

/// Private producer storage for one stream. All mutation uses exclusive access.
pub struct EventPort<S: MessageSinkInterface> {
    /// One state and receiver shared by every projection of the producer object.
    phase: Phase<S>,
}

impl<S: MessageSinkInterface> Default for EventPort<S> {
    /// Creates an unbound stream for factory initialization before publication.
    fn default() -> Self {
        Self::new()
    }
}

impl<S: MessageSinkInterface> EventPort<S> {
    /// Creates an unbound stream without allocating a queue or receiver.
    pub const fn new() -> Self {
        Self { phase: Phase::Unbound }
    }

    /// Attaches once. Failure leaves the original consumer and queued messages intact.
    pub fn attach(&mut self, consumer: ComRef<'_, S>) -> Result<(), NcStatus> {
        match self.phase {
            Phase::Unbound => {
                self.phase = Phase::Bound(consumer.add_ref());
                Ok(())
            }
            Phase::Bound(_) => Err(NC_E_ALREADY_BOUND),
            Phase::Closed => Err(NC_E_CLOSED),
        }
    }

    /// Reports whether snapshots need to be constructed for a live consumer.
    pub fn is_bound(&self) -> bool {
        matches!(self.phase, Phase::Bound(_))
    }

    /// Posts a call-scoped snapshot. Unobserved previews intentionally discard events.
    /// Receiver decoding owns all variable data before the producer can release it.
    pub fn post(&mut self, event: &S::Message) -> Result<(), NcStatus> {
        match &mut self.phase {
            Phase::Unbound => Ok(()),
            Phase::Bound(receiver) => S::post(receiver.as_com_ref(), event),
            Phase::Closed => Err(NC_E_CLOSED),
        }
    }

    /// Drops the receiver and permanently retires the stream; repeated closure is harmless.
    pub fn close(&mut self) {
        self.phase = Phase::Closed;
    }
}

/// Maps a concrete stream into the behavioral parent's owned application queue.
/// Mapping captures data immediately; application behavior runs when its inbox drains.
#[cfg(feature = "std")]
pub fn bind_stream<S: EventStreamInterface, M: 'static>(
    stream: ComRef<'_, S>,
    sender: std::sync::mpsc::Sender<M>,
    mut map: impl FnMut(<S::Sink as MessageSinkInterface>::Message) -> M + 'static,
) -> Result<(), NcStatus> {
    let sink = S::Sink::receiver(move |event| {
        // Parent retirement disconnects its inbox. The widget must not turn an
        // already committed user edit into a failure merely because it closed.
        let _ = sender.send(map(event));
        Ok(())
    })?;
    S::attach(stream, sink.as_com_ref())
}

/// Queries a concrete object's sole stream and wires it to the behavioral parent.
/// The source and receiver are negotiated once during form construction.
#[cfg(feature = "std")]
pub fn bind_source<C: EventSource, M: 'static>(
    source: ComRef<'_, C>,
    sender: std::sync::mpsc::Sender<M>,
    map: impl FnMut(EventOf<C>) -> M + 'static,
) -> Result<(), NcStatus> {
    let stream = source.add_ref().query::<C::Stream>()?;
    bind_stream(stream.as_com_ref(), sender, map)
}

/// Exact receiver interface associated with a concrete event source.
pub type SinkOf<C> = <<C as EventSource>::Stream as EventStreamInterface>::Sink;

/// Attaches a caller-owned typed sink after factory creation, before publication.
/// Querying the stream checks the same generic ABI used by independent plugins.
pub fn attach_source<C: EventSource, S: MessageSinkInterface<Message = EventOf<C>>>(source: ComRef<'_, C>, sink: ComRef<'_, S>) -> Result<(), NcStatus> {
    let stream = source.add_ref().query::<C::Stream>()?;
    // Independently generated Rust projections share COM identity, not Rust nominal type.
    let sink = sink.add_ref().query::<SinkOf<C>>()?;
    C::Stream::attach(stream.as_com_ref(), sink.as_com_ref())
}
