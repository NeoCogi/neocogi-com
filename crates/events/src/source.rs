//
// Copyright 2023-Present (c) Raja Lehtihet & Wael El Oraiby
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice,
// this list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice,
// this list of conditions and the following disclaimer in the documentation
// and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its contributors
// may be used to endorse or promote products derived from this software without
// specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE
// LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
// CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
// SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
// INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
// CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
// ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
// POSSIBILITY OF SUCH DAMAGE.

//! Exclusive port connections; messages are stored only in consumer inboxes.

use std::{
    cell::RefCell,
    fmt,
    rc::{Rc, Weak},
};
use neocogi_com::{com_object, ComInterface, ComPtr, ComRef, NcStatus};
use neocogi_com_events_contracts::*;
use crate::{EventPort, EventSender, EVENT_E_BUSY, EVENT_E_CLOSED};

/// Concrete protocol snapshot and its generated foreign receiver operations.
pub trait EventPayload: 'static {
    /// The exact COM protocol used across plugin boundaries.
    type Sink: ComInterface;
    /// Encodes an owned snapshot for immediate translation into the consumer's inbox.
    fn deliver(&self, sink: ComRef<'_, Self::Sink>) -> Result<(), NcStatus>;
    /// Builds the concrete receiver without storing application state in the producer.
    fn receiver<Message: 'static, Map: FnMut(Self) -> Message + 'static>(sender: EventSender<Message>, map: Map) -> Result<ComPtr<Self::Sink>, NcStatus>
    where
        Self: Sized;
}

/// Producer-owned connection point with no queue and no retained consumer ownership.
/// Emission runs only the mapper; the runtime invokes consumer behavior later.
/// Mappers must not perform UI work: their producer may still be inside an update.
pub struct EventSource<E: EventPayload> {
    /// Endpoints share only connection state, never the producer's semantic model.
    port: Rc<RefCell<Port<E::Sink>>>,
}
/// Connection availability is represented by one exhaustive lifecycle state.
enum Port<Sink: ComInterface> {
    /// The lease owns the receiver; a producer holds only this weak link.
    Open(Weak<Connection<Sink>>),
    /// Logical closure rejects replacement subscriptions and future emissions.
    Closed,
}
impl<E: EventPayload> EventSource<E> {
    /// Creates a live, disconnected port without allocating any message storage.
    pub fn new() -> Self {
        Self {
            port: Rc::new(RefCell::new(Port::Open(Weak::new()))),
        }
    }
    /// Reports whether a current lease has an active receiver.
    pub fn connected(&self) -> bool {
        self.connection().is_some_and(|value| value.active())
    }
    /// Obtains the receiver owner outside the short connection-state borrow.
    fn connection(&self) -> Option<Rc<Connection<E::Sink>>> {
        match &*self.port.borrow() {
            Port::Open(connection) => connection.upgrade(),
            Port::Closed => None,
        }
    }
    /// Returns a weak source endpoint without emission or producer ownership.
    pub fn event(&self) -> EventEndpoint<E> {
        EventEndpoint(Rc::downgrade(&self.port))
    }
    /// Posts a committed snapshot directly through the typed receiver into its consumer inbox.
    /// No subscriber means no retained history. Foreign decoding/mapping failures are returned.
    pub fn emit(&self, event: E) -> Result<(), NcStatus> {
        match self.connection() {
            Some(connection) => connection.post(&event),
            None => Ok(()),
        }
    }
    /// Closes future posting without retracting messages already owned by a consumer.
    pub fn close_ordinary(&self) {
        let connection = self.connection();
        *self.port.borrow_mut() = Port::Closed;
        if let Some(connection) = connection {
            connection.cancel();
        }
    }
    /// Posts one final fact after revoking the source, then releases the receiver on every outcome.
    pub fn close(&self, terminal: E) -> Result<(), NcStatus> {
        let connection = self.connection();
        *self.port.borrow_mut() = Port::Closed;
        match connection {
            Some(connection) => {
                let result = connection.post(&terminal);
                connection.cancel();
                result
            }
            None => Ok(()),
        }
    }
    /// Connects the generated protocol receiver and returns its sole lease owner.
    pub fn subscribe(&self, sink: ComRef<'_, E::Sink>) -> Result<ComPtr<IEventSubscriptionAbi>, NcStatus> {
        self.event().subscribe(sink)
    }
}
impl<E: EventPayload> Default for EventSource<E> {
    /// Uses the same live disconnected state as `new`.
    fn default() -> Self {
        Self::new()
    }
}
impl<E: EventPayload> Drop for EventSource<E> {
    /// Drops the receiver without erasing already posted consumer messages.
    fn drop(&mut self) {
        self.close_ordinary();
    }
}

