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
//! The Cargo package is `neocogi-com-host`; the Rust target is `neocogi_com_host`.
//! Native and browser hosts link this implementation once and lend its immutable
//! C service tables to providers. No runtime DSO is deployed. Pure ABI records remain
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

/// Initializes the selected host implementation and lends its canonical C API.
///
/// Initialization is empty, infallible and idempotent. It invokes no provider,
/// loads no library and seeds no contract registry. State, immutable tables and
/// callback code remain valid for the process/Wasm-instance lifetime, including
/// after the application entry returns. Copies of the supplied access do not
/// create additional owners. Reusable libraries/plugins receive that access;
/// they do not select or initialize another implementation. Host catalog/debug
/// entrypoints synchronize concurrent calls; caller-owned persistence contexts
/// retain their thread-local and exclusive-operation requirements.
pub fn initialize_process() -> &'static neocogi_com::NcHostApiV1 {
    let _ = state::RuntimeState::global();
    &api::HOST_API
}
