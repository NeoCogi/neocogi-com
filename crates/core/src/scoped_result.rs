//! One checked Rust visitor for a synchronous optional interface loan.

use core::{cell::Cell, ffi::c_void, ptr};

use crate::{ComInterface, ComMut, ComRaw, ComRef, NC_E_INVALID_ARG, NcStatus, ThreadingModel};

/// C continuation for delivery of a call-scoped interface loan.
///
/// The provider passes the caller's context unchanged and invokes the callback
/// on the calling thread before returning. Both-null denotes absence. A present
/// pair must remain live under its declared access and threading capability for
/// the complete callback. The callback must not unwind.
#[doc(hidden)]
pub type NcScopedResultCallback<I> = unsafe extern "C" fn(context: *mut c_void, loan: ComRaw<I>);

/// Invokes a scoped getter and returns the Rust visitor's output if it saw a loan.
///
/// The visitor runs once for a valid present pair. The native status and the
/// visitor's return value remain independent. The loan cannot escape the visitor.
///
/// ```compile_fail
/// use neocogi_com::{ComRef, IUnknown, NC_OK, NcStatus, ThreadLocal, __nc_with_scoped_result};
///
/// fn escape() -> Result<Option<ComRef<'static, IUnknown>>, NcStatus> {
///     unsafe { __nc_with_scoped_result::<IUnknown, ThreadLocal, _>(|loan| loan, |_, _| NC_OK) }
/// }
/// ```
///
/// # Safety
///
/// `invoke` may use the callback and context only synchronously on this thread.
/// It must pass the context unchanged and preserve it through every invocation.
/// Present pairs must expose readable common tables and identities. A pair with
/// the exact identity must identify a live matching interface with the declared
/// mode `M`, backed by its lender for the callback's entire execution. The
/// callback, context and loans must not be used after their respective scopes.
#[doc(hidden)]
pub unsafe fn __nc_with_scoped_result<I, M, R>(
    visit: impl for<'loan> FnOnce(ComRef<'loan, I, M>) -> R,
    invoke: impl FnOnce(*mut c_void, NcScopedResultCallback<I>) -> NcStatus,
) -> Result<Option<R>, NcStatus>
where
    I: ComInterface,
    M: ThreadingModel,
{
    let state = VisitorState::new(visit);
    run_visitor(&state, state.shared_callback::<I, M>(), invoke)
}

/// Invokes an exclusive scoped visitor whose callable result remains valid
/// only until that visitor returns.
///
/// # Safety
///
/// `invoke` follows the same synchronous context and pair obligations as
/// [`__nc_with_scoped_result`]. A present pair must additionally carry an
/// exclusive target reborrow for the complete visit.
#[doc(hidden)]
pub unsafe fn __nc_with_scoped_mut_result<I, M, R>(
    visit: impl for<'loan> FnOnce(ComMut<'loan, I, M>) -> R,
    invoke: impl FnOnce(*mut c_void, NcScopedResultCallback<I>) -> NcStatus,
) -> Result<Option<R>, NcStatus>
where
    I: ComInterface,
    M: ThreadingModel,
{
    let state = VisitorState::new(visit);
    run_visitor(&state, state.mutable_callback::<I, M>(), invoke)
}

/// The caller owns the callback context until native return, including any
/// rejected result or unused user visitor that still needs to be dropped.
fn run_visitor<I: ComInterface, F, R>(
    state: &VisitorState<F, R>,
    callback: NcScopedResultCallback<I>,
    invoke: impl FnOnce(*mut c_void, NcScopedResultCallback<I>) -> NcStatus,
) -> Result<Option<R>, NcStatus> {
    let status = invoke(ptr::from_ref(state).cast_mut().cast(), callback);
    match (state.delivery.get(), status.is_ok()) {
        (Delivery::Unvisited, false) => Err(status),
        (Delivery::Once, true) => state.result.take().ok_or(NC_E_INVALID_ARG)?,
        (Delivery::Duplicate, _) | (Delivery::Once, false) | (Delivery::Unvisited, true) => Err(NC_E_INVALID_ARG),
    }
}

