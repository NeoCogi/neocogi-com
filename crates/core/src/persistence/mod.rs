//! Built-in opt-in persistence contracts shared by providers and the runtime.
//!
//! Interface identity, vtable layouts and clients require no allocation. Owned
//! initialization records are enabled with `alloc`; no runtime implementation
//! or process registry is linked by depending on these contracts.

mod interfaces;
pub use interfaces::*;
#[cfg(feature = "alloc")]
mod record;
#[cfg(feature = "alloc")]
pub use record::{Limits, ObjectRecord};
