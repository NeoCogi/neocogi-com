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
pub use message::EventMessage;

#[cfg(feature = "alloc")]
mod error;
#[cfg(feature = "alloc")]
pub use error::error;

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
    ///
    /// An observed binding retains a shared receiver reference. A borrowed
    /// exclusive receiver cannot grant that ownership and returns
    /// `NC_E_INVALID_ARG`, leaving its sole owner alive and unchanged.
    /// `None` produces an unobserved binding without acquiring any reference.
    pub fn from_queue(queue: Option<crate::ComRef<'_, IEventReceiverAbi>>) -> Result<Self, crate::NcStatus> {
        Ok(match queue {
            Some(queue) => Self::Observed(queue.try_add_ref()?),
            None => Self::Unobserved,
        })
    }
    /// Moves the sole constructor receiver reference into producer-owned state.
    pub fn into_queue(self) -> Option<crate::ComPtr<IEventReceiverAbi>> {
        match self {
            Self::Observed(queue) => Some(queue),
            Self::Unobserved => None,
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    //! Receiver capture distinguishes a valid borrow from shared lifetime authority.
    use super::*;
    use crate::{ComBox, NcStatus, NcValueRef, NC_E_CLOSED, NC_E_INVALID_ARG};
    use std::{cell::Cell, rc::Rc};

    #[crate::com_object(interfaces(crate::events::IEventReceiver), ownership = exclusive)]
    /// A valid exclusive receiver which must never gain a second owner.
    struct ExclusiveReceiver {
        /// Independent observation of synchronous owner destruction.
        drops: Rc<Cell<u32>>,
    }
    impl Drop for ExclusiveReceiver {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }
    impl IEventReceiver for ExclusiveReceiver {
        fn post(&mut self, _origin: NcValueRef<'_, EventOrigin>, event: ComBox<IEventAbi>) -> Result<(), (NcStatus, ComBox<IEventAbi>)> {
            Err((NC_E_CLOSED, event))
        }
    }

    /// Shared capture retains exactly the destination; absence acquires nothing.
    #[test]
    fn binding_retains_shared_receiver_until_consumed_queue_is_dropped() {
        assert!(matches!(EventBinding::from_queue(None).unwrap(), EventBinding::Unobserved));
        let (queue, inbox) = channel();
        let weak = queue.downgrade().unwrap();
        let binding = EventBinding::from_queue(Some(queue.as_com_ref())).unwrap();
        drop(queue);
        assert!(weak.upgrade().unwrap().is_some());
        let retained = binding.into_queue().unwrap();
        drop(retained);
        assert!(weak.upgrade().unwrap().is_none());
        assert!(matches!(inbox.try_recv(), Err(std::sync::mpsc::TryRecvError::Disconnected)));
    }

    /// Rejection neither destroys nor duplicates the live exclusive receiver.
    #[test]
    fn binding_rejects_exclusive_receiver_without_changing_owner_lifetime() {
        let drops = Rc::new(Cell::new(0));
        let owner = ExclusiveReceiver::into_box(ExclusiveReceiver { drops: drops.clone() })
            .query::<IEventReceiverAbi>()
            .unwrap();
        let weak = owner.downgrade();
        assert!(matches!(EventBinding::from_queue(Some(owner.as_com_ref())), Err(NC_E_INVALID_ARG)));
        assert!(matches!(EventBinding::from_queue(Some(weak.as_com_ref())), Err(NC_E_INVALID_ARG)));
        assert!(weak.is_alive());
        assert_eq!(drops.get(), 0);
        drop(owner);
        assert_eq!(drops.get(), 1);
        assert!(!weak.is_alive());
    }
}
