//
// Copyright 2023-Present (c) Raja Lehtihet & Wael El Oraiby
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice,
// this list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice,
// this list of conditions and the following disclaimer in the documentation
// and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its contributors
// may be used to endorse or promote products derived from this software without
// specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE
// LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
// CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
// SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
// INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
// CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
// ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
// POSSIBILITY OF SUCH DAMAGE.

//! Producer-owned typed snapshots and application-owned COM subscriptions.

use std::{
    cell::{Cell, RefCell},
    fmt,
    collections::VecDeque,
    rc::{Rc, Weak},
};
use neocogi_com::{com_object, ComInterface, ComPtr, ComRef, NcStatus};
use neocogi_com_events_contracts::*;
use crate::{EVENT_E_BUSY, EVENT_E_CLOSED, EVENT_E_LIMIT};

/// A concrete semantic snapshot defines its one concrete foreign delivery contract.
pub trait EventPayload: 'static {
    /// Concrete sink interface shared by every snapshot in this native event family.
    type Sink: ComInterface;
    /// Delivers a committed snapshot without reading the live producer or retaining a queue borrow.
    fn deliver(&self, sink: ComRef<'_, Self::Sink>) -> Result<(), NcStatus>;
}
/// Producer-owned queue for one concrete semantic event type.
///
/// Store this in a widget or application component and expose only [`Self::event`] to subscribers.
/// Emission never invokes application code: EventSession dispatches the owned payload after widget
/// traversal releases its borrows. Events emitted without a subscription are discarded.
///
/// The source is deliberately not cloneable. Several internal controls belonging to one component
/// may explicitly share an `Rc<EventSource<E>>`; ordinary widgets own the source directly. Neither
/// its weak endpoint nor a subscription extends the producer's lifetime. Sources belong to the UI
/// thread and are neither `Send` nor `Sync`.
pub struct EventSource<E: 'static> {
    /// Private queue ownership; neither callers nor subscribers can borrow or drain this storage.
    port: Rc<RefCell<Queue<E>>>,
}

impl<E: 'static> EventSource<E> {
    /// Creates a source with no subscriber and no pending payload allocation.
    pub fn new() -> Self {
        Self {
            port: Rc::new(RefCell::new(Queue::new())),
        }
    }

    /// Reports whether producing an owned snapshot would have a current receiver.
    pub fn connected(&self) -> bool {
        matches!(*self.port.borrow(), Queue::Connected { .. })
    }

    /// Finishes a source whose every payload is a completed request, preserving all committed outcomes.
    /// Ordinary widget sources must use close/close_ordinary, which discard obsolete interaction work.
    pub fn finish_pending(&self) {
        self.port.borrow_mut().finish_pending();
    }

    /// Logical widget closure discards ordinary events even while retained COM handles keep its semantic value alive.
    pub fn close_ordinary(&self) {
        let retired = self.port.borrow_mut().close(None);
        drop(retired);
    }

    /// Returns a weak, subscribable endpoint for this source without granting emission access.
    pub fn event(&self) -> EventEndpoint<E> {
        EventEndpoint::new(&self.port)
    }

    /// Queues an owned snapshot after the producer commits the state described by it.
    ///
    /// The short queue borrow ends here. Delivery occurs at EventSession's next dispatch boundary;
    /// callers must not expect their subscribed application handler to have run on return.
    pub fn emit(&self, event: E) {
        let unused = self.port.borrow_mut().emit(event);
        // User-defined payload destruction can reenter the source; no queue borrow survives.
        drop(unused);
    }

    /// Permanently closes this source, preserving one terminal event for its current subscriber.
    ///
    /// Pending ordinary events are discarded. Handles immediately report expired, further emission
    /// is ignored, and subsequent close calls preserve the first terminal event. Without a current
    /// subscriber the terminal event is discarded too; closing never creates replay history.
    pub fn close(&self, terminal: E) {
        let retired = self.port.borrow_mut().close(Some(terminal));
        drop(retired);
    }
}

impl<E: 'static> Drop for EventSource<E> {
    /// Expires endpoints and discards ordinary events without erasing an explicit terminal event.
    fn drop(&mut self) {
        let retired = self.port.borrow_mut().close(None);
        drop(retired);
    }
}

impl<E: 'static> Default for EventSource<E> {
    /// Uses the same disconnected initial state as [`Self::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// One retained-producer-owned typed event queue and its connection state.
