//! Application-owned state, typed callback adapters, and registration-order dispatch.

use std::{
    cell::{Cell, Ref, RefCell, RefMut},
    ops::{Deref, DerefMut},
    rc::Rc,
};

use neocogi_com::{ComPtr, ComRef, NcStatus, NC_E_INVALID_ARG, com_object};
use neocogi_com_events_contracts::{IEventDispatcher, IEventDispatcherAbi, IEventDispatcherInterfacesFor, IEventSubscriptionAbi};

use crate::{EVENT_E_BUSY, EVENT_E_CLOSED};

/// Default finite callback budget for one deterministic dispatch boundary.
pub const EVENT_DISPATCH_LIMIT: u32 = 1_000_000;

/// Borrowable application dispatcher capability accepted by an eventful host.
/// Hosts consume this only synchronously; implementing it must not transfer
/// state or handler ownership to the host.
pub trait EventDispatchSource {
    /// Borrows the exact generated dispatcher projection for one update call.
    fn as_dispatcher(&self) -> ComRef<'_, IEventDispatcherAbi>;
}

/// Application-local typed behavior; neither state nor handler crosses the COM ABI.
///
/// Implement this trait on a concrete struct or enum whose dependencies are
/// explicit fields. Closures and function pointers do not automatically implement
/// it; there is no implicit `FnMut` adapter hiding a handler's captured state.
///
/// ```
/// use neocogi_com::NcStatus;
/// use neocogi_com_events::EventHandler;
///
/// /// Accumulates each event together with an explicitly owned adjustment.
/// struct Add {
///     /// Per-handler adjustment, visible instead of captured by a closure.
///     adjustment: u32,
/// }
/// impl EventHandler<u32, u32> for Add {
///     /// Applies the event and this handler's named configuration to local state.
///     fn handle(&mut self, total: &mut u32, event: &u32) -> Result<(), NcStatus> {
///         *total += *event + self.adjustment;
///         Ok(())
///     }
/// }
/// let mut handler = Add { adjustment: 1 };
/// let mut total = 0;
/// handler.handle(&mut total, &2).unwrap();
/// assert_eq!(total, 3);
/// ```
///
/// A closure cannot substitute for an explicit handler implementation:
///
/// ```compile_fail
/// use neocogi_com::NcStatus;
/// use neocogi_com_events::EventHandler;
///
/// /// Requires the same handler capability as generated event adapters.
/// fn require_handler(_handler: impl EventHandler<u32, u32>) {}
/// let adjustment = 1;
/// require_handler(move |total: &mut u32, event: &u32| -> Result<(), NcStatus> {
///     *total += *event + adjustment;
///     Ok(())
/// });
/// ```
///
/// A capture-free function pointer is not an implicit handler either:
///
/// ```compile_fail
/// use neocogi_com::NcStatus;
/// use neocogi_com_events::EventHandler;
///
/// /// A plain function intentionally has no EventHandler implementation.
/// fn record(total: &mut u32, event: &u32) -> Result<(), NcStatus> {
///     *total += *event;
///     Ok(())
/// }
/// /// Requires explicit typed behavior, not a callable function-pointer shortcut.
/// fn require_handler(_handler: impl EventHandler<u32, u32>) {}
/// require_handler(record as fn(&mut u32, &u32) -> Result<(), NcStatus>);
/// ```
pub trait EventHandler<State, Event> {
    /// Handles one owned semantic snapshot while borrowing only local state.
    fn handle(&mut self, state: &mut State, event: &Event) -> Result<(), NcStatus>;
}

