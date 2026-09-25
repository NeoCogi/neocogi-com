//! Owning and borrowed smart pointers for reference-counted COM interfaces.
//!
//! [`ComPtr`] models one owned reference and performs `AddRef`/`Release` on
//! clone/drop, while [`ComRef`] represents a call-scoped borrowed pointer.
//! Threading marker types make the capability carried by each pointer visible
//! to Rust without changing the underlying C pointer representation.
//!
//! # Thread-safety table
//!
//! | Rust handle | Ownership and reference-count behavior | `Send` | `Sync` | Safe weakening |
//! |---|---|:---:|:---:|---|
//! | `ComPtr<T, ThreadLocal>` | Owns one reference; clone calls `AddRef`, drop calls `Release` | No | No | None |
//! | `ComPtr<T, ThreadSafe>` | Owns one reference; clone calls `AddRef`, drop calls `Release` | Yes | Yes | `into_thread_local` or `as_thread_local_ref` |
//! | `ComRef<'a, T, ThreadLocal>` | Borrows a reference; copy and drop do not touch the count | No | No | None |
//! | `ComRef<'a, T, ThreadSafe>` | Borrows a reference; copy and drop do not touch the count | Yes | Yes | `into_thread_local` |
//!
//! [`ComRef::add_ref`] is the explicit exception for a borrowed handle: it
//! calls `AddRef` and returns an owning [`ComPtr`] with the same threading
//! marker. [`ComPtr::query`] likewise preserves the source marker because
//! `QueryInterface` selects another interface on the same object; it cannot
//! strengthen that object's threading guarantee.
//!
//! # Why the marker is necessary
//!
//! The C ABI pointer contains only an interface-header address. It has the same
//! representation for thread-local and thread-safe objects and therefore has
//! no runtime bit from which Rust could infer `Send` or `Sync`. The interface
//! type `T` cannot provide the answer either: the same interface contract may
//! be implemented by both a thread-affine object and a thread-safe object. Once a
//! concrete implementation has been returned as `ComPtr<T, M>`, its Rust type has
//! been erased, so [`crate::ComInterface`] identifies the vtable contract but
//! not the implementation's threading behavior.
//!
//! An IDL interface declares a minimum threading capability and a plugin
//! descriptor records its factory implementation capability, but Rust
//! auto traits cannot depend on metadata inspected at runtime: a value's `Send`
//! and `Sync` implementations must be known from its type. Generated factory
//! bindings bridge the two systems and return a statically marked
//! `ComPtr<T, M>` only after activation validation.
//!
//! `M` preserves the producer's threading promise across that erasure. Rust can
//! then decide the auto-trait behavior of the handle at compile time:
//!
//! - `Send` permits moving an owner to another thread. For [`ComPtr`], this also
//!   permits the final `Release` and object destruction to happen there.
//! - `Sync` permits sharing `&ComPtr` or `&ComRef` between threads, so calls may
//!   occur concurrently through aliases.
//! - `ThreadLocal` deliberately grants neither permission. Atomic reference
//!   counting protects only the counter; it does not make methods, contained
//!   state, foreign resources, or destruction thread-safe.
//!
//! `ThreadSafe` is positive evidence supplied by a generated IDL description,
//! checked factory activation, or an unsafe raw-pointer boundary. Weakening
//! it to `ThreadLocal` is safe because that removes permissions. The reverse
//! conversion would invent evidence that the pointer does not contain, so no
//! safe upgrade is provided. `ThreadLocal` consequently describes the current
//! handle's capability, not an immutable property of the object: a thread-safe
//! object may simultaneously have thread-safe aliases and deliberately weakened
//! thread-local aliases.

use core::marker::PhantomData;
use core::ptr::NonNull;

use crate::{ComInterface, NC_E_POINTER, NcAbiThreadingMode, NcStatus};

