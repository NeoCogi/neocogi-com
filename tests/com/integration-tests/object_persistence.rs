//! Ordinary object persistence regressions own one embedded runtime.

// Only this final root retains the implementation; shared qualification imports it.
extern crate neocogi_com_host as _;
include!("support/object_persistence_cases.rs");