///
/// Connected owns queued events; Closed may retain one explicit terminal fact.
/// Finished preserves all completed-request outcomes while rejecting new emission.
/// Disconnected owns no subscription; state is not mirrored by independent flags.
enum Queue<E: 'static> {
    /// No subscription exists; emitted events are discarded.
    Disconnected,
    /// Exactly one subscription exists and owns the right to drain this FIFO.
    Connected {
        /// Ordinary snapshots awaiting the next safe dispatch boundary.
        pending: VecDeque<E>,
    },
    /// A request-completion source finished; every queued outcome remains valid.
    Finished {
        /// Committed request outcomes awaiting their existing consumer, without accepting new emission or subscription.
        pending: VecDeque<E>,
    },
    /// The producer closed or was dropped; only an explicitly published terminal event survives.
    Closed {
        /// One final payload retained by the existing subscription until its next dispatch.
        terminal: Option<E>,
    },
}

impl<E: 'static> Queue<E> {
    /// Creates a disconnected port.
    ///
    /// Widgets construct their ports before they are mounted or subscribed, so the initial state
    /// deliberately has no queue allocation and records no events.
    fn new() -> Self {
        Self::Disconnected
    }

    /// Appends an event when a subscription is connected, otherwise discards it.
    ///
    /// The widget calls this only after committing the semantic or lifecycle state described by
    /// `event`. Delivery is deferred; this method never invokes application code.
    fn emit(&mut self, event: E) -> Option<E> {
        match self {
            Self::Connected { pending } => {
                pending.push_back(event);
                None
            }
            Self::Disconnected | Self::Finished { .. } | Self::Closed { .. } => Some(event),
        }
    }

    /// Transitions a disconnected port to a connected port with an empty queue.
    ///
    /// The exclusive connection enforces the design's one-port/one-state-method rule.
    fn connect(&mut self) -> Result<(), SubscribeError> {
        match self {
            Self::Disconnected => {
                *self = Self::Connected { pending: VecDeque::new() };
                Ok(())
            }
            Self::Connected { .. } => Err(SubscribeError::AlreadySubscribed),
            Self::Finished { .. } | Self::Closed { .. } => Err(SubscribeError::WidgetExpired),
        }
    }

    /// Transitions a connected port to the disconnected state and drops queued payloads.
    ///
    /// Calling this for an already disconnected port is harmless. In normal operation it is called
    /// by the exclusive subscription's [`Drop`] implementation.
    fn disconnect(&mut self) -> Self {
        // Return retired payload ownership for destruction after the caller releases its borrow.
        let replacement = match self {
            Self::Closed { .. } | Self::Finished { .. } => Self::Closed { terminal: None },
            Self::Connected { .. } | Self::Disconnected => Self::Disconnected,
        };
        std::mem::replace(self, replacement)
    }

    /// Closes once, retaining a terminal payload only if a subscriber was connected.
    fn close(&mut self, terminal: Option<E>) -> Self {
        match self {
            Self::Connected { .. } => std::mem::replace(self, Self::Closed { terminal }),
            Self::Disconnected => {
                *self = Self::Closed { terminal: None };
                Self::Closed { terminal } // Return an unobserved candidate for destruction outside the borrow.
            }
            Self::Closed { .. } | Self::Finished { .. } => Self::Closed { terminal }, // Preserve committed terminal facts; retire the unused candidate.
        }
    }

    /// Finishes committed request outcomes without changing their FIFO.
    fn finish_pending(&mut self) {
        match self {
            Self::Connected { pending } => {
                let pending = std::mem::take(pending);
                *self = Self::Finished { pending };
            }
            Self::Disconnected => *self = Self::Finished { pending: VecDeque::new() },
            Self::Closed { .. } | Self::Finished { .. } => {}
        }
    }

    /// Captures the eligible prefix length without moving any unattempted payloads.
    fn window(&self, limit: usize) -> Result<DeliveryWindow, NcStatus> {
        let window = match self {
            Self::Disconnected => DeliveryWindow::Ordinary(0),
            Self::Connected { pending } | Self::Finished { pending } => DeliveryWindow::Ordinary(pending.len()),
            Self::Closed { terminal } => DeliveryWindow::Terminal(usize::from(terminal.is_some())),
        };
        match window {
            DeliveryWindow::Ordinary(count) | DeliveryWindow::Terminal(count) if limit == 0 && count != 0 => Err(EVENT_E_LIMIT),
            DeliveryWindow::Ordinary(count) => Ok(DeliveryWindow::Ordinary(count.min(limit))),
            DeliveryWindow::Terminal(count) => Ok(DeliveryWindow::Terminal(count.min(limit))),
        }
    }
}