/// One explicitly typed subscription operation, local to the consumer's binary.
///
/// A connector stores its producer capability and any connection policy in named
/// fields. Generated contract bindings implement this trait for each event port;
/// the generic session never needs to know a widget, sink, or producer type.
/// Connectors are consumed during registration, not retained for event delivery.
pub trait EventConnector<State: 'static, Handler> {
    /// Constructs the local sink and connects the producer inside registration.
    ///
    /// The session has already excluded registration and dispatch reentry before
    /// calling this method. On failure, the connector must release any uncommitted
    /// connection; the session restores its previous ordered lease sequence.
    fn connect(self, session: &EventSession<State>, handler: Handler) -> Result<ComPtr<IEventSubscriptionAbi>, NcStatus>;
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
            leases: RefCell::new(Some(Vec::new())),
            closed: Cell::new(false),
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
    /// The short RefCell borrows are required because COM sink methods are `&self`
    /// and independent sources invoke them synchronously through an ABI pointer.
    pub fn delivery<Handler>(&self, handler: Handler) -> EventDelivery<State, Handler> {
        EventDelivery {
            state: self.state.clone(),
            handler: RefCell::new(Some(handler)),
        }
    }

    /// Claims the registration transaction before any foreign subscription call.
    /// The concrete connector creates the sink and connects the producer only
    /// after the transaction is claimed. Failures restore the exact previous
    /// registration order. Successful registration retains the lease, not the
    /// connector. Generated connectors borrow their source projection only during
    /// this synchronous operation; application handlers retain their own dependencies.
    pub fn register<Handler, Connector>(&self, handler: Handler, connector: Connector) -> Result<ComPtr<IEventSubscriptionAbi>, NcStatus>
    where
        Connector: EventConnector<State, Handler>,
    {
        // Own the sequence before constructing a sink or calling foreign code;
        // Registrations restores it on both connection failure and success.
        let mut leases = self.registry.take()?;
        let lease = connector.connect(self, handler)?;
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

impl<State: 'static> EventDispatchSource for EventSession<State> {
    /// Borrows this session's separate COM allocation without an AddRef transfer.
    fn as_dispatcher(&self) -> ComRef<'_, IEventDispatcherAbi> {
        self.dispatcher.as_com_ref()
    }
}

impl<State: 'static> Drop for EventSession<State> {
    /// Invalidates retained dispatcher/lease references and cancels idle registrations.
    fn drop(&mut self) {
        let idle = self.registry.take();
        self.registry.closed.set(true);
        // An active transaction observes `closed` and performs the same cleanup
        // when it restores its sole sequence after the callback returns.
        drop(idle);
    }
}

/// Registration storage taken by value before any foreign callback or Release.
struct Registry {
    /// Destruction is observable even if a retained dispatcher is delivering.
    closed: Cell<bool>,
    /// `None` denotes the active transaction, not an empty registration list.
    leases: RefCell<Option<Vec<ComPtr<IEventSubscriptionAbi>>>>,
}

impl Registry {
    /// Takes the sole ordered vector, excluding registration and dispatch reentry.
    fn take(&self) -> Result<Registrations<'_>, NcStatus> {
        if self.closed.get() {
            return Err(EVENT_E_CLOSED);
        }
        let leases = self.leases.try_borrow_mut().map_err(|_| EVENT_E_BUSY)?.take().ok_or(EVENT_E_BUSY)?;
        Ok(Registrations { registry: self, leases: Some(leases) })
    }
}

/// Owned transaction that restores the precise registration sequence on every exit.
struct Registrations<'a> {
    /// Vacant destination survives foreign calls without lending its borrow guard.
    registry: &'a Registry,
    /// Only this transaction owns the ordered leases while it is active.
    leases: Option<Vec<ComPtr<IEventSubscriptionAbi>>>,
}

impl Deref for Registrations<'_> {
    type Target = Vec<ComPtr<IEventSubscriptionAbi>>;
    /// Borrows only the locally owned sequence.
    fn deref(&self) -> &Self::Target {
        self.leases.as_ref().expect("active registration transaction owns its leases")
    }
}

impl DerefMut for Registrations<'_> {
    /// Mutates only the locally owned sequence.
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.leases.as_mut().expect("active registration transaction owns its leases")
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
        if self.registry.closed.get() {
            let leases = self.leases.as_mut().expect("active transaction owns its leases until drop");
            for lease in leases.iter() {
                let _ = lease.cancel();
            }
            leases.clear();
        }
        *self.registry.leases.borrow_mut() = self.leases.take();
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
                if self.registry.closed.get() {
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
    /// Taking the exact handler type rejects recursive invocation before aliasing.
    handler: RefCell<Option<Handler>>,
}

impl<State, Handler> EventDelivery<State, Handler> {
    /// Invokes one decoded event with an exclusive local state and handler borrow.
    /// Both borrow guards end before a recoverable callback failure is reported.
    pub fn deliver<Event>(&self, event: Event) -> Result<(), NcStatus>
    where
        Handler: EventHandler<State, Event>,
    {
        let mut handler = self.handler.try_borrow_mut().map_err(|_| EVENT_E_BUSY)?.take().ok_or(EVENT_E_BUSY)?;
        let result = match self.state.try_borrow_mut() {
            Ok(mut state) => handler.handle(&mut state, &event),
            Err(_) => Err(EVENT_E_BUSY),
        };
        *self.handler.borrow_mut() = Some(handler);
        result
    }
}

/// Registration regressions use the same transaction and dispatcher as production.
#[cfg(test)]
mod tests;
