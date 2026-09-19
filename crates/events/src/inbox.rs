//! Typed consumer storage, independent of application state and producer lifetimes.

use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::{Rc, Weak},
};
use neocogi_com::{ComPtr, NcStatus};
use crate::{IEventSubscriptionAbi, EVENT_E_CLOSED, EVENT_E_LIMIT};

/// Generated connection operation for a concrete protocol and consumer message type.
/// The temporary port retains its source only until connection returns.
pub trait EventPort {
    /// Owned protocol message decoded by the generated receiver.
    type Event;
    /// Installs a receiver that translates protocol messages directly into the consumer inbox.
    fn connect<Message: 'static, Map: FnMut(Self::Event) -> Message + 'static>(
        self,
        sender: EventSender<Message>,
        map: Map,
    ) -> Result<ComPtr<IEventSubscriptionAbi>, NcStatus>;
}

/// One consumer's FIFO and connections; application state stays in its ordinary owner.
/// Mappers run at emission time and should only translate messages or attach routing IDs.
/// Behavior runs later, when the runtime invokes the consumer at a safe update boundary.
pub struct Events<Message> {
    /// Receivers share only this short enqueue boundary, never the consumer's mutable state.
    pending: Rc<RefCell<VecDeque<Message>>>,
    /// Retaining leases keeps mappings connected even if the caller drops its returned handle.
    connections: Vec<ComPtr<IEventSubscriptionAbi>>,
}

impl<Message> Events<Message> {
    /// Creates an empty consumer with no source connections.
    pub fn new() -> Self {
        Self {
            pending: Rc::new(RefCell::new(VecDeque::new())),
            connections: Vec::new(),
        }
    }
    /// Returns non-owning posting access; retaining it cannot retain the consumer.
    pub fn sender(&self) -> EventSender<Message> {
        EventSender { pending: Rc::downgrade(&self.pending) }
    }
    /// Reads the number of committed messages waiting for this consumer.
    pub fn len(&self) -> usize {
        self.pending.borrow().len()
    }
    /// Reports whether the consumer has any pending work.
    pub fn is_empty(&self) -> bool {
        self.pending.borrow().is_empty()
    }
    /// Removes the oldest message, releasing the queue borrow before caller behavior runs.
    pub fn pop(&mut self) -> Option<Message> {
        self.pending.borrow_mut().pop_front()
    }
    /// Disconnects future posts without retracting facts already committed to this inbox.
    pub fn cancel(&mut self) -> Result<(), NcStatus> {
        for connection in &self.connections {
            connection.cancel()?;
        }
        self.connections.clear();
        Ok(())
    }
    /// Consumes a bounded prefix for a framework dispatch boundary.
    /// New messages wait for the next visit. An error consumes only the attempted message.
    #[doc(hidden)]
    pub fn dispatch(&mut self, limit: u32, mut consume: impl FnMut(Message) -> Result<(), NcStatus>) -> Result<u32, NcStatus> {
        let count = self.len().min(limit as usize);
        if count == 0 && !self.is_empty() {
            return Err(EVENT_E_LIMIT);
        }
        for _ in 0..count {
            // Posting may occur during behavior, but no borrow of the queue crosses that call.
            let message = self.pop().expect("the exclusive consumer owns the captured prefix");
            consume(message)?;
        }
        Ok(count as u32)
    }
}
impl<Message: 'static> Events<Message> {
    /// Connects a typed protocol to this inbox without capturing application state.
    pub fn map<Port: EventPort>(&mut self, port: Port, map: impl FnMut(Port::Event) -> Message + 'static) -> Result<ComPtr<IEventSubscriptionAbi>, NcStatus> {
        // Prune disconnected receivers during setup; there is no dispatch registry to sweep.
        let mut index = 0;
        while index < self.connections.len() {
            match self.connections[index].active()? {
                true => index += 1,
                false => {
                    self.connections[index].cancel()?;
                    self.connections.remove(index);
                }
            }
        }
        let connection = port.connect(self.sender(), map)?;
        self.connections.push(connection.clone());
        Ok(connection)
    }
}
impl<Message> Default for Events<Message> {
    /// Uses the same disconnected, empty construction as `new`.
    fn default() -> Self {
        Self::new()
    }
}
impl<Message> Drop for Events<Message> {
    /// Explicit cancellation also invalidates separately retained lease references.
    fn drop(&mut self) {
        for connection in &self.connections {
            let _ = connection.cancel();
        }
    }
}

