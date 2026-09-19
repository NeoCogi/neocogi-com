//! COM dispatcher contract shared by runtime and consumer implementations.
//! Concrete output receivers are generated with their message declarations;
//! this crate owns no queue, mapping or application behavior.

#![no_std]
#![deny(missing_docs)]

include!(concat!(env!("OUT_DIR"), "/neocogi_com_events.rs"));

/// A finite dispatcher budget was exhausted while messages remained queued.
pub const EVENT_E_LIMIT: neocogi_com::NcStatus = neocogi_com::NcStatus::from_code(-1007);
