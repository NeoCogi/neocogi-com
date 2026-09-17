//! Process-neutral typed event transport for COM component producers and consumers.
//!
//! Producer queues and application handlers remain native Rust data in their
//! respective binaries. Only the generated lease, dispatcher, and event sink
//! interfaces cross a plugin boundary. The process-global COM runtime neither
//! owns queues nor stores application state.
//!
//! Subscription setup uses a consumed [`EventConnector`] with explicit producer
//! and policy fields. [`EventSession::register`] claims its ordered transaction
//! before calling the connector; only the resulting lease is retained. Generated
//! NCIDL helpers supply these operations without application-written sink wiring.
//! This setup protocol is separate from [`EventHandler`] invocation during dispatch.

#![deny(missing_docs)]

mod source;
pub use source::{EventEndpoint, EventPayload, EventSource};
mod session;
pub use session::{EventConnector, EventDelivery, EventDispatchSource, EventHandler, EventSession, EVENT_DISPATCH_LIMIT};

/// Isolated transport-algorithm fixtures, absent from ordinary builds.
#[cfg(feature = "test-support")]
pub mod test_support {
    pub use super::source::{DeliveryWindow, EventLease, SubscribeError};
}

pub use neocogi_com_events_contracts::{IEventDispatcher, IEventDispatcherAbi, IEventSubscription, IEventSubscriptionAbi};

/// Rust binding protocol accepted by compiler-generated COM event adapters and connectors.
/// Bump this with the IDL generator when its required transport API or delivery
/// semantics change; an unchanged NCIDL wire ABI alone does not cover that case.
pub const EVENT_BINDING_API_VERSION: u32 = 2;

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
/// A synchronous event transaction would reenter an active subscription or session.
pub const EVENT_E_BUSY: neocogi_com::NcStatus = neocogi_com::NcStatus::from_code(-1001);
/// A finite dispatch budget was exhausted while events were still pending.
pub const EVENT_E_LIMIT: neocogi_com::NcStatus = neocogi_com::NcStatus::from_code(-1007);

/// Copies a complete offset-zero UTF-8 delivery argument into local owned text.
/// No foreign string view or producer borrow is retained across a handler call.
pub fn copy_text(value: neocogi_com::NcCopyFromBufferOffset<'_, u8>) -> Result<String, neocogi_com::NcStatus> {
    if value.offset() != 0 {
        return Err(neocogi_com::NC_E_INVALID_ARG);
    }
    std::str::from_utf8(value.as_slice())
        .map(str::to_owned)
        .map_err(|_| neocogi_com::NC_E_INVALID_ARG)
}

/// Copies a complete offset-zero input slice into a producer-independent native snapshot.
pub fn copy_slice<Value: Copy>(value: neocogi_com::NcCopyFromBufferOffset<'_, Value>) -> Result<Vec<Value>, neocogi_com::NcStatus> {
    if value.offset() != 0 {
        return Err(neocogi_com::NC_E_INVALID_ARG);
    }
    Ok(value.as_slice().to_vec())
}

#[cfg(test)]
mod copy_tests {
    use super::*;
    use neocogi_com::{NC_E_INVALID_ARG, NcCopyFromBufferOffset};

    /// An event adapter must own foreign input before its application handler runs.
    #[test]
    fn text_and_slice_decoding_copy_complete_valid_arguments() {
        let text = String::from("éclair");
        let captured = copy_text(NcCopyFromBufferOffset::new(0, text.as_bytes())).unwrap();
        assert_eq!(captured, "éclair");
        let bytes = [1u8, 2, 3];
        let captured = copy_slice(NcCopyFromBufferOffset::new(0, &bytes)).unwrap();
        assert_eq!(captured, bytes);
    }

    /// An offset slice or malformed UTF-8 is never exposed as a partial fact.
    #[test]
    fn decoding_rejects_offsets_and_invalid_text() {
        assert_eq!(copy_text(NcCopyFromBufferOffset::new(1, b"valid")), Err(NC_E_INVALID_ARG));
        assert_eq!(copy_text(NcCopyFromBufferOffset::new(0, &[0xff])), Err(NC_E_INVALID_ARG));
        assert_eq!(copy_slice(NcCopyFromBufferOffset::new(1, &[1u32, 2u32])), Err(NC_E_INVALID_ARG));
    }
}
