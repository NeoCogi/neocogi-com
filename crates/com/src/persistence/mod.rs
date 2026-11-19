//! Opt-in object snapshots, caller-owned factory registries and provider contracts.
//!
//! Providers opt in by implementing `IPersistable` and `IPersistenceFactory`.
//! The core owns their ABI identities; this module supplies the object thunks
//! used by `com_object` without introducing a dependency back into the facade.
//!
//! With `runtime`, `FactoryRegistry` and `snapshot` use caller-supplied `RuntimeHandle`
//! access to the host owner. Every registry retains its creation table through
//! Drop; contracts need no runtime feature or implementation dependency.
//!
//! Snapshot and restore require the application to explicitly register or load
//! `nc.serialization` in the supplied host. The host requests named `ncb`, not
//! the provider's default format. A missing provider returns `NC_E_NOT_FOUND`;
//! there is no implicit codec fallback. Creating a registry and registering its
//! recipes remain independent of codec availability.

pub use neocogi_com::persistence::*;
#[rustfmt::skip]
mod providers;
#[doc(hidden)]
pub use providers::*;

#[cfg(feature = "runtime")]
mod runtime_api;
#[cfg(feature = "runtime")]
pub use runtime_api::{FactoryRegistry, snapshot, snapshot_with_limits};
