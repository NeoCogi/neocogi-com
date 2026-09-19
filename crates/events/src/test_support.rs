//! Native algorithm collectors using production generated receivers and consumer inboxes.
use crate::{PortRef, PortMessage, Events};
use neocogi_com::NcStatus;

/// Convenient collector for isolated native widget algorithms; no alternate transport is used.
pub struct Capture<Event: PortMessage> {
    /// The fixture owns its queue exclusively and drains through ordinary mutable access.
    events: Events<Event>,
}
impl<Event: PortMessage> Capture<Event> {
    /// Installs the protocol's actual generated mapper and holds its ordinary connection lease.
    pub fn connect(port: PortRef<Event>) -> Result<Self, NcStatus> {
        let mut events = Events::new();
        events.map(port, |event| event)?;
        Ok(Self { events })
    }
    /// Collects a bounded owned prefix after the algorithm has finished mutating its widget.
    pub fn drain(&mut self, limit: usize) -> Result<Vec<Event>, NcStatus> {
        Ok((0..limit).map_while(|_| self.events.pop()).collect())
    }
}
