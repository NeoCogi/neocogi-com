//! Already compiled IDL fixtures emitted into each test's local Rust scope.
//!
//! No fixture macro accepts a contract grammar. Each no-argument macro contains
//! complete Rust emitted from its owning `.ncidl` source by the ordinary compiler.
//! Local types let tests supply native Default/inherent implementations and let
//! trybuild consume contracts without depending on another target's OUT_DIR.
#![deny(missing_docs)]
include!(concat!(env!("OUT_DIR"), "/contracts.rs"));