/// Receiver ownership moves out of the slot while foreign code runs.
enum Receiver<Sink: ComInterface> {
    /// The connected receiver can accept one synchronous protocol call.
    Ready(ComPtr<Sink>),
    /// One call owns the receiver outside the cell; recursive posting is rejected.
    Posting,
    /// Cancellation is permanent for this lease, independently of replacement connections.
    Disconnected,
}
/// Native lease state lives beside the producer but owns only the foreign receiver.
struct Connection<Sink: ComInterface> {
    /// Interior mutability is confined to the shared COM cancellation/posting boundary.
    receiver: RefCell<Receiver<Sink>>,
}
impl<Sink: ComInterface> Connection<Sink> {
    /// A receiver being called is still connected until explicitly cancelled.
    fn active(&self) -> bool {
        !matches!(*self.receiver.borrow(), Receiver::Disconnected)
    }
    /// Releases receiver ownership after ending the slot borrow, permitting destructor reentry.
    fn cancel(&self) {
        let retired = self.receiver.replace(Receiver::Disconnected);
        drop(retired);
    }
    /// Invokes the exact protocol without carrying a source or receiver-cell borrow across it.
    fn post<E: EventPayload<Sink = Sink>>(&self, event: &E) -> Result<(), NcStatus> {
        let receiver = {
            let mut slot = self.receiver.borrow_mut();
            match &*slot {
                Receiver::Posting => return Err(EVENT_E_BUSY),
                Receiver::Disconnected => return Ok(()),
                Receiver::Ready(_) => match std::mem::replace(&mut *slot, Receiver::Posting) {
                    Receiver::Ready(receiver) => receiver,
                    _ => unreachable!("exclusive slot still contains its matched receiver"),
                },
            }
        };
        let access = Posting {
            connection: self,
            receiver: Some(receiver),
        };
        event.deliver(access.receiver.as_ref().expect("active posting owns its receiver").as_com_ref())
    }
}
/// Restores a live receiver after posting, or releases it if cancellation occurred during the call.
struct Posting<'a, Sink: ComInterface> {
    /// Connection remains alive through the posting operation.
    connection: &'a Connection<Sink>,
    /// Sole receiver reference, temporarily moved out to avoid foreign calls under a borrow.
    receiver: Option<ComPtr<Sink>>,
}
impl<Sink: ComInterface> Drop for Posting<'_, Sink> {
    /// Restoration runs on both returned errors and unwinding.
    fn drop(&mut self) {
        let mut slot = self.connection.receiver.borrow_mut();
        if matches!(*slot, Receiver::Posting) {
            *slot = Receiver::Ready(self.receiver.take().expect("posting owns receiver"));
        }
    }
}

#[com_object(interfaces(IEventSubscription))]
/// Retainable lease owns one receiver; it has no queue or dispatch operation.
struct Subscription<Sink: ComInterface> {
    /// A replacement lease always gets a separate cancellation identity.
    connection: Rc<Connection<Sink>>,
}
impl<Sink: ComInterface> IEventSubscription for Subscription<Sink> {
    /// Reports whether this exact connection still accepts new facts.
    fn active(&self) -> Result<bool, NcStatus> {
        Ok(self.connection.active())
    }
    /// Disconnects future emission; committed consumer messages remain valid.
    fn cancel(&self) -> Result<(), NcStatus> {
        self.connection.cancel();
        Ok(())
    }
}

/// Weak typed capability identifying a producer-owned connection point.
pub struct EventEndpoint<E: EventPayload>(Weak<RefCell<Port<E::Sink>>>);
impl<E: EventPayload> EventEndpoint<E> {
    /// Represents an unavailable producer without retaining any substitute object.
    pub fn expired() -> Self {
        Self(Weak::new())
    }
    /// Observes logical liveness without claiming subscription ownership.
    pub fn is_alive(&self) -> bool {
        self.0.upgrade().is_some_and(|port| matches!(*port.borrow(), Port::Open(_)))
    }
    /// Claims one exclusive connection, constructing foreign ownership before borrowing the slot.
    pub fn subscribe(&self, sink: ComRef<'_, E::Sink>) -> Result<ComPtr<IEventSubscriptionAbi>, NcStatus> {
        let port = self.0.upgrade().ok_or(EVENT_E_CLOSED)?;
        let connection = Rc::new(Connection {
            receiver: RefCell::new(Receiver::Ready(sink.add_ref())),
        });
        {
            let mut slot = port.borrow_mut();
            match &*slot {
                Port::Closed => return Err(EVENT_E_CLOSED),
                Port::Open(previous) if previous.upgrade().is_some_and(|value| value.active()) => return Err(EVENT_E_BUSY),
                Port::Open(_) => *slot = Port::Open(Rc::downgrade(&connection)),
            }
        }
        Subscription::from_value(Subscription { connection }).query()
    }
}
impl<E: EventPayload> EventPort for EventEndpoint<E> {
    type Event = E;
    /// Uses the same generated receiver as foreign source projections.
    fn connect<Message: 'static, Map: FnMut(E) -> Message + 'static>(
        self,
        sender: EventSender<Message>,
        map: Map,
    ) -> Result<ComPtr<IEventSubscriptionAbi>, NcStatus> {
        self.subscribe(E::receiver(sender, map)?.as_com_ref())
    }
}
impl<E: EventPayload> Clone for EventEndpoint<E> {
    /// Copies weak connection access without extending producer lifetime.
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<E: EventPayload> fmt::Debug for EventEndpoint<E> {
    /// Reports liveness without exposing receiver or queue internals.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventEndpoint").field("alive", &self.is_alive()).finish_non_exhaustive()
    }
}
