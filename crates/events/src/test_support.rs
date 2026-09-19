//! Native algorithm collectors using production generated receivers and consumer inboxes.
use crate::{Events};
use neocogi_com::NcStatus;

/// Convenient collector for isolated native widget algorithms; no alternate transport is used.
pub struct Capture<Event: 'static> {
    /// The fixture owns its queue exclusively and drains through ordinary mutable access.
    events: Events<Event>,
}
impl<Event: 'static> Capture<Event> {
    /// Installs the concrete generated receiver directly into the producer.
    pub fn connect(install: impl FnOnce(Box<dyn FnMut(Event) -> Result<(), NcStatus>>) -> Result<(), NcStatus>) -> Result<Self, NcStatus> {
        let mut events = Events::new();
        events.map(install, |event| event)?;
        Ok(Self { events })
    }
    /// Collects a bounded owned prefix after the algorithm has finished mutating its widget.
    pub fn drain(&mut self, limit: usize) -> Result<Vec<Event>, NcStatus> {
        Ok((0..limit).map_while(|_| self.events.pop()).collect())
    }
}
