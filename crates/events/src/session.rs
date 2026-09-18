//! Application-owned state, typed callback adapters, and registration-order dispatch.

use std::{
    cell::{Ref, RefCell, RefMut},
    ops::{Deref, DerefMut},
    rc::Rc,
};

use neocogi_com::{ComPtr, ComRef, NcStatus, NC_E_INVALID_ARG, com_object};
use neocogi_com_events_contracts::{IEventDispatcher, IEventDispatcherAbi, IEventDispatcherInterfacesFor, IEventSubscriptionAbi};

use crate::{EVENT_E_BUSY, EVENT_E_CLOSED};

/// Default finite callback budget for one deterministic dispatch boundary.
pub const EVENT_DISPATCH_LIMIT: u32 = 1_000_000;

/// Application-local typed behavior; neither state nor handler crosses the COM ABI.
///
/// Methods, functions, and closures with the signature
/// `FnMut(&mut State, &Event) -> Result<(), NcStatus>` implement this trait
/// automatically. A subscription stores the concrete callable inline in its
/// generated receiver; captures need no type erasure or additional shared cells.
/// Explicit implementations remain available for reusable handler objects.
///
/// ```
/// use neocogi_com::NcStatus;
/// use neocogi_com_events::EventHandler;
///
/// let mut adjustment = 0;
/// let mut handler = move |total: &mut u32, event: &u32| -> Result<(), NcStatus> {
///     adjustment += 1; // FnMut gives exclusive access to ordinary captures.
///     *total += *event + adjustment;
///     Ok(())
/// };
/// let mut total = 0;
/// handler.handle(&mut total, &2).unwrap();
/// assert_eq!(total, 3);
/// ```
///
/// Callable adaptation preserves the exact state and event types:
///
/// ```compile_fail
/// use neocogi_com::NcStatus;
/// use neocogi_com_events::EventHandler;
///
/// fn require_handler(_handler: impl EventHandler<u32, String>) {}
/// require_handler(|_: &mut u32, _: &u32| Ok(())); // Wrong event type.
/// ```
///
/// ```compile_fail
/// use neocogi_com_events::EventHandler;
///
/// fn require_handler(_handler: impl EventHandler<u32, u32>) {}
/// require_handler(|_: &mut String, _: &u32| Ok(())); // Wrong state type.
/// ```
pub trait EventHandler<State, Event> {
    /// Handles one owned semantic snapshot while borrowing only local state.
    fn handle(&mut self, state: &mut State, event: &Event) -> Result<(), NcStatus>;
}

/// Adapts a concrete callable without allocating a wrapper or changing delivery ownership.
impl<State, Event, Callback> EventHandler<State, Event> for Callback
where
    Callback: FnMut(&mut State, &Event) -> Result<(), NcStatus>,
{
    /// The receiver's exclusive borrow also protects mutable closure captures.
    fn handle(&mut self, state: &mut State, event: &Event) -> Result<(), NcStatus> {
        self(state, event)
    }
}

/// A typed source port consumed when an owner subscribes to its events.
/// The associated event fixes handler typing before any COM call. Generated
/// implementations retain source capabilities only until setup finishes.
pub trait EventPort {
    /// Owned native snapshot delivered by this exact port.
    type Event;

    /// Builds a local sink and connects it after the session claims registration.
    /// Failure must release uncommitted ownership; no producer stores the session.
    fn connect<State: 'static, Handler: EventHandler<State, Self::Event> + 'static>(
        self,
        session: &EventSession<State>,
        handler: Handler,
    ) -> Result<ComPtr<IEventSubscriptionAbi>, NcStatus>;
}

/// Application-owned state and subscriptions, independent of a UI implementation.
pub struct EventSession<State: 'static> {
    /// One owned state value shared only with local concrete sink specializations.
    state: Rc<RefCell<State>>,
    /// Sole registration sequence shared with the borrowed dispatcher capability.
    registry: Rc<Registry>,
    /// Foreign drivers borrow this projection synchronously and must not retain it.
    dispatcher: ComPtr<IEventDispatcherAbi>,
}