/// Sealed marker for the compile-time threading capability of a COM handle.
///
/// The marker is part of the Rust type only; C ABI interface pointers have the
/// same representation regardless of threading mode. Sealing is important
/// because the crate's safe `Send` and `Sync` implementations recognize the two
/// supported capabilities and must not accept downstream-invented modes with
/// unspecified semantics.
pub trait ThreadingModel: sealed::Sealed + 'static {
    /// Canonical ABI signature identity for this pointer threading mode.
    const ABI_THREADING_MODE: NcAbiThreadingMode;
}

/// Default pointer mode carrying no permission for cross-thread use.
///
/// This restricts the particular pointer, not every alias to the underlying
/// object. A thread-safe object may be viewed through a `ThreadLocal` pointer as
/// a conservative capability downgrade. The marker deliberately makes both
/// owning and borrowed handles neither `Send` nor `Sync`.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ThreadLocal {
    _not_send_sync: PhantomData<*mut ()>,
}

/// Pointer mode positively guaranteeing cross-thread calls and destruction.
///
/// Handles carrying this marker are `Send` and `Sync`, so the producer must
/// guarantee safe concurrent method calls and final release on an arbitrary
/// thread. The marker records that pre-existing guarantee; it does not create
/// synchronization or make a thread-local object safe.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ThreadSafe;

impl ThreadingModel for ThreadLocal {
    const ABI_THREADING_MODE: NcAbiThreadingMode = NcAbiThreadingMode::ThreadLocal;
}

impl ThreadingModel for ThreadSafe {
    const ABI_THREADING_MODE: NcAbiThreadingMode = NcAbiThreadingMode::ThreadSafe;
}

mod sealed {
    /// Prevents downstream crates from inventing unsupported threading modes.
    pub trait Sealed {}

    impl Sealed for super::ThreadLocal {}
    impl Sealed for super::ThreadSafe {}
}

/// Owning smart pointer for a COM interface pointer.
///
/// Cloning calls `AddRef`; dropping calls `Release`. Values returned from
/// [`ComPtr::query`] already own one reference. `ComPtr<T>` is thread-local by
/// default; `ComPtr<T, ThreadSafe>` is `Send` and `Sync`. The marker describes
/// permissions on this handle rather than changing the interface pointer's ABI
/// representation.
pub struct ComPtr<T: ComInterface + ?Sized, M: ThreadingModel = ThreadLocal> {
    ptr: NonNull<T>,
    _interface: PhantomData<T>,
    _mode: PhantomData<M>,
}

// SAFETY: `ThreadSafe` is only constructible through APIs whose safety contract
// requires the pointee to support cross-thread COM calls.
unsafe impl<T: ComInterface + ?Sized> Send for ComPtr<T, ThreadSafe> {}
// SAFETY: Sharing follows the same `ThreadSafe` contract as sending. Method
// calls still go through the pointee's vtable and object-level synchronization.
unsafe impl<T: ComInterface + ?Sized> Sync for ComPtr<T, ThreadSafe> {}

/// Borrowed COM interface pointer valid for a call-scoped lifetime.
///
/// `ComRef` does not own a reference and does not call `Release` on drop. It is
/// the Rust representation for borrowed interface pointer parameters in
/// generated COM methods. If a callee wants to retain the object beyond the
/// current call, it must call [`ComRef::add_ref`], which performs `AddRef` and
/// returns a normal owning [`ComPtr`] carrying the same threading capability.
/// The lifetime parameter still limits how long either threading mode may be
/// borrowed; `ThreadSafe` changes cross-thread permission, not borrow duration.
///
/// A borrowed client cannot extend its owner's lifetime:
///
/// ```compile_fail
/// use neocogi_com_core::{ComPtr, ComRef, IUnknown};
/// fn escape(owner: ComPtr<IUnknown>) -> ComRef<'static, IUnknown> {
///     owner.as_com_ref()
/// }
/// ```
pub struct ComRef<'a, T: ComInterface + ?Sized, M: ThreadingModel = ThreadLocal> {
    ptr: NonNull<T>,
    _interface: PhantomData<&'a T>,
    _mode: PhantomData<M>,
}

impl<'a, T: ComInterface + ?Sized, M: ThreadingModel> Copy for ComRef<'a, T, M> {}

