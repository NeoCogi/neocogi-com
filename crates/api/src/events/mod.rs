//! Construction-bound typed COM event queues and producer-local delivery state.
//! Application COM objects own posting access to their ordinary message channels.
//! A synchronous post acquires the snapshot; application behavior runs later.
//! This module neither links the process runtime nor creates a receiver adapter.

#![deny(missing_docs)]

// Contract definitions require macros but neither allocation nor runtime linkage.
#[allow(unused_imports)]
mod contracts;
pub use contracts::*;

#[cfg(feature = "alloc")]
mod message;
#[cfg(feature = "alloc")]
pub use message::{EventMessage, EventPayload, downcast_event};

#[cfg(feature = "alloc")]
mod stream;
#[cfg(feature = "alloc")]
pub use stream::{QueueOf, EventOf, EventPort, EventSource, EventQueueInterface};
