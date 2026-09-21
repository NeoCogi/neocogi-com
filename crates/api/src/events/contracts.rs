//! COM dispatcher contract shared by runtime and consumer implementations.
//! Concrete output receivers are generated with their message declarations;
//! this contract module owns no queue, mapping or application behavior.

include!(concat!(env!("OUT_DIR"), "/neocogi_com_events.rs"));

/// The runtime reached its finite update budget before establishing quiescence.
pub const EVENT_E_LIMIT: neocogi_com::NcStatus = neocogi_com::NcStatus::from_code(-1007);
