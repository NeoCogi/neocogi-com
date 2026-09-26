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

#[cfg(feature = "alloc")]
mod stream;
#[cfg(feature = "alloc")]
pub use stream::{EventOf, EventPort, EventSource, EventStreamInterface, MessageSinkInterface};
#[cfg(feature = "std")]
pub use stream::{bind_source, bind_stream};
