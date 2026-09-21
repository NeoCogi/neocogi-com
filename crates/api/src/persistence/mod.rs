//! Explicit object persistence contracts and provider support.
//!
//! Providers opt in by implementing `IPersistable` and `IPersistenceFactory`.
//! The core owns their ABI identities; this module supplies the object thunks
//! used by `com_object` without introducing a dependency back into the facade.

pub use neocogi_com_core::persistence::*;
mod providers;
#[doc(hidden)]
pub use providers::*;