/// One dispatch attempt owns only its remaining budget; payloads stay in the FIFO.
/// New emissions wait for the next sweep and closure cannot substitute a terminal
/// fact for an ordinary snapshot whose delivery was already scheduled.
pub enum DeliveryWindow {
    /// Ordinary or committed-request events eligible at the start of this attempt.
    Ordinary(usize),
    /// The single terminal fact already present when this attempt began.
    Terminal(usize),
}

/// Exclusive queue lease owns no producer or callback and cannot cancel a later replacement subscription.
pub struct EventLease<E: 'static> {
    /// Shared queue data alone lets a terminal event outlive its producer.
    port: Rc<RefCell<Queue<E>>>,
    /// Per-lease cancellation distinguishes this lease from a later subscriber to the same live source.
    cancelled: Cell<bool>,
}
impl<E: 'static> EventLease<E> {
    /// Connects before publishing lease ownership; failed exclusive connection changes no existing subscriber.
    pub fn connect(endpoint: EventEndpoint<E>) -> Result<Self, SubscribeError> {
        let port = endpoint.0.upgrade().ok_or(SubscribeError::WidgetExpired)?;
        port.borrow_mut().connect()?;
        Ok(Self { port, cancelled: Cell::new(false) })
    }
    /// Live empty sources stay registered; closed sources survive only until their terminal fact is attempted.
    pub fn active(&self) -> bool {
        if self.cancelled.get() {
            return false;
        }
        match &*self.port.borrow() {
            Queue::Connected { .. } => true,
            Queue::Finished { pending } => !pending.is_empty(),
            Queue::Closed { terminal } => terminal.is_some(),
            Queue::Disconnected => false,
        }
    }
    /// Captures a bounded prefix; callbacks never borrow or detach the pending FIFO.
    pub fn window(&self, limit: usize) -> Result<DeliveryWindow, NcStatus> {
        match self.cancelled.get() {
            true => Ok(DeliveryWindow::Ordinary(0)),
            false => self.port.borrow().window(limit),
        }
    }

    /// Removes only the next attempted event, leaving every unattempted event in place.
    /// Cancellation and closure are observed between callbacks. The returned owned
    /// value is delivered and destroyed after this short queue borrow has ended.
    pub fn next(&self, window: &mut DeliveryWindow) -> Option<E> {
        if self.cancelled.get() {
            return None;
        }
        let mut queue = self.port.borrow_mut();
        match (window, &mut *queue) {
            (DeliveryWindow::Ordinary(count), Queue::Connected { pending } | Queue::Finished { pending }) if *count != 0 => {
                *count -= 1;
                pending.pop_front()
            }
            (DeliveryWindow::Terminal(count), Queue::Closed { terminal }) if *count != 0 => {
                *count -= 1;
                terminal.take()
            }
            _ => None,
        }
    }

    /// Collects a bounded prefix solely for isolated native widget tests.
    #[cfg(any(test, feature = "test-support"))]
    pub fn drain(&self, limit: usize) -> Result<Vec<E>, NcStatus> {
        let mut window = self.window(limit)?;
        Ok(std::iter::from_fn(|| self.next(&mut window)).collect())
    }

    /// Disconnects once and discards ordinary/terminal pending snapshots without invoking user code.
    pub fn cancel(&self) {
        match self.cancelled.replace(true) {
            false => {
                let retired = self.port.borrow_mut().disconnect();
                drop(retired);
            }
            true => {}
        }
    }
}
impl<E: 'static> Drop for EventLease<E> {
    /// An already cancelled old lease cannot disconnect its source's newer subscription.
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Weak, typed capability identifying one event source owned by retained UI.
///
/// Holding or cloning this value does not keep the widget or its pending events alive.
/// The event type parameter prevents connecting a handler for one payload type to another port at
/// compile time.
pub struct EventEndpoint<E: 'static>(Weak<RefCell<Queue<E>>>);