impl<'a, T: ComInterface + ?Sized, M: ThreadingModel> Clone for ComRef<'a, T, M> {
    /// Copies the non-owning pointer and its existing borrow lifetime.
    fn clone(&self) -> Self {
        *self
    }
}

// SAFETY: `ComRef` is a borrowed view. `ThreadSafe` construction requires the
// pointee to be callable from other threads for the borrow's lifetime.
unsafe impl<'a, T: ComInterface + ?Sized> Send for ComRef<'a, T, ThreadSafe> {}
// SAFETY: Shared access has the same object-level requirement as `ComPtr`:
// every exposed method must be safe under concurrent calls.
unsafe impl<'a, T: ComInterface + ?Sized> Sync for ComRef<'a, T, ThreadSafe> {}

// Method views borrow the pointer word rather than forging a ComRef with a
// longer lifetime. The optional unsafe interface trait certifies the view layout.
impl<T: crate::ComClientInterface, M: ThreadingModel> core::ops::Deref for ComPtr<T, M> {
    type Target = T::Client<M>;
    /// Lends the generated method view without AddRef or a second allocation.
    fn deref(&self) -> &Self::Target {
        // SAFETY: ComClientInterface certifies transparent pointer storage and
        // borrowing-only operations. This reference cannot outlive &self.
        unsafe { &*core::ptr::from_ref(&self.ptr).cast::<Self::Target>() }
    }
}

impl<T: crate::ComClientInterface, M: ThreadingModel> core::ops::DerefMut for ComPtr<T, M> {
    /// Lends a mutable method view without claiming exclusive ownership of the COM object.
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: The generated view is transparent over this pointer word.
        // Object-level exclusivity is enforced by the callee's dispatch gate,
        // because another independently owned COM pointer may still exist.
        unsafe { &mut *core::ptr::from_mut(&mut self.ptr).cast::<Self::Target>() }
    }
}

impl<T: crate::ComClientInterface, M: ThreadingModel> core::ops::Deref for ComRef<'_, T, M> {
    type Target = T::Client<M>;
    /// Lends inherited methods within the existing borrow's lifetime.
    fn deref(&self) -> &Self::Target {
        // SAFETY: The certified view shares the pointer word, capability and
        // lifetime of this live ComRef; it cannot manufacture another owner.
        unsafe { &*core::ptr::from_ref(&self.ptr).cast::<Self::Target>() }
    }
}

impl<T: crate::ComClientInterface, M: ThreadingModel> core::ops::DerefMut for ComRef<'_, T, M> {
    /// Lends a mutable method view for the duration of this borrowed handle.
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: The generated view is transparent over this pointer word.
        // The callee's generated dispatch gate arbitrates aliases, including
        // copies of this non-owning handle.
        unsafe { &mut *core::ptr::from_mut(&mut self.ptr).cast::<Self::Target>() }
    }
}

