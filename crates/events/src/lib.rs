//! Process-neutral typed event transport for COM component producers and consumers.
//!
//! Producer queues and application handlers remain native Rust data in their
//! respective binaries. Only the generated lease, dispatcher, and event sink
//! interfaces cross a plugin boundary. The process-global COM runtime neither
//! owns queues nor stores application state.

#![deny(missing_docs)]

mod source;
pub use source::{EventEndpoint, EventPayload, EventSource};
mod session;
pub use session::{EventDelivery, EventHandler, EventSession, EVENT_DISPATCH_LIMIT};

/// Isolated transport-algorithm fixtures, absent from ordinary builds.
#[cfg(feature = "test-support")]
pub mod test_support {
    pub use super::source::{EventBatch, EventLease, SubscribeError};
}

pub use neocogi_com_events_contracts::{IEventDispatcher, IEventDispatcherAbi, IEventSubscription, IEventSubscriptionAbi};

/// An event source has permanently closed before a new subscription was claimed.
pub const EVENT_E_CLOSED: neocogi_com::NcStatus = neocogi_com::NcStatus::from_code(-1000);
/// A synchronous event transaction would reenter an active subscription or session.
pub const EVENT_E_BUSY: neocogi_com::NcStatus = neocogi_com::NcStatus::from_code(-1001);
/// A finite dispatch budget was exhausted while events were still pending.
pub const EVENT_E_LIMIT: neocogi_com::NcStatus = neocogi_com::NcStatus::from_code(-1007);