impl<E: 'static> EventEndpoint<E> {
    /// Creates a weak handle to a live retained-producer-owned port.
    fn new(port: &Rc<RefCell<Queue<E>>>) -> Self {
        Self(Rc::downgrade(port))
    }

    /// Creates the same observable state as a handle whose widget has already been removed.
    ///
    /// Projection through an unavailable typed widget handle uses this value so event access stays
    /// weak and fallible without manufacturing an owner.
    pub fn expired() -> Self {
        Self(Weak::new())
    }

    /// Returns whether the concrete retained producer still owns this event source.
    ///
    /// This is a liveness observation, not an ownership claim. The handle remains weak, and a later
    /// attempt to subscribe can still report [`EVENT_E_CLOSED`] if the producer is
    /// removed or closed between operations.
    pub fn is_alive(&self) -> bool {
        self.0
            .upgrade()
            .is_some_and(|port| !matches!(*port.borrow(), Queue::Closed { .. } | Queue::Finished { .. }))
    }
}

impl<E: 'static> Clone for EventEndpoint<E> {
    /// Clones only the weak capability; this neither clones an event nor retains a widget.
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<E: 'static> fmt::Debug for EventEndpoint<E> {
    /// Reports current liveness without exposing port identity or queued payloads.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventEndpoint").field("alive", &self.is_alive()).finish_non_exhaustive()
    }
}

/// Failure to subscribe an application-state method to a retained COM event.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SubscribeError {
    /// The retained producer that owns the event port has already been removed or was unavailable
    /// when its typed handle projected the event capability.
    WidgetExpired,
    /// The event port already has its exclusive subscription.
    ///
    /// In normal application code this means the port is already subscribed through its owning
    /// context.
    AlreadySubscribed,
}

impl fmt::Display for SubscribeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::WidgetExpired => "event owner has expired",
            Self::AlreadySubscribed => "event already has a subscriber",
        })
    }
}

impl std::error::Error for SubscribeError {}

#[com_object(interfaces(IEventSubscription))]
/// One generic local allocation specializes queue and sink types behind a concrete subscription interface.
struct Subscription<E: EventPayload> {
    /// Exclusive queue data and cancellation, shared with the existing native producer implementation.
    lease: EventLease<E>,
    /// Cancellation releases application ownership immediately, even if the lease itself is retained elsewhere.
    sink: RefCell<Option<ComPtr<E::Sink>>>,
    /// Direct reentrant dispatch must reject before claiming a second delivery window.
    dispatching: Cell<bool>,
}

/// Connects the native port before publishing a checked owned subscription interface.
fn subscribe<E: EventPayload>(endpoint: EventEndpoint<E>, sink: ComRef<'_, E::Sink>) -> Result<ComPtr<IEventSubscriptionAbi>, NcStatus> {
    let lease = EventLease::connect(endpoint).map_err(|error| match error {
        SubscribeError::WidgetExpired => EVENT_E_CLOSED,
        SubscribeError::AlreadySubscribed => EVENT_E_BUSY,
    })?;
    Ok(Subscription::from_value(Subscription {
        lease,
        sink: RefCell::new(Some(sink.add_ref())),
        dispatching: Cell::new(false),
    })
    .query()
    .expect("subscription declares IEventSubscription"))
}

impl<E: EventPayload> IEventSubscription for Subscription<E> {
    /// Delivers a bounded FIFO prefix, preserving unattempted events after a callback error.
    fn dispatch(&self, limit: u32) -> Result<u32, NcStatus> {
        if self.dispatching.replace(true) {
            return Err(EVENT_E_BUSY);
        }
        let _guard = DispatchGuard(&self.dispatching);
        let mut window = self.lease.window(limit as usize)?;
        // Take ownership instead of calling foreign AddRef while borrowing the slot.
        let sink = SinkAccess {
            subscription: self,
            value: self.sink.borrow_mut().take(),
        };
        let mut delivered = 0;
        if let Some(sink) = &sink.value {
            while let Some(event) = self.lease.next(&mut window) {
                // The attempted event is consumed even on error; its unattempted
                // successors have never left the queue and need no restoration.
                event.deliver(sink.as_com_ref())?;
                delivered += 1;
            }
        }
        Ok(delivered)
    }
    /// Reports liveness without taking another source or sink reference.
    fn active(&self) -> Result<bool, NcStatus> {
        Ok(self.lease.active())
    }
    /// Cancels once and releases retained application state without invoking the handler.
    fn cancel(&self) -> Result<(), NcStatus> {
        self.lease.cancel();
        self.release_sink();
        Ok(())
    }
}
impl<E: EventPayload> Subscription<E> {
    /// Drops owned application state after releasing the short sink slot borrow, permitting safe destructor reentry.
    fn release_sink(&self) {
        let sink = self.sink.borrow_mut().take();
        drop(sink);
    }
}