impl<T: ComInterface + ?Sized, M: ThreadingModel> ComPtr<T, M> {
    /// Wraps a raw owned interface pointer.
    ///
    /// This function does not call `AddRef`; the pointer must already represent
    /// an owned reference. For a borrowed raw pointer, use
    /// [`ComPtr::clone_from_raw`] instead so the returned owner receives its own
    /// reference.
    ///
    /// # Safety
    ///
    /// `ptr` must be either null or a valid pointer for interface `T` with one
    /// owned reference. If non-null, it must remain valid until the returned
    /// [`ComPtr`] is dropped or cloned according to COM reference-count rules.
    /// When constructing `ComPtr<T, ThreadSafe>`, the pointed-to object must
    /// also satisfy the high-level `ThreadSafeComObject` contract or an
    /// equivalent foreign-ABI thread-safety guarantee.
    pub unsafe fn from_raw_owned(ptr: *mut T) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self {
            ptr,
            _interface: PhantomData,
            _mode: PhantomData,
        })
    }

    /// Validates a transferred reference before exposing its typed methods.
    /// A rejected non-null result is released exactly once through its common
    /// prefix. No typed vtable slot is read before the identity check succeeds.
    ///
    /// # Safety
    /// `ptr` must be null or own one reference to a live core-ABI-5 interface
    /// header. Its common prefix must be valid, even when it is not interface T.
    /// The caller guarantees the threading capability M of the returned object.
    #[doc(hidden)]
    pub unsafe fn __nc_from_raw_owned_checked(ptr: *mut T) -> Result<Self, NcStatus> {
        // SAFETY: The caller guarantees the live common prefix and ownership.
        if let Err(status) = unsafe { raw::validate_interface(ptr) } {
            // SAFETY: Release only needs the common prefix, not T's methods.
            unsafe { raw::release(ptr) };
            return Err(status);
        }
        // SAFETY: Identity now proves the exact interface; ownership is unchanged.
        unsafe { Self::from_raw_owned(ptr) }.ok_or(NC_E_POINTER)
    }

    /// Clones a borrowed raw interface pointer by calling `AddRef`.
    ///
    /// Returns `None` for null.
    ///
    /// # Safety
    ///
    /// `ptr` must be either null or a valid borrowed interface pointer for `T`.
    /// If non-null, it must point to a live COM object whose `IUnknown` header and
    /// vtable are valid for an `AddRef` call. This function does not take
    /// ownership of the incoming raw pointer; it creates a new owned reference.
    /// When constructing `ComPtr<T, ThreadSafe>`, the pointed-to object must
    /// also satisfy the high-level `ThreadSafeComObject` contract or an
    /// equivalent foreign-ABI thread-safety guarantee.
    pub unsafe fn clone_from_raw(ptr: *mut T) -> Option<Self> {
        let ptr = NonNull::new(ptr)?;
        // SAFETY: The caller guarantees that `ptr` is a live borrowed interface
        // pointer whose `IUnknown` header and vtable are valid for `AddRef`.
        unsafe { raw::add_ref(ptr.as_ptr()) };
        Some(Self {
            ptr,
            _interface: PhantomData,
            _mode: PhantomData,
        })
    }

    /// Returns a borrowed const raw interface pointer without changing its
    /// reference count.
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    /// Returns a borrowed mutable raw interface pointer without changing its
    /// reference count.
    ///
    /// COM vtable methods usually take `*mut Interface` even for logically shared
    /// calls. This pointer is borrowed from `self`; callers must not release it or
    /// store it beyond the lifetime guaranteed by the owning [`ComPtr`].
    pub fn as_mut_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// Returns the borrowed non-null interface pointer.
    pub fn as_non_null(&self) -> NonNull<T> {
        self.ptr
    }

    /// Returns a borrowed COM reference without changing the reference count.
    pub fn as_com_ref(&self) -> ComRef<'_, T, M> {
        ComRef {
            ptr: self.ptr,
            _interface: PhantomData,
            _mode: PhantomData,
        }
    }

    /// Transfers this owned COM reference to the caller as a raw pointer.
    ///
    /// The caller becomes responsible for eventually releasing this reference,
    /// either by passing it to [`ComPtr::from_raw_owned`] or by calling
    /// [`raw::release`].
    pub fn into_raw(self) -> *mut T {
        let this = core::mem::ManuallyDrop::new(self);
        this.ptr.as_ptr()
    }

    /// Queries this object for another interface by identity.
    ///
    /// This is the only supported interface cast operation. The returned
    /// pointer may have a different address from `self`. The object must support
    /// `U::INTERFACE_ID`. Failures preserve the exact [`NcStatus`]; use
    /// [`ComPtr::is`] when only a boolean capability check is needed.
    pub fn query<U: ComInterface>(&self) -> Result<ComPtr<U, M>, NcStatus> {
        let mut out = core::ptr::null_mut();
        let unknown = T::iunknown(self.as_mut_ptr());
        // SAFETY: `self` owns a live interface pointer, so its `IUnknown`
        // prefix and vtable remain valid for the duration of this call. A
        // successful `QueryInterface` writes one owned reference to `out`.
        let status = unsafe { ((*(*unknown).vtable).query_interface)(unknown, &U::INTERFACE_ID, &mut out) };
        if status.is_ok() {
            // SAFETY: Successful `QueryInterface` transfers ownership of one
            // reference for the requested interface. Null is still guarded and
            // mapped to `NC_E_POINTER` for defensive foreign implementations.
            unsafe { ComPtr::<U, M>::__nc_from_raw_owned_checked(out.cast::<U>()) }
        } else {
            Err(status)
        }
    }

    /// Checks a metadata-supplied interface identity without naming its Rust projection.
    ///
    /// This is useful when a designer has a generated COM descriptor but the
    /// concrete projection type is selected by a registered widget provider.
    /// A successful query is released immediately; no typeless handle escapes.
    pub fn supports_interface_id(&self, expected: &crate::NcInterfaceId) -> Result<(), NcStatus> {
        let mut out = core::ptr::null_mut();
        let unknown = T::iunknown(self.as_mut_ptr());
        // SAFETY: this live pointer has a valid IUnknown prefix, and QueryInterface
        // receives a readable identity and writable pointer slot for this call.
        let status = unsafe { ((*(*unknown).vtable).query_interface)(unknown, expected, &mut out) };
        if !status.is_ok() {
            return Err(status);
        }
        if out.is_null() {
            return Err(crate::NC_E_POINTER);
        }
        let projected = out.cast::<crate::IUnknown>();
        // SAFETY: successful QueryInterface returned one owned COM reference.
        // Every interface shares the IUnknown vtable prefix used to release it.
        unsafe { ((*(*projected).vtable).release)(projected) };
        Ok(())
    }

    /// Returns whether this object supports interface `U`.
    ///
    /// Use [`ComPtr::query`] instead when the exact failure status matters.
    pub fn is<U: ComInterface>(&self) -> bool {
        let queried = self.query::<U>();
        queried.is_ok()
    }
}

