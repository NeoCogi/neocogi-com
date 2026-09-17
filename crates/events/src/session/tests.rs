//! Focused registration tests over the production session and checked COM leases.

use super::*;
use std::cell::Cell;
use neocogi_com_events_contracts::{IEventSubscription, IEventSubscriptionInterfacesFor};

#[com_object(interfaces(IEventSubscription))]
/// Deterministic foreign lease with no pending events and observable cancellation.
struct IdleSubscription {
    /// COM cancellation takes `&self`, so this fixture needs one shared flag.
    cancelled: Cell<bool>,
}

impl IEventSubscription for IdleSubscription {
    /// Models an empty FIFO without inventing any application callback behavior.
    fn dispatch(&self, _limit: u32) -> Result<u32, NcStatus> {
        Ok(0)
    }

    /// Keeps the lease eligible until the production session cancels it.
    fn active(&self) -> Result<bool, NcStatus> {
        Ok(!self.cancelled.get())
    }

    /// Makes cancellation idempotent and visible through retained COM references.
    fn cancel(&self) -> Result<(), NcStatus> {
        self.cancelled.set(true);
        Ok(())
    }
}

/// Consumed connection operation with an ordinary exclusive stack observation.
struct ProbePort<'calls> {
    /// Borrow ends when registration returns; the session cannot retain this probe.
    calls: &'calls mut usize,
    /// Returns the original domain error instead of publishing a lease when set.
    fail: bool,
}

impl EventPort for ProbePort<'_> {
    type Event = ();
    /// Proves all session reentry is excluded before connection work starts.
    fn connect<State: 'static, Handler: EventHandler<State, ()> + 'static>(
        self,
        session: &EventSession<State>,
        _handler: Handler,
    ) -> Result<ComPtr<IEventSubscriptionAbi>, NcStatus> {
        *self.calls += 1;
        assert_eq!(session.dispatch(), Err(EVENT_E_BUSY));
        assert_eq!(session.cancel(), Err(EVENT_E_BUSY));

        // Nested registration must reject its operation without invoking it.
        let mut nested_calls = 0;
        let nested = session.subscribe(ProbePort { calls: &mut nested_calls, fail: false }, Ignore);
        assert_eq!(nested.unwrap_err(), EVENT_E_BUSY);
        assert_eq!(nested_calls, 0);

        match self.fail {
            true => Err(NC_E_INVALID_ARG),
            false => IdleSubscription::from_value(IdleSubscription { cancelled: Cell::new(false) }).query(),
        }
    }
}

/// Rejected handler whose destruction is observed independently of session state.
struct DropProbe {
    /// Shared observation outlives the rejected handler without borrowing stack state.
    drops: Rc<Cell<usize>>,
}

/// Explicit inert handler for setup-only tests; no callback closure is hidden.
struct Ignore;
impl<State> EventHandler<State, ()> for Ignore {
    /// The idle test source never dispatches, but supplies a correctly typed handler.
    fn handle(&mut self, _state: &mut State, _event: &()) -> Result<(), NcStatus> {
        Ok(())
    }
}
impl EventHandler<(), ()> for DropProbe {
    /// Delivery is inert; only setup rejection and handler destruction are observed.
    fn handle(&mut self, _state: &mut (), _event: &()) -> Result<(), NcStatus> {
        Ok(())
    }
}

impl Drop for DropProbe {
    /// Marks release when a busy session rejects the uninvoked port's handler.
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

/// Failed setup restores exact lease identities/order, and subsequent setup succeeds.
#[test]
fn failed_port_restores_order_and_success_retains_only_leases() {
    let session = EventSession::new(());
    let mut calls = 0;
    let first = session.subscribe(ProbePort { calls: &mut calls, fail: false }, Ignore).unwrap();
    let second = session.subscribe(ProbePort { calls: &mut calls, fail: false }, Ignore).unwrap();
    let original = [first.as_mut_ptr(), second.as_mut_ptr()];

    let failure = session.subscribe(ProbePort { calls: &mut calls, fail: true }, Ignore);
    assert_eq!(failure.unwrap_err(), NC_E_INVALID_ARG);
    {
        // Observe the production sequence without replacing its transaction model.
        let transaction = session.registry.take().unwrap();
        let stored = &transaction.leases;
        assert_eq!(stored.len(), 2);
        assert_eq!([stored[0].as_mut_ptr(), stored[1].as_mut_ptr()], original);
    }

    let third = session.subscribe(ProbePort { calls: &mut calls, fail: false }, Ignore).unwrap();
    assert_eq!(calls, 4);
    drop(second);
    drop(third);
    assert_eq!(session.registry.take().unwrap().len(), 3);
    assert_eq!(session.dispatch(), Ok(0));
    session.cancel().unwrap();
    assert_eq!(first.active(), Ok(false));
    assert!(session.registry.take().unwrap().is_empty());
}

/// Busy registration releases its supplied values without invoking connection work.
#[test]
fn busy_session_rejects_port_before_invocation_and_recovers() {
    let session = EventSession::new(());
    let transaction = session.registry.take().unwrap();
    let mut calls = 0;
    let drops = Rc::new(Cell::new(0));
    let failure = session.subscribe(ProbePort { calls: &mut calls, fail: false }, DropProbe { drops: drops.clone() });
    assert_eq!(failure.unwrap_err(), EVENT_E_BUSY);
    assert_eq!(calls, 0);
    assert_eq!(drops.get(), 1);

    // Returning the actual transaction restores the idle session, not a fixture copy.
    drop(transaction);
    session.subscribe(ProbePort { calls: &mut calls, fail: false }, Ignore).unwrap();
    assert_eq!(calls, 1);
}