/// Synchronous recursion guard owns no producer or callback borrow.
struct DispatchGuard<'a>(
    /// Restored after success, callback error or budget exhaustion.
    &'a Cell<bool>,
);
impl Drop for DispatchGuard<'_> {
    /// Failed delivery cannot leave the subscription permanently busy.
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// Owns the callback capability across delivery; no RefCell borrow or per-batch AddRef crosses user code.
struct SinkAccess<'a, E: EventPayload> {
    /// Existing lease and vacant sink slot remain alive for this synchronous call.
    subscription: &'a Subscription<E>,
    /// Sole sink reference is returned only if cancellation or terminal delivery has not ended it.
    value: Option<ComPtr<E::Sink>>,
}
impl<E: EventPayload> Drop for SinkAccess<'_, E> {
    /// Restores active callback ownership after success/error; inactive sinks drop after all slot borrows end.
    fn drop(&mut self) {
        if self.subscription.lease.active() {
            *self.subscription.sink.borrow_mut() = self.value.take();
        }
    }
}

impl<E: EventPayload> EventEndpoint<E> {
    /// Claims the one producer port and returns an application-owned COM lease.
    /// The producer never retains the sink; cancellation releases it immediately.
    pub fn subscribe(&self, sink: ComRef<'_, E::Sink>) -> Result<ComPtr<IEventSubscriptionAbi>, NcStatus> {
        subscribe(self.clone(), sink)
    }
}
impl<E: EventPayload> EventSource<E> {
    /// Connects a concrete sink using the same exclusive endpoint and bounded FIFO delivery.
    pub fn subscribe(&self, sink: ComRef<'_, E::Sink>) -> Result<ComPtr<IEventSubscriptionAbi>, NcStatus> {
        self.event().subscribe(sink)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neocogi_com::{com_interface, NC_E_INVALID_ARG};
    use crate::EVENT_E_LIMIT;

    #[com_interface(auto_guid = "neocogi.ui.test.EventLeaseSink:v1", assigned_signature = b"00000000000000000000000000000036")]
    /// Independent typed consumer used to exercise generic lease transport.
    trait ITestSink {
        /// Consumes one concrete test value and may return a recoverable application error.
        fn receive(&self, value: i32) -> Result<(), NcStatus>;
    }
    /// One immutable event with no producer or application reference.
    struct TestEvent(i32);
    impl EventPayload for TestEvent {
        type Sink = ITestSinkAbi;
        /// Forwards the actual captured value through the independent contract.
        fn deliver(&self, sink: ComRef<'_, ITestSinkAbi>) -> Result<(), NcStatus> {
            sink.receive(self.0)
        }
    }
    /// Application-owned observations deliberately retain a lease for callback cancellation/reentry.
    #[derive(Default)]
    struct Probe {
        /// Complete callback attempts, including the deliberately failing one.
        values: RefCell<Vec<i32>>,
        /// Retaining this lease would cycle without explicit cancellation releasing its sink.
        lease: RefCell<Option<ComPtr<IEventSubscriptionAbi>>>,
        /// Actual sink destruction count independent of retained lease references.
        drops: Cell<u32>,
        /// Value whose callback publishes another event before failing once.
        fail: Cell<Option<i32>>,
        /// Value whose callback cancels its own subscription during bounded delivery.
        cancel: Cell<Option<i32>>,
    }
    #[com_object(interfaces(ITestSink))]
    /// Concrete application callback owns its producer capability, an allowed direction of ownership.
    struct Sink {
        /// Producer does not own this sink, so only the deliberate application lease reference can cycle.
        source: Rc<EventSource<TestEvent>>,
        /// Shared test observations with no borrowed stack state stored in COM.
        probe: Rc<Probe>,
    }
    impl ITestSink for Sink {
        /// Exercises direct recursion, failure and cancellation without a borrowed subscription sink slot.
        fn receive(&self, value: i32) -> Result<(), NcStatus> {
            let lease = self.probe.lease.borrow().clone().unwrap();
            assert_eq!(lease.dispatch(1), Err(EVENT_E_BUSY));
            self.probe.values.borrow_mut().push(value);
            if self.probe.fail.get() == Some(value) {
                self.probe.fail.set(None);
                self.source.emit(TestEvent(4));
                return Err(NC_E_INVALID_ARG);
            }
            if self.probe.cancel.get() == Some(value) {
                lease.cancel()?;
            }
            Ok(())
        }
    }
    impl Drop for Sink {
        /// Confirms cancellation/terminal delivery can release application state before final lease release.
        fn drop(&mut self) {
            self.probe.drops.set(self.probe.drops.get() + 1);
        }
    }
    /// Builds an application-owned sink and lease over the same native source used by widgets.
    fn connected() -> (Rc<EventSource<TestEvent>>, Rc<Probe>, ComPtr<IEventSubscriptionAbi>) {
        let source = Rc::new(EventSource::new());
        let probe = Rc::new(Probe::default());
        let sink: ComPtr<ITestSinkAbi> = Sink::from_value(Sink {
            source: source.clone(),
            probe: probe.clone(),
        })
        .query()
        .unwrap();
        let lease = subscribe(source.event(), sink.as_com_ref()).unwrap();
        *probe.lease.borrow_mut() = Some(lease.clone());
        (source, probe, lease)
    }
    /// Errors preserve unattempted FIFO snapshots; in-callback cancellation discards the remainder and releases the sink.
    #[test]
    fn callback_errors_and_cancellation_preserve_order_and_release_application_ownership() {
        let (source, probe, lease) = connected();
        for value in [1, 2, 3] {
            source.emit(TestEvent(value));
        }
        probe.fail.set(Some(2));
        assert_eq!(lease.dispatch(0), Err(EVENT_E_LIMIT));
        assert_eq!(lease.dispatch(3), Err(NC_E_INVALID_ARG));
        assert_eq!(*probe.values.borrow(), [1, 2]);
        assert_eq!(lease.dispatch(1), Ok(1));
        assert_eq!(*probe.values.borrow(), [1, 2, 3]);
        source.emit(TestEvent(5));
        probe.cancel.set(Some(4));
        assert_eq!(lease.dispatch(10), Ok(1));
        assert_eq!(*probe.values.borrow(), [1, 2, 3, 4]);
        assert_eq!(probe.drops.get(), 1);
        assert_eq!(lease.active(), Ok(false));
        assert_eq!(lease.dispatch(10), Ok(0));
        // A still-retained cancelled lease no longer owns a sink and cannot
        // disconnect a newer subscriber to the same producer.
        let replacement = EventLease::connect(source.event()).unwrap();
        drop(lease);
        probe.lease.borrow_mut().take();
        source.emit(TestEvent(6));
        assert_eq!(replacement.drain(1).unwrap()[0].0, 6);
    }
    /// Explicit terminal facts survive ordinary queue closure, deliver once, and release the retained application sink.
    #[test]
    fn terminal_delivery_survives_logical_source_closure_once() {
        let (source, probe, lease) = connected();
        source.emit(TestEvent(1));
        source.close(TestEvent(9));
        source.close(TestEvent(10));
        assert_eq!(lease.active(), Ok(true));
        assert_eq!(lease.dispatch(1), Ok(1));
        assert_eq!(*probe.values.borrow(), [9]);
        assert_eq!(lease.active(), Ok(false));
        assert_eq!(probe.drops.get(), 1);
        assert_eq!(lease.dispatch(1), Ok(0));
        probe.lease.borrow_mut().take();
    }
}
#[cfg(test)]
mod lease_tests {
    use super::*;