impl<T: ComInterface + ?Sized> ComPtr<T, ThreadSafe> {
    /// Consumes this owner and removes its cross-thread capability.
    ///
    /// This is a zero-cost capability downgrade: it preserves the interface
    /// pointer and transfers the same single owned COM reference without
    /// calling either `AddRef` or `Release`. Other thread-safe aliases to the
    /// underlying object remain thread-safe.
    pub fn into_thread_local(self) -> ComPtr<T, ThreadLocal> {
        let this = core::mem::ManuallyDrop::new(self);
        ComPtr {
            ptr: this.ptr,
            _interface: PhantomData,
            _mode: PhantomData,
        }
    }

    /// Borrows this owner through a thread-local capability.
    ///
    /// The returned reference has the same lifetime and pointer but cannot be
    /// sent or shared across threads. No COM reference-count operation occurs.
    pub fn as_thread_local_ref(&self) -> ComRef<'_, T, ThreadLocal> {
        ComRef {
            ptr: self.ptr,
            _interface: PhantomData,
            _mode: PhantomData,
        }
    }
}

impl<'a, T: ComInterface + ?Sized, M: ThreadingModel> ComRef<'a, T, M> {
    /// Checks an incoming pointer's complete identity without acquiring ownership.
    ///
    /// # Safety
    /// A non-null pointer must have a live core-ABI-5 common header/vtable for
    /// lifetime 'a and carry threading capability M. Its typed slots may differ
    /// from T; they are never called unless the full identity matches.
    #[doc(hidden)]
    pub unsafe fn __nc_from_raw_borrowed_checked(ptr: *mut T) -> Result<Self, NcStatus> {
        // SAFETY: The call's borrowed common prefix is valid by contract.
        unsafe { raw::validate_interface(ptr) }?;
        // SAFETY: Validation establishes T without changing the borrowed lifetime.
        unsafe { Self::from_raw_borrowed(ptr) }.ok_or(NC_E_POINTER)
    }

