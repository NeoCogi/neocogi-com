//! Built-in opt-in persistence contracts shared by providers and the runtime.
//!
//! Interface identity, vtable layouts and wrappers require no allocation. Owned
//! initialization records are enabled with `alloc`; no runtime implementation
//! or process registry is linked by depending on these contracts.

#[rustfmt::skip]
mod interfaces;
pub use interfaces::*;
#[cfg(feature = "alloc")]
mod saved_object;
#[cfg(feature = "alloc")]
pub use saved_object::{Limits, SavedObject, LeafSize};
