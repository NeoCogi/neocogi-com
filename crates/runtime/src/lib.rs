//! Process-global runtime implementation for `neocogi-com`.
//!
//! One append-only plugin catalog owns validated descriptors, borrowed hard
//! dependencies and registration provenance. Dynamic records retain their first
//! diagnostic path and borrow metadata/code from process-pinned native mappings. Static
//! records borrow process-lifetime code and metadata. No module identity,
//! module registry, class registry or runtime contract graph exists.
//! Registration validates metadata before the catalog write lock, then publishes
//! one complete record after checking dependencies, identity and capacity.
//! Repeated descriptors reuse the first registration; distinct descriptors
//! claiming the same plugin GUID are rejected. Plugin code never runs under
//! the catalog lock. Inspection derives views from the authoritative records.
//!
//! Runtime ABI v1 returns plugin GUIDs from loading, enumerates only
//! plugins and exposes origin paths through plugin inspection. It permits 1,024
//! distinct dynamic providers and 64 simultaneous debug messengers. Live-load
//! failures diagnose and abort; static registration and debug capacity failures
//! return statuses. Caller-owned persistence contexts stay outside the catalog.
//!
//! The Cargo package is `neocogi-com-runtime`; the Rust target is `nc_com`.
//! Static and browser hosts link this implementation once. Native shared hosts
//! import the C ABI from the sole shared `nc_com` image. Pure ABI records remain
//! in `neocogi-com-core`; importing them creates no runtime state.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

mod api;
mod debug;
mod descriptor;
mod factory_acquisition;
mod factory_plugin_registry;
mod factory_registration;
mod inspection;
mod loader;
mod persistence;
mod state;

pub use loader::{
    FactoryInspection, PluginDependencyPlanError, PluginInspection, PluginLoadError, PluginLoadErrorKind, inspect_plugin_library, plan_plugin_load_order,
};
