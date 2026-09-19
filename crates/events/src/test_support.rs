//! Native algorithm collectors using production generated receivers and consumer inboxes.
use std::cell::RefCell;
use crate::{EventEndpoint, EventPayload, Events};
use neocogi_com::NcStatus;

/// Convenient collector for isolated native widget algorithms; no alternate transport is used.
pub struct Capture<Event: EventPayload> {
    /// Tests read through shared fixture handles; only the consumer storage is borrowed dynamically.
    events: RefCell<Events<Event>>,
}
impl<Event: EventPayload> Capture<Event> {
    /// Installs the protocol's actual generated mapper and holds its ordinary connection lease.
    pub fn connect(port: EventEndpoint<Event>) -> Result<Self, NcStatus> {
        let mut events = Events::new();
        events.map(port, |event| event)?;
        Ok(Self { events: RefCell::new(events) })
    }
    /// Collects a bounded owned prefix after the algorithm has finished mutating its widget.
    pub fn drain(&self, limit: usize) -> Result<Vec<Event>, NcStatus> {
        let mut events = self.events.borrow_mut();
        Ok((0..limit).map_while(|_| events.pop()).collect())
    }
}
