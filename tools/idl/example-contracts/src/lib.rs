//! Compile-checked reference contract generated entirely from NeoCogi IDL.

#![no_std]
#![deny(missing_docs)]

include!(concat!(env!("OUT_DIR"), "/neocogi_idl_example.include.rs"));

/// Immutable event construction for this independently authored contract.
pub mod events;
