//! Opt-in object snapshots, caller-owned factory registries and provider contracts.
//!
//! Providers opt in by implementing `IPersistable` and `IPersistenceFactory`.
//! The core owns their ABI identities; this module supplies the object thunks
//! used by `com_object` without introducing a dependency back into the facade.
//!
//! With `runtime`, `FactoryRegistry` and `snapshot` forward operations to the
//! linked runtime. `static-runtime` selects the same implementation inside the
//! executable or browser module. Contracts remain usable without either feature.

pub use neocogi_com_core::persistence::*;
#[rustfmt::skip]
mod providers;
#[doc(hidden)]
pub use providers::*;

#[cfg(feature = "runtime")]
mod client;
#[cfg(feature = "runtime")]
pub use client::{FactoryRegistry, snapshot, snapshot_with_limits};