impl<State: 'static> EventSession<State> {
    /// Constructs a session with no subscriptions and application-owned state.
    pub fn new(state: State) -> Self {
        let registry = Rc::new(Registry {
            state: RefCell::new(RegistrationState::Ready(Vec::new())),
        });
        let dispatcher = Dispatcher::from_value(Dispatcher { registry: registry.clone() })
            .query()
            .expect("dispatcher declares IEventDispatcher");
        Self {
            state: Rc::new(RefCell::new(state)),
            registry,
            dispatcher,
        }
    }

    /// Reads local state between callbacks; a conflicting callback borrow reports BUSY.
    pub fn state(&self) -> Result<Ref<'_, State>, NcStatus> {
        self.state.try_borrow().map_err(|_| EVENT_E_BUSY)
    }

    /// Mutates local state between callbacks; finish this borrow before dispatch.
    pub fn state_mut(&self) -> Result<RefMut<'_, State>, NcStatus> {
        self.state.try_borrow_mut().map_err(|_| EVENT_E_BUSY)
    }

    /// Borrows the generated COM dispatcher without transferring session ownership.
    pub fn as_dispatcher(&self) -> ComRef<'_, IEventDispatcherAbi> {
        self.dispatcher.as_com_ref()
    }

    /// Delivers queued events and finite cascades without processing component input.
    pub fn dispatch(&self) -> Result<u32, NcStatus> {
        self.dispatcher.dispatch(EVENT_DISPATCH_LIMIT)
    }

    /// Delivers with an explicit finite callback budget, preserving unattempted events.
    pub fn dispatch_bounded(&self, limit: u32) -> Result<u32, NcStatus> {
        self.dispatcher.dispatch(limit)
    }

    /// Creates a concrete, typed local delivery adapter for one handler.
    /// Distinct sinks share the owner state, but each handler is an ordinary field
    /// exclusively borrowed by its generated mutable COM delivery method.
    pub fn delivery<Handler>(&self, handler: Handler) -> EventDelivery<State, Handler> {
        EventDelivery { state: self.state.clone(), handler }
    }

    /// Claims the registration transaction before any foreign subscription call.
    /// The concrete port creates the sink and connects the producer only
    /// after the transaction is claimed. Failures restore the exact previous
    /// registration order. Successful registration retains the lease, not the
    /// port. Generated ports own their source projection only until
    /// this synchronous operation returns; application handlers retain their own dependencies.
    /// Pass a state method, a function, or a closure with explicit parameter types.
    /// Captures must be owned (`move` when needed) or otherwise valid for `'static`,
    /// because the subscription retains the callable until cancellation or closure.
    pub fn subscribe<Port, Handler>(&self, port: Port, handler: Handler) -> Result<ComPtr<IEventSubscriptionAbi>, NcStatus>
    where
        Port: EventPort,
        Handler: EventHandler<State, Port::Event> + 'static,
    {
        // Own the sequence before constructing a sink or calling foreign code;
        // Registrations restores it on both connection failure and success.
        let mut leases = self.registry.take()?;
        let lease = port.connect(self, handler)?;
        // The returned reference is caller-owned; this clone keeps delivery live
        // even when the caller immediately drops that reference.
        leases.push(lease.clone());
        Ok(lease)
    }

    /// Cancels all leases and releases their sinks; active dispatch reports BUSY.
    pub fn cancel(&self) -> Result<(), NcStatus> {
        let mut leases = self.registry.take()?;
        for lease in leases.iter() {
            lease.cancel()?;
        }
        leases.clear();
        Ok(())
    }
}

impl<State: 'static> Drop for EventSession<State> {
    /// Invalidates retained dispatcher/lease references and cancels idle registrations.
    fn drop(&mut self) {
        // Publish closure before releasing foreign leases. An active transaction
        // owns its vector and performs the same cleanup when it returns.
        let retired = self.registry.state.replace(RegistrationState::Closed);
        if let RegistrationState::Ready(leases) = retired {
            drop(Registrations { registry: &self.registry, leases });
        }
    }
}

/// The registration sequence has exactly one owner or one explicit terminal state.
enum RegistrationState {
    /// Idle session owns its registration-ordered subscriptions.
    Ready(Vec<ComPtr<IEventSubscriptionAbi>>),
    /// One transaction owns the vector outside the shared slot.
    Busy,
    /// Session destruction permanently rejects registration and dispatch.
    Closed,
}

/// Shared lifecycle boundary between the native owner and its COM dispatcher.
struct Registry {
    /// Interior mutability is limited to moving ownership across foreign calls.
    state: RefCell<RegistrationState>,
}

