//! One producer-owned connection slot and its generated COM connection projection.

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use neocogi_com::{com_object, ComInterface, ComMessage, ComPtr, ComRef, NcStatus};
use neocogi_com_events_contracts::IEventSubscriptionInterfacesFor;
use crate::{EventSender, IEventSubscription, IEventSubscriptionAbi, EVENT_E_BUSY, EVENT_E_CLOSED};

/// Generated operations for a concrete message's connection projection.
#[doc(hidden)]
pub trait PortMessage: ComMessage {
    /// Exact generated receiver; its only operation accepts this message's borrowed wire.
    type Receiver: ComInterface;
    /// Posts this concrete value through its checked generated COM entry point.
    fn deliver(&self, receiver: ComRef<'_, Self::Receiver>) -> Result<(), NcStatus>;
    /// Exact connection interface; its receiver accepts only this message.
    type PortAbi: ComInterface;
    /// Builds the weak COM projection without retaining the producer.
    fn project(control: PortControl<Self>) -> ComPtr<Self::PortAbi>;
    /// Invokes the generated connection slot without exposing receiver machinery to authors.
    fn connect(port: ComRef<'_, Self::PortAbi>, receiver: ComRef<'_, Self::Receiver>) -> Result<ComPtr<IEventSubscriptionAbi>, NcStatus>;
    /// Constructs a checked, concrete mapper for this protocol.
    fn mapping<Message: 'static, Map: FnMut(Self) -> Message + 'static>(
        sender: EventSender<Message>,
        map: Map,
        validate: Option<fn(&Self) -> Result<(), NcStatus>>,
    ) -> Result<ComPtr<Self::Receiver>, NcStatus>;
}

/// Producer storage for one typed output. Consumers obtain its connection projection.
pub struct Port<E: PortMessage> {
    /// Independent COM projection and lease identities share cancellation with emission.
    /// Only this short slot mutation requires RefCell; foreign calls run outside its borrow.
    slot: Rc<RefCell<Slot<E::Receiver>>>,
    /// Weak generated projection, allocated once and reused by every accessor.
    projection: ComPtr<E::PortAbi>,
}
/// Monotonic connection identity prevents an old token from cancelling its replacement.
struct Slot<Receiver: ComInterface> {
    /// Last successfully assigned connection identity, checked for overflow.
    generation: u64,
    /// Complete current receiver ownership and logical-lifetime state.
    phase: Phase<Receiver>,
}
/// Receiver ownership is outside the shared slot while foreign code executes.
enum Phase<Receiver: ComInterface> {
    /// Open output without a consumer.
    Disconnected,
    /// Installed concrete mapper is ready to accept a fact.
    Connected(ComPtr<Receiver>),
    /// An emission temporarily owns the receiver; recursive emission is rejected.
    Posting,
    /// The producer logically closed or was destroyed.
    Closed,
}
impl<E: PortMessage> Port<E> {
    /// Creates an empty output and its non-owning COM connection projection.
    pub fn new() -> Self {
        let slot = Rc::new(RefCell::new(Slot {
            generation: 0,
            phase: Phase::Disconnected,
        }));
        let projection = E::project(PortControl { slot: Rc::downgrade(&slot) });
        Self { slot, projection }
    }
    /// Obtains connection access without exposing emission or retaining the producer.
    pub fn as_ref(&self) -> PortRef<E> {
        PortRef::from_result(Ok(self.projection.clone()))
    }
    /// Reports whether one mapping is currently installed, including during its own post.
    pub fn connected(&self) -> bool {
        matches!(self.slot.borrow().phase, Phase::Connected(_) | Phase::Posting)
    }
    /// Commits an owned fact through one typed COM call, without a producer-side queue.
    pub fn emit(&self, message: E) -> Result<(), NcStatus> {
        let (generation, receiver) = {
            let mut slot = self.slot.borrow_mut();
            match slot.phase {
                Phase::Disconnected | Phase::Closed => return Ok(()),
                Phase::Posting => return Err(EVENT_E_BUSY),
                Phase::Connected(_) => {}
            }
            let Phase::Connected(receiver) = std::mem::replace(&mut slot.phase, Phase::Posting) else {
                unreachable!()
            };
            (slot.generation, receiver)
        };
        let posting = Posting {
            slot: &self.slot,
            generation,
            receiver: Some(receiver),
        };
        message.deliver(posting.receiver.as_ref().expect("posting owns receiver").as_com_ref())
    }
    /// Closes future delivery without retracting facts already in a consumer inbox.
    pub fn close_ordinary(&self) {
        let retired = std::mem::replace(&mut self.slot.borrow_mut().phase, Phase::Closed);
        drop(retired); // Foreign receiver destructors run after the slot borrow ends.
    }
    /// Publishes one final fact after closing connection access, then releases its receiver.
    pub fn close(&self, message: E) -> Result<(), NcStatus> {
        let retired = std::mem::replace(&mut self.slot.borrow_mut().phase, Phase::Closed);
        match retired {
            Phase::Connected(receiver) => message.deliver(receiver.as_com_ref()),
            Phase::Posting => Err(EVENT_E_BUSY),
            Phase::Disconnected | Phase::Closed => Ok(()),
        }
    }
}
impl<E: PortMessage> Default for Port<E> {
    /// Uses the same disconnected, live construction as new.
    fn default() -> Self {
        Self::new()
    }
}
impl<E: PortMessage> Drop for Port<E> {
    /// Retained connection projections never prolong producer lifetime.
    fn drop(&mut self) {
        self.close_ordinary();
    }
}
/// Restores only the same uncancelled connection after a foreign post returns.
struct Posting<'a, Receiver: ComInterface> {
    /// Shared slot remains live for the emission, independent of a retained projection.
    slot: &'a RefCell<Slot<Receiver>>,
    /// Exact connection selected before calling foreign code.
    generation: u64,
    /// Sole call-local receiver owner; released outside the slot borrow on cancellation.
    receiver: Option<ComPtr<Receiver>>,
}
impl<Receiver: ComInterface> Drop for Posting<'_, Receiver> {
    /// Reentrant cancellation, replacement and closure cannot be undone by restoration.
    fn drop(&mut self) {
        let mut slot = self.slot.borrow_mut();
        if slot.generation == self.generation && matches!(slot.phase, Phase::Posting) {
            slot.phase = Phase::Connected(self.receiver.take().expect("posting owns receiver"));
        }
    }
}