    /// Bounded prefixes preserve FIFO ahead of newly emitted events and reject exhausted budgets without losing data.
    #[test]
    fn lease_budget_preserves_pending_fifo_and_old_cancellation_cannot_touch_a_replacement() {
        let source = EventSource::<i32>::new();
        let lease = EventLease::connect(source.event()).unwrap();
        assert!(source.connected());
        source.emit(1);
        source.emit(2);
        assert_eq!(lease.drain(0).err(), Some(EVENT_E_LIMIT));
        let first = lease.drain(1).unwrap();
        assert_eq!(first, [1]);
        source.emit(3);
        assert_eq!(lease.drain(2).unwrap(), [2, 3]);
        assert!(lease.active());
        assert!(lease.drain(0).unwrap().is_empty());
        lease.cancel();
        let replacement = EventLease::connect(source.event()).unwrap();
        source.emit(4);
        drop(lease);
        assert_eq!(replacement.drain(1).unwrap(), [4]);
    }

    /// Closure ends an ordinary window; a later window delivers the terminal fact once.
    #[test]
    fn closure_retires_ordinary_window_before_terminal_delivery() {
        let source = EventSource::<i32>::new();
        let lease = EventLease::connect(source.event()).unwrap();
        source.emit(1);
        source.emit(2);
        let mut ordinary = lease.window(2).unwrap();
        assert_eq!(lease.next(&mut ordinary), Some(1));
        source.close(9);
        assert_eq!(lease.next(&mut ordinary), None);
        drop(source);
        assert!(lease.active());
        assert_eq!(lease.drain(1).unwrap(), [9]);
        assert!(!lease.active());
        assert!(lease.drain(1).unwrap().is_empty());
    }