    /// Wraps a borrowed raw interface pointer.
    ///
    /// This function does not call `AddRef`; the returned value is a borrowed
    /// view over the incoming pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must be either null or a valid borrowed interface pointer for `T`.
    /// If non-null, it must remain valid for `'a`. When constructing
    /// `ComRef<'_, T, ThreadSafe>`, the pointed-to object must also satisfy the
    /// high-level `ThreadSafeComObject` contract or an equivalent foreign-ABI
    /// thread-safety guarantee.
    pub unsafe fn from_raw_borrowed(ptr: *mut T) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self {
            ptr,
            _interface: PhantomData,
            _mode: PhantomData,
        })
    }

    /// Returns the borrowed const raw interface pointer.
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    /// Returns the borrowed mutable raw interface pointer.
    pub fn as_mut_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// Returns the borrowed non-null interface pointer.
    pub fn as_non_null(&self) -> NonNull<T> {
        self.ptr
    }

    /// Creates an owned COM reference to this same interface by calling
    /// `AddRef`.
    ///
    /// Use this when a method receives a borrowed [`ComRef`] argument and needs
    /// to retain the object after the current call returns.
    pub fn add_ref(&self) -> ComPtr<T, M> {
        // SAFETY: `ComRef` construction required a live borrowed pointer for
        // `'a`; `AddRef` converts that borrow into a new owned reference.
        unsafe { raw::add_ref(self.as_mut_ptr()) };
        ComPtr {
            ptr: self.ptr,
            _interface: PhantomData,
            _mode: PhantomData,
        }
    }
}

impl<'a, T: ComInterface + ?Sized> ComRef<'a, T, ThreadSafe> {
    /// Removes the cross-thread capability from this borrowed reference.
    ///
    /// The pointer and borrow lifetime are unchanged, and no `AddRef` occurs.
    /// Calling [`ComRef::add_ref`] on the returned value consequently creates a
    /// thread-local owner.
    pub fn into_thread_local(self) -> ComRef<'a, T, ThreadLocal> {
        ComRef {
            ptr: self.ptr,
            _interface: PhantomData,
            _mode: PhantomData,
        }
    }
}

impl<'a, T: ComInterface + ?Sized, M: ThreadingModel> From<&'a ComPtr<T, M>> for ComRef<'a, T, M> {
    /// Borrows an owned COM pointer without changing its reference count.
    fn from(value: &'a ComPtr<T, M>) -> Self {
        value.as_com_ref()
    }
}

impl<T: ComInterface + ?Sized, M: ThreadingModel> Clone for ComPtr<T, M> {
    /// Creates another owner by incrementing the COM reference count.
    fn clone(&self) -> Self {
        // SAFETY: `self` owns a live reference, so the pointee is valid for
        // `AddRef` and cloning creates another owned reference.
        unsafe { raw::add_ref(self.as_mut_ptr()) };
        Self {
            ptr: self.ptr,
            _interface: PhantomData,
            _mode: PhantomData,
        }
    }
}

impl<T: ComInterface + ?Sized, M: ThreadingModel> Drop for ComPtr<T, M> {
    /// Releases the single COM reference owned by this smart pointer.
    fn drop(&mut self) {
        // SAFETY: `ComPtr` owns exactly one reference. Dropping releases that
        // reference and does not use the pointer afterward.
        unsafe { raw::release(self.as_mut_ptr()) };
    }
}

impl<T: ComInterface + ?Sized, M: ThreadingModel> core::fmt::Debug for ComPtr<T, M> {
    /// Formats the interface pointer without dereferencing foreign memory.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ComPtr").field(&self.as_ptr()).finish()
    }
}

impl<'a, T: ComInterface + ?Sized, M: ThreadingModel> core::fmt::Debug for ComRef<'a, T, M> {
    /// Formats the borrowed interface pointer without dereferencing it.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ComRef").field(&self.as_ptr()).finish()
    }
}

/// Raw COM pointer helpers.
///
/// Prefer [`ComPtr`] for normal ownership management. These functions are for
/// FFI adapters and low-level glue that must operate directly on interface
/// pointers.
pub mod raw {
    use super::ComInterface;
    use crate::{IUnknown, NcInterfaceId};