/// Framework access held by a generated COM projection; cannot keep a producer alive.
#[doc(hidden)]
pub struct PortControl<E: PortMessage> {
    /// One weak link reaches the exact output slot, without another source/endpoint registry.
    slot: Weak<RefCell<Slot<E::Receiver>>>,
}
impl<E: PortMessage> PortControl<E> {
    /// Installs one receiver and returns a generation-specific cancellation capability.
    pub fn connect(&self, receiver: ComRef<'_, E::Receiver>) -> Result<ComPtr<IEventSubscriptionAbi>, NcStatus> {
        let receiver = receiver.add_ref(); // No shared slot borrow crosses AddRef.
        let slot = self.slot.upgrade().ok_or(EVENT_E_CLOSED)?;
        let generation = {
            let mut slot = slot.borrow_mut();
            match slot.phase {
                Phase::Closed => return Err(EVENT_E_CLOSED),
                Phase::Connected(_) | Phase::Posting => return Err(EVENT_E_BUSY),
                Phase::Disconnected => {}
            }
            let generation = slot.generation.checked_add(1).ok_or(neocogi_com::NC_E_OUT_OF_RANGE)?;
            slot.generation = generation;
            slot.phase = Phase::Connected(receiver);
            generation
        };
        Lease::from_value(Lease { slot: self.slot.clone(), generation }).query()
    }
}
#[com_object(interfaces(IEventSubscription))]
/// A connection token owns cancellation responsibility, never a producer or queue.
struct Lease<Receiver: ComInterface> {
    /// Weak ownership permits producer destruction while consumers retain committed messages.
    slot: Weak<RefCell<Slot<Receiver>>>,
    /// Identity checked before every observation or cancellation.
    generation: u64,
}
impl<Receiver: ComInterface> IEventSubscription for Lease<Receiver> {
    /// Reports whether this exact mapping still accepts future facts.
    fn active(&self) -> Result<bool, NcStatus> {
        Ok(self.slot.upgrade().is_some_and(|slot| {
            let slot = slot.borrow();
            slot.generation == self.generation && matches!(slot.phase, Phase::Connected(_) | Phase::Posting)
        }))
    }
    /// Releases the current receiver outside the borrow; successors are never affected.
    fn cancel(&self) -> Result<(), NcStatus> {
        let retired = self.slot.upgrade().and_then(|slot| {
            let mut slot = slot.borrow_mut();
            match slot.phase {
                Phase::Connected(_) | Phase::Posting if slot.generation == self.generation => Some(std::mem::replace(&mut slot.phase, Phase::Disconnected)),
                _ => None,
            }
        });
        drop(retired);
        Ok(())
    }
}
impl<Receiver: ComInterface> Drop for Lease<Receiver> {
    /// Final token release disconnects even when callers never explicitly cancel.
    fn drop(&mut self) {
        let _ = self.cancel();
    }
}

