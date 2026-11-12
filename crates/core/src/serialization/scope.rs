//! Call-scoped exclusive format objects for borrowed or fixed-capacity storage.
//!
//! These adapters implement the ordinary COM interface while refusing AddRef,
//! owning queries and weak references. A capability cannot outlive its lender.
//! Owned format objects use the facade's ordinary COM allocation/lifetime path.

use core::{
    cell::{Cell, UnsafeCell},
    ffi::c_void,
    ops::{Deref, DerefMut},
};
use crate::{ComInterface, ComRaw, ComRef, IUnknown, NcInterfaceId, NcStatus, NC_E_BUSY, NC_E_INVALID_ARG, NC_E_POINTER};

/// Owns one provider while lending nonretainable COM references to its state.
/// UnsafeCell is required by foreign aliasing; the busy gate excludes reentrant
/// mutable calls before any Rust reference to the provider is constructed.
pub struct Scoped<T> {
    /// Provider storage accessed only under the call gate.
    value: UnsafeCell<T>,
    /// Whether an ABI method currently has exclusive access to the provider.
    busy: Cell<bool>,
}
impl<T> Scoped<T> {
    /// Starts an idle scope, retaining all borrowed data inside T's own lifetime.
    pub fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
            busy: Cell::new(false),
        }
    }
    /// Recovers the provider after every borrowed capability and call has ended.
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
    /// Lends a correctly typed capability without acquiring an ownership token.
    ///
    /// # Safety
    /// The table's thunks must use this exact Scoped<T> storage, reject retained
    /// ownership, and guard all provider access with borrow_mut. I's metadata must
    /// describe that table. The Rust borrow keeps both the scope and T alive.
    pub unsafe fn interface<I: ComInterface>(&self, table: &'static I::VTable) -> ComRef<'_, I> {
        // SAFETY: the caller proves the typed table; this borrow retains storage.
        unsafe {
            ComRef::from_raw_borrowed(ComRaw {
                object: (self as *const Self).cast_mut().cast(),
                vtable: table,
            })
            .unwrap()
        }
    }
    /// Acquires one typed lease before touching foreign-addressed provider state.
    ///
    /// # Safety
    /// this must identify a live Scoped<T>, retained for the complete lease. All
    /// other accesses must use the same gate. Calls stay on the lending thread.
    pub unsafe fn borrow_mut<'a>(this: *mut c_void) -> Result<Lease<'a, T>, NcStatus>
    where
        T: 'a,
    {
        // SAFETY: the caller supplies this scope's exact live allocation.
        let scope = unsafe { this.cast::<Self>().as_ref() }.ok_or(NC_E_POINTER)?;
        if scope.busy.replace(true) {
            return Err(NC_E_BUSY);
        }
        Ok(Lease { scope })
    }
}
/// Exclusive provider access; no T reference survives releasing its gate.
pub struct Lease<'a, T> {
    /// Retained scope, including its shared reentrancy gate and mutable slot.
    scope: &'a Scoped<T>,
}
impl<T> Deref for Lease<'_, T> {
    type Target = T;
    /// Reborrows initialized state only while this lease holds the gate.
    fn deref(&self) -> &T {
        // SAFETY: this lease still holds the sole provider admission.
        unsafe { &*self.scope.value.get() }
    }
}
impl<T> DerefMut for Lease<'_, T> {
    /// The gate proves exclusive access for this reborrow.
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: the gate excludes every other provider reference.
        unsafe { &mut *self.scope.value.get() }
    }
}
impl<T> Drop for Lease<'_, T> {
    /// Releases admission after all temporary provider references have ended.
    fn drop(&mut self) {
        self.scope.busy.set(false);
    }
}
/// Rejects owning projections, clearing the output as required by IUnknown.
///
/// # Safety
/// The caller lends a live scope and a valid distinct writable output slot.
pub unsafe extern "C" fn query(this: *mut c_void, requested: *const NcInterfaceId, output: *mut ComRaw<IUnknown>) -> NcStatus {
    if output.is_null() {
        return NC_E_POINTER;
    }
    unsafe {
        *output = ComRaw::null();
    }
    if this.is_null() || requested.is_null() {
        return NC_E_POINTER;
    }
    NC_E_INVALID_ARG
}
/// Rejects ownership promotion for a call-scoped exclusive object.
///
/// # Safety
/// The caller lends the scope for this call; no reference is acquired.
pub unsafe extern "C" fn retain(_this: *mut c_void) -> u32 {
    0
}
/// Defensive no-op: a scoped object never grants a releasable ownership token.
///
/// # Safety
/// Calls with a purported ownership token violate the scoped object's contract.
pub unsafe extern "C" fn release(_this: *mut c_void) -> u32 {
    0
}
