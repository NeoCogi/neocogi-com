//! Common exclusive COM events, weak origins and construction-bound receivers.
//! Posting moves one event owner; application behavior runs after traversal.
//! Standard-channel support is optional and independent of the process runtime.

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
pub use stream::{EventOf, EventPort, EventSource};

#[cfg(feature = "std")]
mod channel;
#[cfg(feature = "std")]
pub use channel::channel;
