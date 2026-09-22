//! Concrete COM mappings into standard Rust channels.
//! Only generated typed receivers cross plugin boundaries.
//! Mapping is immediate; consumer behavior is deferred to caller-driven dispatch.
//! The retained UI runtime schedules mounted drivers. This module owns the Rust
//! adapters; enabling it neither links the COM process runtime nor creates its catalog.

#![deny(missing_docs)]

// Contract definitions require macros but neither allocation nor runtime linkage.
mod contracts;
pub use contracts::*;

#[cfg(feature = "std")]
mod named;
#[cfg(feature = "std")]
pub use named::{NamedOutputs, bind_output};
#[cfg(feature = "std")]
mod mapping;
#[cfg(feature = "std")]
pub use mapping::bind;
#[cfg(feature = "std")]
mod component;
#[cfg(feature = "std")]
pub use component::component_dispatcher;
#[cfg(feature = "std")]
mod scoped;
#[cfg(feature = "std")]
pub use scoped::with_dispatcher;

/// A scoped dispatcher was retained beyond its borrowed callback lifetime.
pub const EVENT_E_CLOSED: neocogi_com::NcStatus = neocogi_com::NcStatus::from_code(-1000);
/// Shared component access would reenter an active mutable operation.
pub const EVENT_E_BUSY: neocogi_com::NcStatus = neocogi_com::NcStatus::from_code(-1001);