/// Typed, non-owning posting access used by generated protocol receivers.
pub struct EventSender<Message> {
    /// Weak ownership ensures source connections cannot prolong an abandoned inbox.
    pending: Weak<RefCell<VecDeque<Message>>>,
}
impl<Message> EventSender<Message> {
    /// Commits a message without invoking consumer behavior or retaining a queue borrow.
    pub fn post(&self, message: Message) -> Result<(), NcStatus> {
        let pending = self.pending.upgrade().ok_or(EVENT_E_CLOSED)?;
        pending.borrow_mut().push_back(message);
        Ok(())
    }
}
impl<Message> Clone for EventSender<Message> {
    /// Copies posting access without extending consumer lifetime.
    fn clone(&self) -> Self {
        Self { pending: self.pending.clone() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// A callback may post successors; errors preserve their FIFO order with unattempted work.
    #[test]
    fn prefix_and_failure_preserve_fifo() {
        let mut events = Events::new();
        let sender = events.sender();
        sender.post(1).unwrap();
        sender.post(2).unwrap();
        assert_eq!(
            events.dispatch(1, |n| {
                assert_eq!(n, 1);
                sender.post(3)
            }),
            Ok(1)
        );
        assert_eq!(events.dispatch(0, |_| Ok(())), Err(EVENT_E_LIMIT));
        assert_eq!(events.dispatch(10, |_| Err(EVENT_E_CLOSED)), Err(EVENT_E_CLOSED));
        assert_eq!(events.pop(), Some(3));
        drop(events);
        assert_eq!(sender.post(4), Err(EVENT_E_CLOSED));
    }
}

/// Separate component driver automatically pumped when its root exposes IComponentEventProvider.
/// The driver retains neither the component nor its inbox, so mounting creates no ownership cycle.
#[neocogi_com::com_object(interfaces(neocogi_com_events_contracts::IEventDispatcher))]
struct ComponentDispatcher<Owner: 'static, Message: 'static> {
    /// Component ownership belongs to its widget/controller, never the mounted driver.
    owner: Weak<Owner>,
    /// Messages remain private to this one consumer and its generated receivers.
    pending: Weak<RefCell<VecDeque<Message>>>,
    /// Ordinary component entry point; no per-event handler type or scheduling loop is needed.
    consume: fn(&Owner, Message) -> Result<(), NcStatus>,
}
impl<Owner: 'static, Message: 'static> neocogi_com_events_contracts::IEventDispatcher for ComponentDispatcher<Owner, Message> {
    /// Visits the starting FIFO prefix without carrying a queue borrow into component behavior.
    fn dispatch(&mut self, limit: u32) -> Result<u32, NcStatus> {
        let count = match self.pending.upgrade() {
            Some(pending) => pending.borrow().len().min(limit as usize),
            None => return Ok(0),
        };
        if count == 0 && self.pending.upgrade().is_some_and(|pending| !pending.borrow().is_empty()) {
            return Err(EVENT_E_LIMIT);
        }
        let mut delivered = 0;
        for _ in 0..count {
            let Some(owner) = self.owner.upgrade() else { break };
            let message = self.pending.upgrade().and_then(|pending| pending.borrow_mut().pop_front());
            match message {
                Some(message) => {
                    (self.consume)(&owner, message)?;
                    delivered += 1;
                }
                None => break,
            }
        }
        Ok(delivered)
    }
}
impl<Message: 'static> Events<Message> {
    /// Creates the framework driver exposed by a composite's mounted root.
    /// The consumer entry point handles one message; the framework owns all queue traversal.
    pub fn component_dispatcher<Owner: 'static>(
        &self,
        owner: &Rc<Owner>,
        consume: fn(&Owner, Message) -> Result<(), NcStatus>,
    ) -> Result<ComPtr<crate::IEventDispatcherAbi>, NcStatus> {
        ComponentDispatcher::from_value(ComponentDispatcher {
            owner: Rc::downgrade(owner),
            pending: Rc::downgrade(&self.pending),
            consume,
        })
        .query()
    }
}
