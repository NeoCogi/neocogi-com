//! Common event/receiver interfaces and generic constructor bindings shared by binaries.
//! Derived interfaces describe immutable facts, including explicit failure reports;
//! the receiver protocol stays the same for every concrete event interface.

include!(concat!(env!("OUT_DIR"), "/neocogi_com_events.include.rs"));