impl Registry {
    /// Takes the sole ordered vector, excluding registration and dispatch reentry.
    fn take(&self) -> Result<Registrations<'_>, NcStatus> {
        let mut state = self.state.try_borrow_mut().map_err(|_| EVENT_E_BUSY)?;
        match &*state {
            RegistrationState::Closed => Err(EVENT_E_CLOSED),
            RegistrationState::Busy => Err(EVENT_E_BUSY),
            RegistrationState::Ready(_) => match std::mem::replace(&mut *state, RegistrationState::Busy) {
                RegistrationState::Ready(leases) => Ok(Registrations { registry: self, leases }),
                _ => unreachable!("the matched ready state cannot change under its exclusive borrow"),
            },
        }
    }

    /// Reads the terminal state without retaining a borrow during foreign calls.
    fn closed(&self) -> bool {
        matches!(*self.state.borrow(), RegistrationState::Closed)
    }
}

/// Owned transaction that restores the precise registration sequence on every exit.
struct Registrations<'a> {
    /// Vacant destination survives foreign calls without lending its borrow guard.
    registry: &'a Registry,
    /// Only this transaction owns the ordered leases while it is active.
    leases: Vec<ComPtr<IEventSubscriptionAbi>>,
}

impl Deref for Registrations<'_> {
    type Target = Vec<ComPtr<IEventSubscriptionAbi>>;
    /// Borrows only the locally owned sequence.
    fn deref(&self) -> &Self::Target {
        &self.leases
    }
}

impl DerefMut for Registrations<'_> {
    /// Mutates only the locally owned sequence.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.leases
    }
}

impl Registrations<'_> {
    /// Cancels inactive sinks before pruning them, preserving surviving order.
    fn prune(&mut self) -> Result<(), NcStatus> {
        let mut index = 0;
        while index < self.len() {
            if self[index].active()? {
                index += 1;
            } else {
                self[index].cancel()?;
                drop(self.remove(index));
            }
        }
        Ok(())
    }
}

impl Drop for Registrations<'_> {
    /// Restores the sole vector after foreign calls; closing cancels every lease.
    fn drop(&mut self) {
        match self.registry.closed() {
            true => {
                // Cancellation can reenter the registry; it already reports Closed.
                for lease in &self.leases {
                    let _ = lease.cancel();
                }
            }
            false => {
                *self.registry.state.borrow_mut() = RegistrationState::Ready(std::mem::take(&mut self.leases));
            }
        }
    }
}

#[com_object(interfaces(IEventDispatcher))]
/// Retainable COM projection containing leases only, never application state.
struct Dispatcher {
    /// Shares the sole sequence with its application-owned session.
    registry: Rc<Registry>,
}

impl IEventDispatcher for Dispatcher {
    /// Sweeps each registered FIFO in order until quiescent or budget-exhausted.
    fn dispatch(&self, limit: u32) -> Result<u32, NcStatus> {
        let mut leases = self.registry.take()?;
        leases.prune()?;
        let mut delivered = 0;
        loop {
            let before = delivered;
            for lease in leases.iter() {
                if self.registry.closed() {
                    return Err(EVENT_E_CLOSED);
                }
                let remaining = limit - delivered;
                let count = lease.dispatch(remaining)?;
                if count > remaining {
                    return Err(NC_E_INVALID_ARG);
                }
                delivered += count;
            }
            if before == delivered {
                break;
            }
        }
        leases.prune()?;
        Ok(delivered)
    }
}

/// Concrete handler and state held within an application-local sink specialization.
pub struct EventDelivery<State, Handler> {
    /// State belongs to the application session, not a producer DSO.
    state: Rc<RefCell<State>>,
    /// The mutable COM sink receiver supplies exclusive access to this concrete handler.
    handler: Handler,
}

impl<State, Handler> EventDelivery<State, Handler> {
    /// Invokes one decoded event with an exclusive local state and handler borrow.
    /// Both borrow guards end before a recoverable callback failure is reported.
    pub fn deliver<Event>(&mut self, event: Event) -> Result<(), NcStatus>
    where
        Handler: EventHandler<State, Event>,
    {
        // The generated mutable sink gate excludes recursive invocation. Only
        // state shared by distinct sinks needs a dynamic borrow at this boundary.
        let mut state = self.state.try_borrow_mut().map_err(|_| EVENT_E_BUSY)?;
        self.handler.handle(&mut state, &event)
    }
}

/// Registration regressions use the same transaction and dispatcher as production.
#[cfg(test)]
mod tests;
