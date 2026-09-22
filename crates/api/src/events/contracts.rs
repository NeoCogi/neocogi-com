//! Named output binding contract shared by independent producer and consumer binaries.
//! Concrete output receivers are generated with their message declarations;
//! this contract module owns no queue, mapping or application behavior.

include!(concat!(env!("OUT_DIR"), "/neocogi_com_events.rs"));
