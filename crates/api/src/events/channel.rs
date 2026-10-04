//! An ordinary local inbox for hosts that do not need an application-specific envelope.
use super::{EventMessage, EventOrigin, IEventAbi, IEventReceiver, IEventReceiverAbi};
use crate::{ComBox, ComPtr, NcStatus, NcValueRef, NC_E_CLOSED};
use std::sync::mpsc::{Receiver, Sender};

#[crate::com_object(interfaces(crate::events::IEventReceiver))]
/// Owns posting access only; the receiver and application state belong to the host.
struct EventChannel {
    /// Messages keep their exclusive event owner until the consumer handles them.
    send: Sender<EventMessage>,
}
impl IEventReceiver for EventChannel {
    fn post(&mut self, origin: NcValueRef<'_, EventOrigin>, event: ComBox<IEventAbi>) -> Result<(), (NcStatus, ComBox<IEventAbi>)> {
        let message = EventMessage::capture(origin, event)?;
        self.send.send(message).map_err(|error| (NC_E_CLOSED, error.0.event))
    }
}
/// Creates one common COM receiver for any number of widget and service producers.
/// Dropping the inbox makes subsequent posts return Closed with the original owner.
pub fn channel() -> (ComPtr<IEventReceiverAbi>, Receiver<EventMessage>) {
    let (send, receive) = std::sync::mpsc::channel();
    let queue = EventChannel::from_value(EventChannel { send })
        .query()
        .expect("EventChannel implements IEventReceiver");
    (queue, receive)
}
