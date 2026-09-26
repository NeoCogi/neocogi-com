//! Ordinary generic sink and stream templates shared by producer and consumer binaries.
//! Concrete specializations are generated in the consuming contract modules;
//! this module owns no queue, mapping, or application behavior.

include!(concat!(env!("OUT_DIR"), "/neocogi_com_events.include.rs"));