/// Read-only connection view of one port; it grants no authority to emit messages.
pub struct PortRef<E: PortMessage> {
    /// Accessor failure is reported by map together with other connection errors.
    projection: Result<ComPtr<E::PortAbi>, NcStatus>,
    /// Optional domain validation executes before mapping, after wire decoding.
    validate: Option<fn(&E) -> Result<(), NcStatus>>,
}
impl<E: PortMessage> PortRef<E> {
    /// Receives the generated output projection; accessors do not install a mapping.
    #[doc(hidden)]
    pub fn from_result(projection: Result<ComPtr<E::PortAbi>, NcStatus>) -> Self {
        Self { projection, validate: None }
    }
    /// Attaches a domain check without changing the concrete wire protocol.
    pub fn checked(mut self, validate: fn(&E) -> Result<(), NcStatus>) -> Self {
        self.validate = Some(validate);
        self
    }
    /// Transfers this exact COM connection projection to a generated output getter.
    #[doc(hidden)]
    pub fn into_projection(self) -> Result<ComPtr<E::PortAbi>, NcStatus> {
        self.projection
    }
}
impl<E: PortMessage> Clone for PortRef<E> {
    /// Cloning a connection view does not retain its producer or create another mapping.
    fn clone(&self) -> Self {
        Self {
            projection: self.projection.clone(),
            validate: self.validate,
        }
    }
}
impl<E: PortMessage> std::fmt::Debug for PortRef<E> {
    /// Does not expose private projection pointers or mapper state.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PortRef").finish_non_exhaustive()
    }
}
impl<E: PortMessage> PortRef<E> {
    /// Installs one concrete receiver through the same COM path for native and plugin outputs.
    pub(crate) fn connect<Message: 'static, Map: FnMut(E) -> Message + 'static>(
        self,
        sender: EventSender<Message>,
        map: Map,
    ) -> Result<ComPtr<IEventSubscriptionAbi>, NcStatus> {
        let projection = self.projection?;
        let receiver = E::mapping(sender, map, self.validate)?;
        E::connect(projection.as_com_ref(), receiver.as_com_ref())
    }
}

/// Generated marker associates a declared output with its exact concrete message.
#[doc(hidden)]
pub trait PortKey {
    /// Message accepted by this output's connection projection.
    type Event: PortMessage;
}
/// Object macro binds a declared output to its real storage without subscription forwarding.
#[doc(hidden)]
pub trait PortField<Key: PortKey> {
    /// Obtains checked connection access through ordinary shared object access.
    fn port(&self) -> Result<PortRef<Key::Event>, NcStatus>;
}
