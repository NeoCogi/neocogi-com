//! Common event/receiver interfaces and generic constructor bindings shared by binaries.
//! Binding specializations describe native payload associations in contract modules;
//! the receiver protocol stays the same for every concrete event interface.

include!(concat!(env!("OUT_DIR"), "/neocogi_com_events.include.rs"));
