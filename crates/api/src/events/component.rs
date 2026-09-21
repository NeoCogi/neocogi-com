//! Automatic dispatch into a component's ordinary state and standard receiver.

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::mpsc::{Receiver, TryRecvError},
};
use neocogi_com::{ComPtr, NcStatus};
use super::{IEventDispatcher, IEventDispatcherAbi, EVENT_E_BUSY};

/// Separate mounted driver; the component retains ownership of both state and receiver.
#[neocogi_com::com_object(interfaces(neocogi_com::events::IEventDispatcher))]
struct ComponentDispatcher<Owner: 'static, Message: 'static> {
    /// A mounted runtime cannot keep an otherwise abandoned component alive.
    owner: Weak<RefCell<Owner>>,
    /// Selects the component's standard receiver without copying or sharing its queue.
    messages: fn(&Owner) -> &Receiver<Message>,
    /// Handles one owned message after the receiver borrow has ended.
    consume: fn(&mut Owner, Message) -> Result<(), NcStatus>,
}
impl<Owner: 'static, Message: 'static> IEventDispatcher for ComponentDispatcher<Owner, Message> {
    /// Claims exclusive component access before dequeuing, preserving work when busy.
    fn dispatch(&mut self) -> Result<bool, NcStatus> {
        let Some(owner) = self.owner.upgrade() else { return Ok(false) };
        let mut owner = owner.try_borrow_mut().map_err(|_| EVENT_E_BUSY)?;
        match (self.messages)(&owner).try_recv() {
            Ok(message) => {
                (self.consume)(&mut owner, message)?;
                Ok(true)
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => Ok(false),
        }
    }
}

/// Creates the separate COM driver exposed by a composite's mounted root.
/// The runtime discovers that driver and schedules one message per visit automatically.
/// Shared model access exists only because controller and mounted driver are distinct
/// COM identities; the message queue itself needs no Rc, RefCell or custom wrapper.
pub fn component_dispatcher<Owner: 'static, Message: 'static>(
    owner: &Rc<RefCell<Owner>>,
    messages: fn(&Owner) -> &Receiver<Message>,
    consume: fn(&mut Owner, Message) -> Result<(), NcStatus>,
) -> Result<ComPtr<IEventDispatcherAbi>, NcStatus> {
    ComponentDispatcher::from_value(ComponentDispatcher {
        owner: Rc::downgrade(owner),
        messages,
        consume,
    })
    .query()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    /// A component owns the only receiver alongside its ordinary mutable state.
    struct Model {
        /// Concrete local messages never cross COM or require the Send trait.
        messages: Receiver<Rc<u32>>,
        /// Records behavior independently of channel storage.
        values: Vec<u32>,
    }

    /// A conflicting model borrow preserves messages; dropping the model disconnects senders.
    #[test]
    fn busy_component_preserves_work_and_driver_does_not_retain_owner() {
        let (sender, messages) = mpsc::channel();
        let model = Rc::new(RefCell::new(Model { messages, values: Vec::new() }));
        let mut driver = component_dispatcher(
            &model,
            |model| &model.messages,
            |model, message| {
                model.values.push(*message);
                Ok(())
            },
        )
        .unwrap();
        sender.send(Rc::new(1)).unwrap();
        let held = model.borrow();
        assert_eq!(driver.dispatch(), Err(EVENT_E_BUSY));
        drop(held);
        assert_eq!(driver.dispatch(), Ok(true));
        assert_eq!(&model.borrow().values, &[1]);
        assert_eq!(driver.dispatch(), Ok(false));
        drop(model);
        assert_eq!(driver.dispatch(), Ok(false));
        assert!(sender.send(Rc::new(2)).is_err());
    }
}
