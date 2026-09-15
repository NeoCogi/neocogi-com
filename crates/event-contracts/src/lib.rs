//! Generated COM event lease and dispatcher interfaces.
//!
//! The NCIDL module is the only source of the ABI definitions and the canonical
//! lease identity required by the COM-owned IDL compiler's `event` keyword.
//! It is independent of UI and of the process-global COM runtime implementation;
//! generated binding and transport semantics are version-checked separately.

#![no_std]
#![deny(missing_docs)]

include!(concat!(env!("OUT_DIR"), "/neocogi_com_events.rs"));
