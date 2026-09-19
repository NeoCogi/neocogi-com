//! Concrete mappings into consumer-owned message inboxes.
//! Only generated typed receivers cross plugin boundaries.
//! Mapping is immediate; consumer behavior is deferred to runtime dispatch.

#![deny(missing_docs)]

mod inbox;
pub use inbox::{Events, EventSender};
mod scoped;
pub use scoped::with_dispatcher;

pub use neocogi_com_events_contracts::{IEventDispatcher, IEventDispatcherAbi, EVENT_E_LIMIT};

/// A direct sender no longer has a live consumer.
pub const EVENT_E_CLOSED: neocogi_com::NcStatus = neocogi_com::NcStatus::from_code(-1000);
/// Shared component access would reenter an active mutable operation.
pub const EVENT_E_BUSY: neocogi_com::NcStatus = neocogi_com::NcStatus::from_code(-1001);
