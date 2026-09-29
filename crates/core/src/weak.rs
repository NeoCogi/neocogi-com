//! Typed weak COM ownership over an allocator-independent C lifetime protocol.
//!
//! Weak references retain only lifetime bookkeeping. Promotion owns a strong
//! reference before exposing any object state, and never resurrects a destroyed
//! value. Logical domain closure is deliberately independent of this protocol.
use core::{marker::PhantomData, ptr::NonNull};
use crate::lifetime::{NcWeakControl, nc_weak_retain, nc_weak_release, nc_weak_upgrade};
use crate::{ComInterface, ComPtr, ComRaw, IUnknown, NcInterfaceId, NcStatus, ThreadLocal, ThreadSafe, ThreadingModel, NC_E_CLOSED, NC_E_NOINTERFACE, NC_E_POINTER};

/// One nullable pointer to the common COM lifetime header. Success transfers
/// one weak reference; a null pointer represents absence and owns nothing.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcWeakRaw {
    /// Header at the allocation's canonical object address, retained through
    /// shared weak operations even after the concrete value is destroyed.
    pub control: *mut NcWeakControl,
}
impl NcWeakRaw {
    /// Empty output initialized before every fallible lifetime operation.
    pub const fn null() -> Self {
        Self { control: core::ptr::null_mut() }
    }
    /// Presence proves shape only; foreign validity remains an unsafe obligation.
    pub fn is_present(self) -> bool {
        !self.control.is_null()
    }
}

/// A typed non-owning reference to a COM object's lifetime.
///
/// Clone/drop retain/release bookkeeping only. `upgrade` returns a temporary
/// strong owner, or None after final strong release. It does not test widget
/// closure or current form membership. Queued event payloads remain independently
/// owned even when their origin expires. The threading marker preserves exactly
/// the originating handle's permissions, including after object destruction.
///
/// Thread-local weak references cannot be sent or shared with another thread:
/// ```compile_fail
/// use neocogi_com_core::{WeakComPtr, IUnknown};
/// fn require_send<T: Send>() {}
/// require_send::<WeakComPtr<IUnknown>>();
/// ```
/// ```compile_fail
/// use neocogi_com_core::{WeakComPtr, IUnknown};
/// fn require_sync<T: Sync>() {}
/// require_sync::<WeakComPtr<IUnknown>>();
/// ```
#[repr(transparent)]
pub struct WeakComPtr<T: ComInterface + ?Sized, M: ThreadingModel = ThreadLocal> {
    /// Exactly one provider-owned weak reference; safe construction requires presence.
    control: NonNull<NcWeakControl>,
    /// Keeps the requested interface and threading permissions in the static type.
    marker: PhantomData<(*const T, M)>,
}
impl<T: ComInterface + ?Sized, M: ThreadingModel> WeakComPtr<T, M> {
    /// Adopts an owned weak token for this exact interface and threading capability.
    ///
    /// # Safety
    /// A present token must own one weak reference to the common allocation header.
    /// Its provider must support T with capability M and uphold atomic promotion,
    /// unique control identity, allocator-local release, and code lifetime through
    /// all weak references. Null tokens transfer nothing and are rejected.
    pub unsafe fn from_raw_owned(raw: NcWeakRaw) -> Option<Self> {
        NonNull::new(raw.control).map(|control| Self { control, marker: PhantomData })
    }
    /// Borrows the control identity without extending or transferring its lifetime.
    pub fn as_raw(&self) -> NcWeakRaw {
        NcWeakRaw { control: self.control.as_ptr() }
    }
    /// Transfers this handle's one weak reference to an ABI caller.
    pub fn into_raw(self) -> NcWeakRaw {
        let raw = self.as_raw();
        core::mem::forget(self);
        raw
    }
    /// Acquires the original object if it still exists, validating its exact contract.
    /// Destruction returns None; interface/signature/provider errors remain errors.
    pub fn upgrade(&self) -> Result<Option<ComPtr<T, M>>, NcStatus> {
        // SAFETY: self retains the provider token and its original threading mode.
        let raw = unsafe { upgrade(self.as_raw(), &T::INTERFACE_ID) }?;
        match raw {
            Some(raw) => {
                // SAFETY: the shared helper validated T and transfers one strong owner.
                Ok(unsafe {
                    ComPtr::from_raw_owned(ComRaw::<T> {
                        object: raw.object,
                        vtable: raw.vtable.cast(),
                    })
                })
            }
            None => Ok(None),
        }
    }
    /// Transfers this token to the universal COM identity without promotion.
    ///
    /// Every valid COM object supports IUnknown with the same threading capability.
    /// This consumes the existing weak owner without cloning it, querying the
    /// value, or extending its strong lifetime. It is useful for diagnostics and
    /// heterogeneous identity collections; event payloads retain their own types.
    pub fn into_unknown(self) -> WeakComPtr<IUnknown, M> {
        let control = self.control;
        core::mem::forget(self);
        WeakComPtr { control, marker: PhantomData }
    }

