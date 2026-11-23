//! One checked Rust visitor for a synchronous optional interface loan.

use core::{cell::Cell, ffi::c_void, ptr};

use crate::{ComInterface, ComRaw, ComRef, NC_E_INVALID_ARG, NcStatus, ThreadingModel};

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
    // The native context borrows this stack state as shared storage. The callback
    // writes only its Cells, and neither moves or retains the context.
    let status = invoke(ptr::from_ref(&state).cast_mut().cast(), state.callback::<I, M>());
    match (state.delivery.get(), status.is_ok()) {
        (Delivery::Unvisited, false) => Err(status),
        (Delivery::Once, true) => state.result.into_inner().ok_or(NC_E_INVALID_ARG)?,
        // Keep both a rejected result and an unused visitor in this context until
        // the provider returns, so their destructors run after its guard releases.
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

    /// Selects the typed continuation for this context's visitor.
    fn callback<I, M>(&self) -> NcScopedResultCallback<I>
    where
        I: ComInterface,
        M: ThreadingModel,
        F: for<'loan> FnOnce(ComRef<'loan, I, M>) -> R,
    {
        Self::receive::<I, M>
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
        match state.delivery.get() {
            Delivery::Unvisited => state.delivery.set(Delivery::Once),
            Delivery::Once | Delivery::Duplicate => {
                state.delivery.set(Delivery::Duplicate);
                return;
            }
        }

        // SAFETY: The current callback retains the lending provider and raw
        // backing. Validation checks shape and exact identity. R is independent
        // of the lifetime granted to the visitor.
        let loan = unsafe { ComRef::<'_, I, M>::__nc_from_raw_optional_borrowed_checked(raw) };
        let result = loan.and_then(|loan| {
            let Some(loan) = loan else { return Ok(None) };
            // The first valid delivery owns F. Taking it leaves no mutable
            // reference to the context while the visitor can call foreign code.
            let visit = state.visit.take().ok_or(NC_E_INVALID_ARG)?;
            Ok(Some(visit(loan)))
        });
        // A reentrant duplicate records its phase without touching this result.
        state.result.set(Some(result));
    }
}

#[cfg(test)]
mod tests;
