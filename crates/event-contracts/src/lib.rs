//! Generated COM event lease and dispatcher interfaces.
//!
//! The NCIDL module is the only source of the ABI definitions. This crate is
//! deliberately independent of UI and of the process-global COM runtime.

#![no_std]
#![deny(missing_docs)]

include!(concat!(env!("OUT_DIR"), "/neocogi_com_events.rs"));
