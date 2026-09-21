//! Persistence implementation owned by the runtime in every linkage mode.
//!
//! Registries are explicitly allocated by callers, not installed in global
//! plugin state. Each traversal uses ordinary local mutable budgets and retains
//! COM ownership until it either returns a complete tree or cleans up failure.

mod bytes;
mod children;
mod registry;
mod snapshot;

pub(crate) use registry::Registry;
pub(crate) use snapshot::snapshot_with_limits;