    /// Compares canonical lifetime identities without promoting either object.
    /// Retaining the control blocks prevents address reuse while comparison is possible.
    pub fn same_object<U: ComInterface + ?Sized, N: ThreadingModel>(&self, other: &WeakComPtr<U, N>) -> bool {
        self.control == other.control
    }
}
impl<T: ComInterface + ?Sized, M: ThreadingModel> Clone for WeakComPtr<T, M> {
    /// Retains the same weak control identity, without affecting the strong lifetime.
    fn clone(&self) -> Self {
        // SAFETY: this handle owns a live token while its provider retains another.
        unsafe { nc_weak_retain(self.control.as_ptr()) };
        Self {
            control: self.control,
            marker: PhantomData,
        }
    }
}
impl<T: ComInterface + ?Sized, M: ThreadingModel> Drop for WeakComPtr<T, M> {
    /// Releases exactly this weak reference through the allocating provider.
    fn drop(&mut self) {
        // SAFETY: the owned token is consumed once and never accessed afterwards.
        unsafe { nc_weak_release(self.control.as_ptr()) };
    }
}
impl<T: ComInterface + ?Sized, M: ThreadingModel> core::fmt::Debug for WeakComPtr<T, M> {
    /// Reports identity only; formatting does not promote or access the object value.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("WeakComPtr").field(&self.control).finish()
    }
}
// SAFETY: the original ThreadSafe capability requires concurrent lifetime operations
// and permits final destruction on any thread. Local weak handles gain neither trait.
unsafe impl<T: ComInterface + ?Sized> Send for WeakComPtr<T, ThreadSafe> {}
// SAFETY: as above; immutable sharing only invokes the provider's atomic weak protocol.
unsafe impl<T: ComInterface + ?Sized> Sync for WeakComPtr<T, ThreadSafe> {}

/// Shares provider validation across all typed weak handles instead of duplicating it.
///
/// # Safety
/// Source must be a valid borrowed weak token; expected identity must be readable.
#[inline(never)]
unsafe fn upgrade(source: NcWeakRaw, expected: &NcInterfaceId) -> Result<Option<ComRaw<IUnknown>>, NcStatus> {
    let mut out = ComRaw::null();
    // SAFETY: caller retains the weak token; both argument records are live and distinct.
    let status = unsafe { nc_weak_upgrade(source.control, expected, &mut out) };
    match status {
        NC_E_CLOSED => Ok(None),
        status if status.is_err() => Err(status),
        _ => {
            // SAFETY: successful provider calls transfer a readable common strong pair.
            match unsafe { crate::raw::__nc_validate_iunknown_interface(out, expected) } {
                Ok(()) => Ok(Some(out)),
                Err(error) => {
                    // SAFETY: reject a successful transfer without leaking its strong owner.
                    unsafe { crate::raw::release(out) };
                    Err(error)
                }
            }
        }
    }
}

/// Obtains a provider weak token while a strong reference keeps its source alive.
///
/// # Safety
/// Source is a matching live object/table pair with the caller's threading capability.
#[inline(never)]
pub(crate) unsafe fn downgrade(source: ComRaw<IUnknown>) -> Result<NcWeakRaw, NcStatus> {
    // SAFETY: caller grants access to the current core ABI's common table prefix.
    let operation = unsafe { (*source.vtable).downgrade }.ok_or(NC_E_NOINTERFACE)?;
    let mut out = NcWeakRaw::null();
    // SAFETY: source remains strong-owned for this synchronous operation.
    let status = unsafe { operation(source.object, &mut out) };
    if status.is_err() {
        return Err(status);
    }
    if !out.is_present() {
        return Err(NC_E_POINTER);
    }
    Ok(out)
}