    /// Finished requests survive producer destruction and failures without a restore path.
    #[test]
    fn finalized_request_outcomes_preserve_fifo_without_accepting_new_emission_or_subscribers() {
        let source = EventSource::<i32>::new();
        let endpoint = source.event();
        let lease = EventLease::connect(endpoint.clone()).unwrap();
        for value in [1, 2, 3] {
            source.emit(value);
        }
        let mut window = lease.window(3).unwrap();
        assert_eq!(lease.next(&mut window), Some(1));
        source.finish_pending();
        assert!(!endpoint.is_alive());
        source.emit(4); // Finished sources reject new outcomes.
        source.close(99); // Ordinary closure preserves committed results.
        drop(source);
        assert_eq!(EventLease::connect(endpoint).err(), Some(SubscribeError::WidgetExpired));
        assert_eq!(lease.next(&mut window), Some(2)); // An attempted failing delivery is consumed.
        assert!(lease.active());
        assert_eq!(lease.drain(1).unwrap(), [3]);
        assert!(!lease.active());
        assert!(lease.drain(0).unwrap().is_empty());
    }

    /// Emission during a dispatch window waits for the next sweep even with spare budget.
    #[test]
    fn new_emission_does_not_extend_the_current_window() {
        let source = EventSource::<i32>::new();
        let lease = EventLease::connect(source.event()).unwrap();
        source.emit(1);
        let mut window = lease.window(10).unwrap();
        assert_eq!(lease.next(&mut window), Some(1));
        source.emit(2);
        assert_eq!(lease.next(&mut window), None);
        assert_eq!(lease.drain(10).unwrap(), [2]);
    }
}

#[cfg(test)]
mod payload_drop_tests {
    use super::*;

    /// User payload destructor deliberately reenters its live producer.
    struct Payload {
        /// Weak ownership prevents the payload itself from retaining the producer.
        source: Weak<EventSource<Payload>>,
        /// Shared count proves every discarded payload is released once.
        drops: Rc<Cell<usize>>,
    }
    impl Drop for Payload {
        /// Source inspection must succeed while discarded payloads are being retired.
        fn drop(&mut self) {
            if let Some(source) = self.source.upgrade() {
                let _connected = source.connected();
            }
            self.drops.set(self.drops.get() + 1);
        }
    }
    /// Disconnected emission, cancellation and both terminal-close paths release payloads outside queue borrows.
    #[test]
    fn discarded_payload_destructors_can_reenter_source_observation() {
        let source = Rc::new(EventSource::new());
        let drops = Rc::new(Cell::new(0));
        source.emit(Payload {
            source: Rc::downgrade(&source),
            drops: drops.clone(),
        });
        assert_eq!(drops.get(), 1);
        let lease = EventLease::connect(source.event()).unwrap();
        source.emit(Payload {
            source: Rc::downgrade(&source),
            drops: drops.clone(),
        });
        lease.cancel();
        assert_eq!(drops.get(), 2);
        let replacement = EventLease::connect(source.event()).unwrap();
        source.emit(Payload {
            source: Rc::downgrade(&source),
            drops: drops.clone(),
        });
        source.close(Payload {
            source: Rc::downgrade(&source),
            drops: drops.clone(),
        });
        assert_eq!(drops.get(), 3);
        source.close(Payload {
            source: Rc::downgrade(&source),
            drops: drops.clone(),
        });
        assert_eq!(drops.get(), 4);
        replacement.cancel();
        assert_eq!(drops.get(), 5);
    }
}
