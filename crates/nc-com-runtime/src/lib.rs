//! Process-global runtime implementation for `nc-com`.
//!
//! This crate owns the implementation pieces that require an operating system
//! runtime: dynamic library loading, process-global state, and
//! the C ABI exported by the runtime shared library. Pure ABI data
//! structures stay in `nc-com-core` so hosts and plugins can share the binary
//! contract without also linking a registry implementation.
//!
//! One append-only catalog keeps deterministic registration-order vectors and
//! runtime-private hash indexes for identity, class, interface, relationship,
//! and module lookup. Registration validates and assembles one internal batch
//! before taking the catalog write lock, then publishes every correlated record
//! atomically after state-dependent checks. Concurrent reads share the catalog
//! read lock. The crate is intentionally `std`-only`; `nc-com-core` and
//! `nc-com-runtime-sys` retain the allocation-free and `no_std` ABI layers.
//!
//! Loaded-module records retain one diagnostic path alongside the native
//! library handle. Qualified plugin names and versions are not copied into that
//! catalog: the registered descriptor remains authoritative, and the
//! inspection C table exposes them through `plugin_info`. C callers obtain a
//! module's path and plugin ID through `module_info`, then pass that plugin ID
//! to `plugin_info` for its name and version. The `nc-com` Rust facade performs
//! this join and returns owned `LoadedModuleInfo` values.
//!
//! Runtime ABI v1 caps retained dynamic modules at 1,024 and simultaneous debug
//! messenger registrations at 64. Module-capacity exhaustion follows the
//! existing fatal live-activation path. Debug messenger capacity exhaustion is
//! recoverable as `NC_E_OUT_OF_RANGE`; destruction frees a slot without
//! permitting stale handle reuse.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

mod api;
mod construction;
mod debug;
mod inspection;
mod loader;
mod plugin_registry;
mod registration;
mod state;
mod type_registry;

pub use loader::{
    ClassInspection, ClassInterfaceInspection, InterfaceInspection, MethodInspection,
    PluginDependencyPlanError, PluginInspection, PluginLoadError, PluginLoadErrorKind,
    inspect_plugin_library, plan_plugin_load_order,
};
