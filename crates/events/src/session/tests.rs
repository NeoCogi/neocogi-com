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
struct ProbeConnector<'calls> {
    /// Borrow ends when registration returns; the session cannot retain this probe.
    calls: &'calls mut usize,
    /// Returns the original domain error instead of publishing a lease when set.
    fail: bool,
}

impl<State: 'static, Handler> EventConnector<State, Handler> for ProbeConnector<'_> {
    /// Proves all session reentry is excluded before connection work starts.
    fn connect(self, session: &EventSession<State>, _handler: Handler) -> Result<ComPtr<IEventSubscriptionAbi>, NcStatus> {
        *self.calls += 1;
        assert_eq!(session.dispatch(), Err(EVENT_E_BUSY));
        assert_eq!(session.cancel(), Err(EVENT_E_BUSY));

        // Nested registration must reject its operation without invoking it.
        let mut nested_calls = 0;
        let nested = session.register((), ProbeConnector { calls: &mut nested_calls, fail: false });
        assert_eq!(nested.unwrap_err(), EVENT_E_BUSY);
        assert_eq!(nested_calls, 0);

        match self.fail {
            true => Err(NC_E_INVALID_ARG),
            false => IdleSubscription::from_value(IdleSubscription { cancelled: Cell::new(false) }).query(),
        }
    }
}

/// By-value setup argument whose destruction can be observed without shared cells.
struct DropProbe<'drops> {
    /// Exclusively borrowed counter remains valid only for this registration call.
    drops: &'drops mut usize,
}

impl Drop for DropProbe<'_> {
    /// Marks release when a busy session rejects the uninvoked connector's handler.
    fn drop(&mut self) {
        *self.drops += 1;
    }
}

/// Failed setup restores exact lease identities/order, and subsequent setup succeeds.
#[test]
fn failed_connector_restores_order_and_success_retains_only_leases() {
    let session = EventSession::new(());
    let mut calls = 0;
    let first = session.register((), ProbeConnector { calls: &mut calls, fail: false }).unwrap();
    let second = session.register((), ProbeConnector { calls: &mut calls, fail: false }).unwrap();
    let original = [first.as_mut_ptr(), second.as_mut_ptr()];

    let failure = session.register((), ProbeConnector { calls: &mut calls, fail: true });
    assert_eq!(failure.unwrap_err(), NC_E_INVALID_ARG);
    {
        // Observe the production sequence without replacing its transaction model.
        let transaction = session.registry.take().unwrap();
        let stored = &transaction.leases;
        assert_eq!(stored.len(), 2);
        assert_eq!([stored[0].as_mut_ptr(), stored[1].as_mut_ptr()], original);
    }

    let third = session.register((), ProbeConnector { calls: &mut calls, fail: false }).unwrap();
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
fn busy_session_rejects_connector_before_invocation_and_recovers() {
    let session = EventSession::new(());
    let transaction = session.registry.take().unwrap();
    let mut calls = 0;
    let mut drops = 0;
    let failure = session.register(DropProbe { drops: &mut drops }, ProbeConnector { calls: &mut calls, fail: false });
    assert_eq!(failure.unwrap_err(), EVENT_E_BUSY);
    assert_eq!(calls, 0);
    assert_eq!(drops, 1);

    // Returning the actual transaction restores the idle session, not a fixture copy.
    drop(transaction);
    session.register((), ProbeConnector { calls: &mut calls, fail: false }).unwrap();
    assert_eq!(calls, 1);
}