    /// Validates a common COM header against one complete expected interface identity.
    ///
    /// This non-generic function is the shared validation body used by every
    /// typed raw-pointer check. Keeping interface-specific constants in the
    /// small generic wrapper below prevents the complete pointer and identity
    /// validation sequence from being generated once per interface type.
    ///
    /// # Safety
    ///
    /// A non-null `unknown` must have a readable common header and vtable from
    /// the current core ABI. Non-null identity metadata reached through that
    /// vtable must remain readable and immutable for the call.
    #[doc(hidden)]
    #[inline(never)]
    pub unsafe fn __nc_validate_iunknown_interface(unknown: *mut IUnknown, expected: &NcInterfaceId) -> Result<(), crate::NcStatus> {
        if unknown.is_null() {
            return Err(crate::NC_E_POINTER);
        }
        // SAFETY: Only the common header is read before the identity is known.
        let vtable = unsafe { (*unknown).vtable };
        if vtable.is_null() {
            return Err(crate::NC_E_POINTER);
        }
        // SAFETY: A non-null vtable is readable under the raw caller's contract.
        let identity = unsafe { (*vtable).interface_id };
        if identity.is_null() {
            return Err(crate::NC_E_POINTER);
        }
        // SAFETY: Non-null metadata has static immutable storage by contract.
        let identity = unsafe { &*identity };
        if identity.iid != expected.iid {
            return Err(crate::NC_E_NOINTERFACE);
        }
        if identity.abi_signature != expected.abi_signature {
            return Err(crate::NC_E_ABI_VERSION);
        }
        Ok(())
    }

    /// Validates the actual projected interface against the receiver's contract.
    /// This performs no AddRef/Release and never follows interface-specific slots.
    ///
    /// # Safety
    /// A non-null pointer must have a readable common header and vtable from the
    /// current core ABI. Non-null identity metadata must be readable and immutable.
    /// Pointer validity and the core ABI generation are established by the loader
    /// or raw FFI caller; identity checking cannot validate arbitrary addresses.
    #[inline(always)]
    pub unsafe fn validate_interface<T: ComInterface + ?Sized>(ptr: *mut T) -> Result<(), crate::NcStatus> {
        if ptr.is_null() {
            return Err(crate::NC_E_POINTER);
        }
        let unknown = T::iunknown(ptr);
        // SAFETY: The caller supplies the common-header validity required by
        // this wrapper; the generated interface identity is immutable static
        // metadata for the complete expected contract.
        unsafe { __nc_validate_iunknown_interface(unknown, &T::INTERFACE_ID) }
    }

    /// Calls `AddRef` on a raw interface pointer.
    ///
    /// Returns `0` for null.
    ///
    /// # Safety
    ///
    /// If non-null, `ptr` must be a valid borrowed interface pointer for `T` with
    /// a live `IUnknown` header and vtable. The object must remain live for the
    /// duration of the call.
    pub unsafe fn add_ref<T: ComInterface + ?Sized>(ptr: *mut T) -> u32 {
        if ptr.is_null() {
            return 0;
        }
        let unknown = T::iunknown(ptr);
        // SAFETY: The caller guarantees that `ptr` is a live borrowed interface
        // pointer whose `IUnknown` header and vtable are valid for `AddRef`.
        unsafe { ((*(*unknown).vtable).add_ref)(unknown) }
    }

    /// Calls `Release` on a raw owned interface pointer.
    ///
    /// Returns `0` for null.
    ///
    /// # Safety
    ///
    /// If non-null, `ptr` must be a valid interface pointer for `T` that owns one
    /// reference. After this call, the pointer may be dangling if the returned
    /// reference count is zero. A zero return means the object was destroyed
    /// immediately; final `Release` is not a fallible cleanup boundary.
    pub unsafe fn release<T: ComInterface + ?Sized>(ptr: *mut T) -> u32 {
        if ptr.is_null() {
            return 0;
        }
        let unknown = T::iunknown(ptr);
        // SAFETY: The caller guarantees that `ptr` owns one live reference.
        // The vtable call consumes that reference and may destroy the object.
        unsafe { ((*(*unknown).vtable).release)(unknown) }
    }
}
