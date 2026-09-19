//! Typed protocol connections and consumer-owned message inboxes.
//! Only generated protocol receivers and connection leases cross plugin boundaries.
//! Mapping is immediate; consumer behavior is deferred to runtime dispatch.

#![deny(missing_docs)]

mod port;
pub use port::{Port, PortRef};
#[doc(hidden)]
pub use port::{PortMessage, PortControl, PortField, PortKey};
mod inbox;
pub use inbox::{Events, EventSender};
mod scoped;
pub use scoped::with_dispatcher;

/// Isolated native algorithm collectors, absent from production builds.
#[cfg(feature = "test-support")]
pub mod test_support;

/// Finite work limit shared by every consumer at one runtime update boundary.
pub const EVENT_DISPATCH_LIMIT: u32 = 1_000_000;

pub use neocogi_com_events_contracts::{IEventDispatcher, IEventDispatcherAbi, IEventSubscription, IEventSubscriptionAbi};

/// Rust binding protocol accepted by compiler-generated COM event adapters and ports.
/// Bump this with the IDL generator when its required transport API or delivery
/// semantics change; an unchanged NCIDL wire ABI alone does not cover that case.
pub const EVENT_BINDING_API_VERSION: u32 = 5;

#[cfg(test)]
mod compiler_binding_tests {
    /// Keeps the released compiler and COM transport support API in lockstep.
    /// Generated modules also enforce this when consumers build outside the workspace.
    #[test]
    fn compiler_and_transport_support_agree_on_event_binding_api() {
        assert_eq!(super::EVENT_BINDING_API_VERSION, neocogi_idl_build::EVENT_BINDING_API_VERSION);
    }
}

/// An event source has permanently closed before a new subscription was claimed.
pub const EVENT_E_CLOSED: neocogi_com::NcStatus = neocogi_com::NcStatus::from_code(-1000);
/// A port post or shared component access would reenter an active mutable operation.
pub const EVENT_E_BUSY: neocogi_com::NcStatus = neocogi_com::NcStatus::from_code(-1001);
/// A finite dispatch budget was exhausted while events were still pending.
pub const EVENT_E_LIMIT: neocogi_com::NcStatus = neocogi_com::NcStatus::from_code(-1007);