/// Delivery is marked before validation, and duplicate delivery is terminal.
#[derive(Clone, Copy)]
enum Delivery {
    /// Native code has not called the continuation.
    Unvisited,
    /// The first delivery is in progress or has completed.
    Once,
    /// Native code attempted at least one additional delivery.
    Duplicate,
}

/// Stack-owned visitor and first-delivery result, isolated to the calling thread.
struct VisitorState<F, R> {
    /// Sticky cardinality across validation, visitor execution and reentry.
    delivery: Cell<Delivery>,
    /// Moved out only when a present exact loan can be delivered.
    visit: Cell<Option<F>>,
    /// First delivery, including canonical absence or a validation failure.
    result: Cell<Option<Result<Option<R>, NcStatus>>>,
}

impl<F, R> VisitorState<F, R> {
    /// Starts a delivery while retaining the visitor until native code supplies a loan.
    fn new(visit: F) -> Self {
        Self {
            delivery: Cell::new(Delivery::Unvisited),
            visit: Cell::new(Some(visit)),
            result: Cell::new(None),
        }
    }

    /// Selects the shared callback for this context's exact Rust visitor type.
    fn shared_callback<I: ComInterface, M: ThreadingModel>(&self) -> NcScopedResultCallback<I>
    where
        F: for<'loan> FnOnce(ComRef<'loan, I, M>) -> R,
    {
        Self::receive::<I, M>
    }

    /// Selects the mutable callback for the matching Rust visitor type.
    fn mutable_callback<I: ComInterface, M: ThreadingModel>(&self) -> NcScopedResultCallback<I>
    where
        F: for<'loan> FnOnce(ComMut<'loan, I, M>) -> R,
    {
        Self::receive_mut::<I, M>
    }

    /// Validates the first pair and records the output without retaining the loan.
    ///
    /// # Safety
    ///
    /// The context is the shared-derived pointer to this invocation's live state.
    /// The raw pair and calling thread satisfy the enclosing invocation contract.
    unsafe extern "C" fn receive<I, M>(context: *mut c_void, raw: ComRaw<I>)
    where
        I: ComInterface,
        M: ThreadingModel,
        F: for<'loan> FnOnce(ComRef<'loan, I, M>) -> R,
    {
        // SAFETY: The invocation supplies this exact state for one synchronous call.
        let state = unsafe { &*context.cast::<Self>() };
        state.deliver(
            // SAFETY: The native call supplies the live declared lender.
            || unsafe { ComRef::<'_, I, M>::__nc_from_raw_optional_borrowed_checked(raw) },
            |visit, loan| visit(loan),
        );
    }

    /// Exclusive entry selects the mutable validator while sharing the same
    /// one-delivery state machine and lifetime-independent visitor result.
    unsafe extern "C" fn receive_mut<I, M>(context: *mut c_void, raw: ComRaw<I>)
    where
        I: ComInterface,
        M: ThreadingModel,
        F: for<'loan> FnOnce(ComMut<'loan, I, M>) -> R,
    {
        // SAFETY: The synchronous caller supplied this exact context.
        let state = unsafe { &*context.cast::<Self>() };
        state.deliver(
            // SAFETY: The native call supplies a live exclusive target loan.
            || unsafe { ComMut::<'_, I, M>::__nc_from_raw_optional_borrowed_checked(raw) },
            |visit, loan| visit(loan),
        );
    }

    /// Mark the callback before invoking foreign code. A reentrant duplicate
    /// changes only cardinality and never consumes the visitor or its output.
    fn deliver<L>(&self, validate: impl FnOnce() -> Result<Option<L>, NcStatus>, invoke: impl FnOnce(F, L) -> R) {
        match self.delivery.get() {
            Delivery::Unvisited => self.delivery.set(Delivery::Once),
            Delivery::Once | Delivery::Duplicate => {
                self.delivery.set(Delivery::Duplicate);
                return;
            }
        }
        let result = validate().and_then(|loan| {
            let Some(loan) = loan else { return Ok(None) };
            let visit = self.visit.take().ok_or(NC_E_INVALID_ARG)?;
            Ok(Some(invoke(visit, loan)))
        });
        self.result.set(Some(result));
    }
}

#[cfg(test)]
mod tests;
