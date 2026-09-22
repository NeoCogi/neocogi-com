//! Compile-checked contract importing another generated IDL module crate.

#![no_std]
#![deny(missing_docs)]

include!(concat!(env!("OUT_DIR"), "/neocogi_idl_consumer.include.rs"));
