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
pub use message::{EventMessage, downcast_event};

#[cfg(feature = "alloc")]
mod stream;
#[cfg(feature = "alloc")]
pub use stream::{EventPort};

#[cfg(feature = "std")]
mod channel;
#[cfg(feature = "std")]
pub use channel::channel;

#[cfg(feature = "alloc")]
impl EventBinding {
    /// Selects immutable delivery policy before publishing a widget or service.
    pub fn from_queue(queue: Option<crate::ComRef<'_, IEventReceiverAbi>>) -> Self {
        match queue {
            Some(queue) => Self::Observed(queue.add_ref()),
            None => Self::Unobserved,
        }
    }
    /// Moves the sole constructor receiver reference into producer-owned state.
    pub fn into_queue(self) -> Option<crate::ComPtr<IEventReceiverAbi>> {
        match self {
            Self::Observed(queue) => Some(queue),
            Self::Unobserved => None,
        }
    }
}
