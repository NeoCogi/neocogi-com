//! Synchronous COM bridge for a borrowed Rust consumer, with revocation on every exit.

use std::{cell::Cell, rc::Rc};
use neocogi_com::{com_object, ComRef, NcStatus};
use neocogi_com_events_contracts::{IEventDispatcher, IEventDispatcherAbi, IEventDispatcherInterfacesFor};
use crate::EVENT_E_CLOSED;

/// Exact invocation pair created together by one monomorphized Rust trampoline.
#[derive(Clone, Copy)]
struct Invocation {
    /// Borrowed callback address; dereferenced only while its enclosing scope remains live.
    context: *mut (),
    /// The matching trampoline restores the callback's concrete type before invoking it.
    call: unsafe fn(*mut (), u32) -> Result<u32, NcStatus>,
}
/// A retained foreign capability is callable only inside its original synchronous scope.
#[derive(Clone, Copy)]
enum State {
    /// The stack callback is live and no invocation currently borrows it.
    Ready(Invocation),
    /// The enclosing call returned or unwound; no stack address remains accessible.
    Closed,
}
#[com_object(interfaces(IEventDispatcher))]
/// Local COM allocation exposes only dispatch, never the concrete callback or application state.
struct ScopedDispatcher {
    /// Shared only with the stack guard that revokes foreign access on scope exit.
    state: Rc<Cell<State>>,
}
impl IEventDispatcher for ScopedDispatcher {
    /// Borrows the callback exclusively and restores availability after success, failure or panic.
    fn dispatch(&mut self, limit: u32) -> Result<u32, NcStatus> {
        match self.state.get() {
            State::Ready(invocation) => {
                // SAFETY: the scope keeps this exact pair live. The generated
                // mutable COM call gate excludes recursive access before a second
                // callback borrow could be manufactured.
                unsafe { (invocation.call)(invocation.context, limit) }
            }
            State::Closed => Err(EVENT_E_CLOSED),
        }
    }
}
/// Invalidates all retained COM references before the borrowed callback can leave scope.
struct Scope(
    /// Revocation state shared with every retained COM projection.
    Rc<Cell<State>>,
);
impl Drop for Scope {
    /// Revokes the stack address even when the foreign operation unwinds.
    fn drop(&mut self) {
        self.0.set(State::Closed);
    }
}
/// Restores only the concrete callback type associated with this invocation's context.
unsafe fn invoke<Callback: FnMut(u32) -> Result<u32, NcStatus>>(context: *mut (), limit: u32) -> Result<u32, NcStatus> {
    // SAFETY: with_dispatcher creates this pointer from Callback, and the scope
    // and generated mutable call gate admit only one live borrow at a time.
    unsafe { (&mut *context.cast::<Callback>())(limit) }
}
/// Gives a synchronous runtime call dispatch access to a borrowed native closure.
/// Retaining the capability is harmless: after return it permanently reports CLOSED.
/// This is framework plumbing; application code uses the UI's single update operation.
#[doc(hidden)]
pub fn with_dispatcher<Callback, ResultValue>(callback: &mut Callback, run: impl FnOnce(ComRef<'_, IEventDispatcherAbi>) -> ResultValue) -> ResultValue
where
    Callback: FnMut(u32) -> Result<u32, NcStatus>,
{
    let scope = Scope(Rc::new(Cell::new(State::Ready(Invocation {
        context: core::ptr::from_mut(callback).cast(),
        call: invoke::<Callback>,
    }))));
    let dispatcher = ScopedDispatcher::from_value(ScopedDispatcher { state: scope.0.clone() })
        .query::<IEventDispatcherAbi>()
        .expect("scoped dispatcher declares its interface");
    run(dispatcher.as_com_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    /// The bridge borrows stack state only during the call and revokes retained projections.
    #[test]
    fn borrowed_callback_is_revoked_on_return() {
        let mut total = 0;
        let mut retained = with_dispatcher(
            &mut |count| {
                total += count;
                Ok(count)
            },
            |mut dispatcher| {
                assert_eq!(dispatcher.dispatch(3), Ok(3));
                dispatcher.add_ref()
            },
        );
        assert_eq!(total, 3);
        assert_eq!(retained.dispatch(2), Err(EVENT_E_CLOSED));
    }
    /// An outer panic also revokes a foreign reference before local state goes out of scope.
    #[test]
    fn panic_revokes_the_borrow() {
        let mut retained = None;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_dispatcher(&mut |_| Ok(0), |dispatcher| {
                retained = Some(dispatcher.add_ref());
                panic!("scope exit");
            });
        }));
        assert!(result.is_err());
        assert_eq!(retained.unwrap().dispatch(1), Err(EVENT_E_CLOSED));
    }
    /// The existing mutable COM gate rejects recursion before a second callback borrow exists.
    #[test]
    fn mutable_dispatch_gate_excludes_recursive_borrows() {
        let retained = std::cell::RefCell::new(None::<neocogi_com::ComPtr<IEventDispatcherAbi>>);
        with_dispatcher(
            &mut |_| {
                assert_eq!(retained.borrow_mut().as_mut().unwrap().dispatch(1), Err(neocogi_com::NC_E_BUSY));
                Ok(1)
            },
            |mut dispatcher| {
                *retained.borrow_mut() = Some(dispatcher.add_ref());
                assert_eq!(dispatcher.dispatch(1), Ok(1));
            },
        );
        assert_eq!(retained.borrow_mut().as_mut().unwrap().dispatch(1), Err(EVENT_E_CLOSED));
    }
}
