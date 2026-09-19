//! Typed consumer storage, independent of application state and producer lifetimes.

use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::{Rc, Weak},
};
use neocogi_com::{ComPtr, NcStatus};
use crate::{EVENT_E_CLOSED, EVENT_E_LIMIT};

/// One consumer's FIFO; application state stays in its ordinary owner.
/// Mappers run at emission time and should only translate messages or attach routing IDs.
/// Behavior runs later, when the runtime invokes the consumer at a safe update boundary.
pub struct Events<Message> {
    /// Receivers share only this short enqueue boundary, never the consumer's mutable state.
    pending: Rc<RefCell<VecDeque<Message>>>,
}

impl<Message> Events<Message> {
    /// Creates an empty, independently owned consumer FIFO.
    pub fn new() -> Self {
        Self {
            pending: Rc::new(RefCell::new(VecDeque::new())),
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
    /// Installs or replaces an output's destination. Already queued messages are unchanged.
    /// The local callback is concretely typed; only its generated receiver crosses COM.
    pub fn map<Event: 'static>(
        &mut self,
        install: impl FnOnce(Box<dyn FnMut(Event) -> Result<(), NcStatus>>) -> Result<(), NcStatus>,
        mut map: impl FnMut(Event) -> Message + 'static,
    ) -> Result<(), NcStatus> {
        let pending = Rc::downgrade(&self.pending);
        install(Box::new(move |event| {
            // An abandoned consumer is inert. Do not even evaluate its mapper,
            // and never keep the queue or application alive from the producer.
            if let Some(pending) = pending.upgrade() {
                let message = map(event);
                pending.borrow_mut().push_back(message);
            }
            Ok(())
        }))
    }
}
impl<Message> Default for Events<Message> {
    /// Uses the same empty construction as `new`.
    fn default() -> Self {
        Self::new()
    }
}
/// Typed, non-owning posting access for local application or component work.
pub struct EventSender<Message> {
    /// Weak ownership ensures senders cannot prolong an abandoned inbox.
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
    /// A busy component retains every pending message until exclusive model access is available.
    #[test]
    fn component_access_failure_does_not_consume_a_message() {
        let mut events = Events::new();
        let state = Rc::new(RefCell::new(Vec::new()));
        let mut driver = events
            .component_dispatcher(&state, |state, message| {
                state.push(message);
                Ok(())
            })
            .unwrap();
        events.sender().post(1).unwrap();
        let held = state.borrow();
        assert_eq!(driver.dispatch(), Err(crate::EVENT_E_BUSY));
        assert_eq!(events.len(), 1);
        drop(held);
        assert_eq!(driver.dispatch(), Ok(true));
        assert_eq!(&*state.borrow(), &[1]);
        assert_eq!(events.pop(), None);
        drop(events);
        assert_eq!(driver.dispatch(), Ok(false));
    }
}

/// Separate component driver automatically pumped when its root exposes IComponentEventProvider.
/// The driver retains neither the component nor its inbox, so mounting creates no ownership cycle.
#[neocogi_com::com_object(interfaces(neocogi_com_events_contracts::IEventDispatcher))]
struct ComponentDispatcher<Owner: 'static, Message: 'static> {
    /// Component ownership belongs to its widget/controller, never the mounted driver.
    owner: Weak<RefCell<Owner>>,
    /// Messages remain private to this one consumer and its generated receivers.
    pending: Weak<RefCell<VecDeque<Message>>>,
    /// Ordinary component entry point; no per-event handler type or scheduling loop is needed.
    consume: fn(&mut Owner, Message) -> Result<(), NcStatus>,
}
impl<Owner: 'static, Message: 'static> neocogi_com_events_contracts::IEventDispatcher for ComponentDispatcher<Owner, Message> {
    /// Claims the component before taking one message so a busy owner loses no work.
    fn dispatch(&mut self) -> Result<bool, NcStatus> {
        let Some(owner) = self.owner.upgrade() else { return Ok(false) };
        let mut owner = owner.try_borrow_mut().map_err(|_| crate::EVENT_E_BUSY)?;
        let message = self.pending.upgrade().and_then(|pending| pending.borrow_mut().pop_front());
        match message {
            Some(message) => {
                (self.consume)(&mut owner, message)?;
                Ok(true)
            }
            None => Ok(false),
        }
    }
}
impl<Message: 'static> Events<Message> {
    /// Creates the framework driver exposed by a composite's mounted root.
    /// The consumer entry point handles one message; the framework owns all queue traversal.
    pub fn component_dispatcher<Owner: 'static>(
        &self,
        owner: &Rc<RefCell<Owner>>,
        consume: fn(&mut Owner, Message) -> Result<(), NcStatus>,
    ) -> Result<ComPtr<crate::IEventDispatcherAbi>, NcStatus> {
        ComponentDispatcher::from_value(ComponentDispatcher {
            owner: Rc::downgrade(owner),
            pending: Rc::downgrade(&self.pending),
            consume,
        })
        .query()
    }
}
