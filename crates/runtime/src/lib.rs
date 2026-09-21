//! Process-global runtime implementation for `neocogi-com`.
//!
//! This crate owns dynamic library loading on supported native targets,
//! process-global plugin state, caller-owned persistence contexts, and
//! the C ABI exported by the runtime shared library. Pure ABI data
//! structures stay in `neocogi-com-core` so hosts and plugins can share the binary
//! contract without also linking a registry implementation.
//!
//! One append-only catalog keeps deterministic registration-order vectors and
//! runtime-private hash indexes for plugin and loaded-module lookup.
//! Registration validates a factory descriptor before taking the catalog write
//! lock, then publishes correlated plugin/module records atomically after
//! checking dependencies and existing registrations. There is no class or
//! contract-module registry. Concurrent reads share the catalog
//! read lock. The crate is intentionally `std`-only`; `neocogi-com-core` retains the allocation-free and `no_std` ABI layer.
//!
//! Loaded-module records retain one diagnostic path alongside the native
//! library handle. Qualified plugin names and versions are not copied into that
//! catalog: the registered descriptor remains authoritative, and the
//! inspection C table exposes them through `plugin_info`. C callers obtain a
//! module's path and plugin ID through `module_info`, then pass that plugin ID
//! to `plugin_info` for its name and version. The `neocogi-com` Rust facade performs
//! this join and returns owned `LoadedModuleInfo` values.
//!
//! Runtime ABI v1 caps live-loaded module records at 1,024 and simultaneous debug
//! messenger registrations at 64. Module-capacity exhaustion follows the
//! fatal live-loading path. Debug messenger capacity exhaustion is
//! recoverable as `NC_E_OUT_OF_RANGE`; destruction frees a slot without
//! permitting stale handle reuse.
//!
//! The Cargo package is `neocogi-com-runtime`; the Rust library target is
//! `nc_com`. Its default output is an rlib for static hosts and generated assembly
//! roots. `cargo rustc -p neocogi-com-runtime --lib --crate-type cdylib` produces
//! the standalone `libnc_com.so` / `libnc_com.dylib` / `nc_com.dll` runtime.
//! Browser modules link the same implementation statically; dynamic loading is
//! unavailable there. Persistence contexts remain independent of the catalog.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

mod api;
mod debug;
mod factory_activation;
mod factory_plugin_registry;
mod factory_registration;
mod inspection;
mod loader;
mod module_registry;
mod state;
mod persistence;

pub use loader::{
    FactoryInspection, PluginDependencyPlanError, PluginInspection, PluginLoadError, PluginLoadErrorKind, inspect_plugin_library, plan_plugin_load_order,
};
