//! Owning and borrowed smart pointers for reference-counted COM interfaces.
//!
//! [`ComPtr`] models one owned reference and performs `AddRef`/`Release` on
//! clone/drop, while [`ComRef`] represents a call-scoped borrowed pointer.
//! Threading marker types make the capability carried by each pointer visible
//! to Rust without changing the underlying C object/table representation.
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
//! The C ABI handle contains an object address and interface-table address. It has the same
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

use crate::{ComRaw, ComInterface, NC_E_POINTER, NcAbiThreadingMode, NcStatus};

/// Sealed marker for the compile-time threading capability of a COM handle.
///
/// The marker is part of the Rust type only; C ABI interface pairs have the
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

/// Proven-present representation shared by safe owners and borrowed views.
/// NonNull preserves the two-word niche for Option<ComPtr>, unlike storing the
/// nullable C record directly. Its repr(C) fields match ComRaw for borrowed clients.
#[repr(C)]
pub(crate) struct LiveInterface<T: ComInterface + ?Sized> {
    /// Allocation retained or borrowed by the enclosing safe handle.
    object: core::ptr::NonNull<core::ffi::c_void>,
    /// Matching immutable selected method table.
    vtable: core::ptr::NonNull<T::VTable>,
}
impl<T: ComInterface + ?Sized> Copy for LiveInterface<T> {}
impl<T: ComInterface + ?Sized> Clone for LiveInterface<T> {
    /// Copies addresses without changing the enclosing handle's ownership policy.
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: ComInterface + ?Sized> LiveInterface<T> {
    /// Checks both pointers without reading foreign storage or proving identity.
    pub(crate) fn new(raw: ComRaw<T>) -> Option<Self> {
        Some(Self {
            object: core::ptr::NonNull::new(raw.object)?,
            vtable: core::ptr::NonNull::new(raw.vtable.cast_mut())?,
        })
    }
    /// Borrows a C record without transferring a reference.
    pub(crate) fn as_raw(self) -> ComRaw<T> {
        ComRaw {
            object: self.object.as_ptr(),
            vtable: self.vtable.as_ptr(),
        }
    }
}

/// Owning, typed `{ object, interface_vtable }` COM handle.
///
/// Clone acquires one reference and Drop releases one. The count and mutable-call
/// guard live once in the object, never in each handle. ComRef uses the same pair
/// with a borrowed lifetime; neither representation uses std::any::Any or a Rust
/// trait-object vtable. The interface marker and threading mode remain static.
///
/// # Why two pointers are intentional
///
/// Earlier implementations stored one interface header per inheritance path,
/// then one per distinct contract, then one pointer per header with an owner
/// offset in its vtable. Those designs coupled logical interface identities to
/// physical object slots. They required header initialization, offset generation,
/// inherited-prefix owner recovery, and object storage growing with interfaces.
/// One early flattening also retained full inspection metadata at runtime; compact
/// dispatch must remain independent of those schemas under any representation.
///
/// This representation deliberately stores the allocation and selected table in
/// every handle. Query changes the table while preserving the object address;
/// ordinary methods receive the object directly. Objects contain no interface
/// headers and vtables contain no owner offsets. This removes that layout problem
/// rather than requiring a C++-style primary-base sharing algorithm.
///
/// The cost is explicit: on 64-bit targets a handle is 16 rather than 8 bytes.
/// Removing five old headers saves 40 bytes per object, offset by 8 extra bytes
/// per stored handle; the header/handle storage breaks even at five handles.
/// Alignment, application state, and static tables are separate costs. Larger
/// handles can increase memory traffic; no blanket speed or binary-size win is
/// promised. Review changes against measured object, handle, and deployment costs,
/// rather than treating the second pointer itself as accidental duplication.
///
/// Inherited method-table prefixes remain a separate dispatch-layout decision:
/// two-pointer handles do not by themselves deduplicate their function entries.
/// There is no backward-compatibility requirement to preserve the former ABI.
///
/// Query execution and ownership validation deliberately use one non-generic
/// body. Source and destination identities are data, not reasons to emit another
/// copy of the same checks for every typed handle. Generated object queries use
/// the same separation: small typed adapters supply their counter and tables to
/// one common implementation without exposing application state or field offsets.
pub struct ComPtr<T: ComInterface + ?Sized, M: ThreadingModel = ThreadLocal> {
    /// Present raw pair owning exactly one allocation-wide reference.
    raw: LiveInterface<T>,
    /// Preserves the selected interface type without owning a Rust T value.
    _interface: PhantomData<T>,
    /// Carries the producer's statically established threading capability.
    _mode: PhantomData<M>,
}

// SAFETY: constructors require the producer's concurrent-call/destruction guarantee.
unsafe impl<T: ComInterface + ?Sized> Send for ComPtr<T, ThreadSafe> {}
// SAFETY: the same guarantee permits simultaneous calls through shared handles.
unsafe impl<T: ComInterface + ?Sized> Sync for ComPtr<T, ThreadSafe> {}

/// Borrowed typed object/table pair; copying it neither retains nor releases.
///
/// The lifetime ties synchronous calls to a live owner. Call add_ref to retain
/// it beyond the borrow. The representation tradeoff is documented on ComPtr.
///
/// ```compile_fail
/// use neocogi_com_core::{ComPtr, ComRef, IUnknown};
/// fn escape(owner: ComPtr<IUnknown>) -> ComRef<'static, IUnknown> {
///     owner.as_com_ref()
/// }
/// ```
pub struct ComRef<'a, T: ComInterface + ?Sized, M: ThreadingModel = ThreadLocal> {
    /// Present pair borrowed for the declared lifetime.
    raw: LiveInterface<T>,
    /// Prevents the view from outliving its lending owner.
    _interface: PhantomData<&'a T>,
    /// Preserves thread affinity without changing the C representation.
    _mode: PhantomData<M>,
}
impl<T: ComInterface + ?Sized, M: ThreadingModel> Copy for ComRef<'_, T, M> {}
impl<T: ComInterface + ?Sized, M: ThreadingModel> Clone for ComRef<'_, T, M> {
    /// Copies only an existing borrow, never another owned reference.
    fn clone(&self) -> Self {
        *self
    }
}
// SAFETY: construction establishes cross-thread permission for the borrow's lifetime.
unsafe impl<T: ComInterface + ?Sized> Send for ComRef<'_, T, ThreadSafe> {}
// SAFETY: concurrent access follows that same producer guarantee.
unsafe impl<T: ComInterface + ?Sized> Sync for ComRef<'_, T, ThreadSafe> {}

impl<T: crate::ComClientInterface, M: ThreadingModel> core::ops::Deref for ComPtr<T, M> {
    type Target = T::Client<M>;
    /// Lends a transparent method view over this pair without allocating or retaining.
    fn deref(&self) -> &Self::Target {
        // SAFETY: ComClientInterface certifies a transparent view over ComRaw<T>.
        // LiveInterface has the same repr(C) fields; its non-null pointers are
        // valid raw pointers. Client methods cannot mutate or replace this pair.
        unsafe { &*core::ptr::from_ref(&self.raw).cast() }
    }
}
impl<T: crate::ComClientInterface, M: ThreadingModel> core::ops::DerefMut for ComPtr<T, M> {
    /// Lends mutable method lookup; the object-wide gate arbitrates aliased callers.
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: the generated transparent view cannot escape this borrow.
        unsafe { &mut *core::ptr::from_mut(&mut self.raw).cast() }
    }
}
impl<T: crate::ComClientInterface, M: ThreadingModel> core::ops::Deref for ComRef<'_, T, M> {
    type Target = T::Client<M>;
    /// Lends method access within the existing borrowed lifetime.
    fn deref(&self) -> &Self::Target {
        // SAFETY: the transparent generated client neither owns nor extends the borrow.
        unsafe { &*core::ptr::from_ref(&self.raw).cast() }
    }
}
impl<T: crate::ComClientInterface, M: ThreadingModel> core::ops::DerefMut for ComRef<'_, T, M> {
    /// Lends mutable lookup without asserting unique ownership of the COM object.
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: the caller's exclusive handle borrow bounds this transparent view.
        unsafe { &mut *core::ptr::from_mut(&mut self.raw).cast() }
    }
}
impl<T: ComInterface + ?Sized, M: ThreadingModel> ComPtr<T, M> {
    /// Adopts one raw owned reference without incrementing its count.
    ///
    /// # Safety
    /// A present pair must identify T's exact live implementation and table and
    /// transfer one reference. The implementation must satisfy threading mode M.
    /// Both-null absence is allowed. A half-null pair violates the raw contract.
    pub unsafe fn from_raw_owned(raw: ComRaw<T>) -> Option<Self> {
        Some(Self {
            raw: LiveInterface::new(raw)?,
            _interface: PhantomData,
            _mode: PhantomData,
        })
    }
    /// Validates a returned contract before adopting its transferred reference.
    ///
    /// # Safety
    /// Present pointers must be readable/live and support their common lifecycle
    /// prefix under this core ABI. Even a wrong typed identity must be releasable.
    /// The raw caller establishes threading mode M. Null shapes need no dereference.
    #[doc(hidden)]
    pub unsafe fn __nc_from_raw_owned_checked(raw: ComRaw<T>) -> Result<Self, NcStatus> {
        // SAFETY: the caller supplies readable common lifecycle storage.
        if let Err(status) = unsafe { raw::validate_interface(raw) } {
            // SAFETY: a present mismatched interface still owns one common reference.
            unsafe { raw::release(raw) };
            return Err(status);
        }
        // SAFETY: validation proved T and the input transfers ownership unchanged.
        unsafe { Self::from_raw_owned(raw) }.ok_or(NC_E_POINTER)
    }
    /// Retains a raw borrowed reference and returns a new owning handle.
    ///
    /// # Safety
    /// A present pair must implement T, remain live during AddRef, and satisfy M.
    pub unsafe fn clone_from_raw(raw: ComRaw<T>) -> Option<Self> {
        if !raw.is_present() {
            return None;
        }
        // SAFETY: the caller supplies T's live borrowed pair.
        if unsafe { raw::add_ref(raw) } == 0 { return None; }
        // SAFETY: the preceding AddRef transferred one new owner.
        unsafe { Self::from_raw_owned(raw) }
    }
    /// Borrows the raw ABI pair without transferring ownership or extending lifetime.
    pub fn as_raw(&self) -> ComRaw<T> {
        self.raw.as_raw()
    }
    /// Creates a weak lifetime reference without retaining the object's value.
    /// Providers without the optional lifetime capability return NC_E_NOINTERFACE.
    pub fn downgrade(&self) -> Result<crate::WeakComPtr<T, M>, NcStatus> {
        // SAFETY: self retains/borrows the live source and preserves its capability.
        let raw = unsafe { crate::weak::downgrade(self.as_raw().cast()) }?;
        // SAFETY: successful downgrade transfers a token for this exact source.
        unsafe { crate::WeakComPtr::from_raw_owned(raw) }.ok_or(NC_E_POINTER)
    }

    /// Borrows a typed view without changing the count.
    pub fn as_com_ref(&self) -> ComRef<'_, T, M> {
        ComRef {
            raw: self.raw,
            _interface: PhantomData,
            _mode: PhantomData,
        }
    }
    /// Transfers this owner's pair; the recipient must eventually release it once.
    pub fn into_raw(self) -> ComRaw<T> {
        core::mem::ManuallyDrop::new(self).raw.as_raw()
    }
    /// Selects another contract on the same allocation and acquires one reference.
    /// The object address stays unchanged; the selected interface table may differ.
    pub fn query<U: ComInterface>(&self) -> Result<ComPtr<U, M>, NcStatus> {
        // SAFETY: this owner keeps the matching allocation/table pair alive.
        let out = unsafe { raw::__nc_query_interface(self.as_raw().erase(), &U::INTERFACE_ID) }?;
        // SAFETY: the shared query body validates exact identity and transfers one owner.
        unsafe { ComPtr::<U, M>::from_raw_owned(out.cast()) }.ok_or(NC_E_POINTER)
    }

    /// Checks a discovered contract with a call-scoped borrowed digest.
    pub fn supports_interface_bytes(&self, iid: crate::NcGuid, signature: &[u8; 32]) -> Result<(), NcStatus> {
        self.supports_interface_id(&crate::NcInterfaceId::new(iid, crate::NcAbiSignature::from_borrowed(signature)))
    }
    /// Queries and releases a discovered exact contract without inventing a Rust type.
    pub fn supports_interface_id(&self, expected: &crate::NcInterfaceId) -> Result<(), NcStatus> {
        // SAFETY: this owner retains the source through the common checked query.
        let out = unsafe { raw::__nc_query_interface(self.as_raw().erase(), expected) }?;
        // SAFETY: the validated query transfers exactly one temporary reference.
        unsafe { raw::release(out) };
        Ok(())
    }

    /// Reports support for U; query preserves the detailed failure status when needed.
    pub fn is<U: ComInterface>(&self) -> bool {
        self.query::<U>().is_ok()
    }
}
impl<T: ComInterface + ?Sized> ComPtr<T, ThreadSafe> {
    /// Transfers ownership while weakening cross-thread permission, with no count change.
    pub fn into_thread_local(self) -> ComPtr<T, ThreadLocal> {
        let this = core::mem::ManuallyDrop::new(self);
        ComPtr {
            raw: this.raw,
            _interface: PhantomData,
            _mode: PhantomData,
        }
    }
    /// Lends the same pair with thread-local permission only.
    pub fn as_thread_local_ref(&self) -> ComRef<'_, T, ThreadLocal> {
        ComRef {
            raw: self.raw,
            _interface: PhantomData,
            _mode: PhantomData,
        }
    }
}
impl<'a, T: ComInterface + ?Sized, M: ThreadingModel> ComRef<'a, T, M> {
    /// Validates an incoming borrowed contract before exposing typed methods.
    ///
    /// # Safety
    /// Present pairs must have a live common prefix, stay live for 'a, and satisfy M.
    #[doc(hidden)]
    pub unsafe fn __nc_from_raw_borrowed_checked(raw: ComRaw<T>) -> Result<Self, NcStatus> {
        // SAFETY: the caller supplies readable common storage for the borrow.
        unsafe { raw::validate_interface(raw) }?;
        // SAFETY: the validated pair has the exact contract and unchanged lifetime.
        unsafe { Self::from_raw_borrowed(raw) }.ok_or(NC_E_POINTER)
    }
    /// Wraps a borrowed pair without retaining it.
    ///
    /// # Safety
    /// Present pairs must implement T, remain live for 'a, and satisfy M.
    pub unsafe fn from_raw_borrowed(raw: ComRaw<T>) -> Option<Self> {
        Some(Self {
            raw: LiveInterface::new(raw)?,
            _interface: PhantomData,
            _mode: PhantomData,
        })
    }
    /// Borrows the raw pair without reference-count changes.
    pub fn as_raw(&self) -> ComRaw<T> {
        self.raw.as_raw()
    }
    /// Compares two live capabilities without querying or borrowing object state.
    /// The object/table ABI guarantees one canonical object address across all
    /// projections. Both borrows pin their values, preventing address reuse here.
    pub fn same_object<U: ComInterface + ?Sized, N: ThreadingModel>(&self, other: ComRef<'_, U, N>) -> bool {
        self.raw.object == other.raw.object
    }

    /// Creates a weak lifetime reference without retaining the object's value.
    /// Providers without the optional lifetime capability return NC_E_NOINTERFACE.
    pub fn downgrade(&self) -> Result<crate::WeakComPtr<T, M>, NcStatus> {
        // SAFETY: self retains/borrows the live source and preserves its capability.
        let raw = unsafe { crate::weak::downgrade(self.as_raw().cast()) }?;
        // SAFETY: successful downgrade transfers a token for this exact source.
        unsafe { crate::WeakComPtr::from_raw_owned(raw) }.ok_or(NC_E_POINTER)
    }

    /// Acquires an owner that can outlive this borrowed view.
    pub fn add_ref(&self) -> ComPtr<T, M> {
        self.try_add_ref().expect("interface does not permit shared ownership")
    }

    /// Acquires shared ownership only when the provider permits it. Exclusive
    /// objects return NC_E_INVALID_ARG instead of manufacturing another owner.
    pub fn try_add_ref(&self) -> Result<ComPtr<T, M>, NcStatus> {
        // SAFETY: the borrow keeps the matching pair live through AddRef.
        if unsafe { raw::add_ref(self.as_raw()) } == 0 { return Err(crate::NC_E_INVALID_ARG); }
        Ok(ComPtr { raw: self.raw, _interface: PhantomData, _mode: PhantomData })
    }
}
impl<'a, T: ComInterface + ?Sized> ComRef<'a, T, ThreadSafe> {
    /// Removes cross-thread permission without changing the borrow or count.
    pub fn into_thread_local(self) -> ComRef<'a, T, ThreadLocal> {
        ComRef {
            raw: self.raw,
            _interface: PhantomData,
            _mode: PhantomData,
        }
    }
}
impl<'a, T: ComInterface + ?Sized, M: ThreadingModel> From<&'a ComPtr<T, M>> for ComRef<'a, T, M> {
    /// Borrows the owner for exactly the supplied lifetime.
    fn from(value: &'a ComPtr<T, M>) -> Self {
        value.as_com_ref()
    }
}
impl<T: ComInterface + ?Sized, M: ThreadingModel> Clone for ComPtr<T, M> {
    /// Acquires one additional allocation-wide reference.
    fn clone(&self) -> Self {
        // SAFETY: this owner keeps its common lifecycle prefix live.
        unsafe { raw::add_ref(self.as_raw()) };
        Self {
            raw: self.raw,
            _interface: PhantomData,
            _mode: PhantomData,
        }
    }
}
impl<T: ComInterface + ?Sized, M: ThreadingModel> Drop for ComPtr<T, M> {
    /// Consumes exactly this reference; final release destroys the shared allocation.
    fn drop(&mut self) {
        // SAFETY: construction/clone established exactly one owned reference.
        unsafe { raw::release(self.as_raw()) };
    }
}
impl<T: ComInterface + ?Sized, M: ThreadingModel> core::fmt::Debug for ComPtr<T, M> {
    /// Formats both addresses without dereferencing foreign storage.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ComPtr").field(&self.as_raw()).finish()
    }
}
impl<T: ComInterface + ?Sized, M: ThreadingModel> core::fmt::Debug for ComRef<'_, T, M> {
    /// Formats the borrowed pair without extending its lifetime.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ComRef").field(&self.as_raw()).finish()
    }
}

/// Unsafe ABI operations on explicit object/table pairs.
pub mod raw {
    use crate::{ComInterface, ComRaw, IUnknown, NcInterfaceId};
    /// Performs query, exact contract validation, and ownership cleanup once for
    /// every source/destination interface combination. Types are checked at the
    /// boundary; the body requires only the common lifecycle prefix.
    ///
    /// Keeping this body non-generic prevents typed constants and ownership
    /// cleanup from duplicating the full query sequence in every caller. Success
    /// transfers one exact requested interface on the source allocation. A failed
    /// foreign call transfers nothing; a rejected successful result is released.
    ///
    /// # Safety
    /// Source must be a present, matching live object/table pair retained through
    /// the call. Expected identity metadata must be readable. A successful foreign
    /// query transfers one reference with a readable, releasable lifecycle prefix;
    /// null/half-null result shapes are rejected without reading their pointers.
    #[doc(hidden)]
    #[inline(never)]
    pub unsafe fn __nc_query_interface(source: ComRaw<IUnknown>, expected: &NcInterfaceId) -> Result<ComRaw<IUnknown>, crate::NcStatus> {
        let mut out = ComRaw::null();
        // SAFETY: the source owner lives through this synchronous operation.
        let status = unsafe { ((*source.vtable).query_interface)(source.object, expected, &mut out) };
        if status.is_err() {
            return Err(status);
        }
        // SAFETY: successful queries supply one common, releasable owned pair.
        let validation = unsafe { __nc_validate_iunknown_interface(out, expected) }.and_then(|()| match out.object == source.object {
            true => Ok(()),
            false => Err(crate::NC_E_INVALID_ARG),
        });
        match validation {
            Ok(()) => Ok(out),
            Err(status) => {
                // SAFETY: consume the successful query's transfer on rejection.
                unsafe {
                    release(out);
                }
                Err(status)
            }
        }
    }
    /// Checks presence and exact interface identity without invoking typed methods.
    ///
    /// # Safety
    /// Present object/table pairs must have readable common prefixes under the
    /// current core ABI. Validation cannot establish arbitrary pointer validity.
    #[doc(hidden)]
    #[inline(never)]
    pub unsafe fn __nc_validate_iunknown_interface(raw: ComRaw<IUnknown>, expected: &NcInterfaceId) -> Result<(), crate::NcStatus> {
        if !raw.is_present() {
            return Err(crate::NC_E_POINTER);
        }
        // SAFETY: the caller grants access to the common prefix and static identity.
        let identity = unsafe { (*raw.vtable).interface_id };
        if identity.is_null() {
            return Err(crate::NC_E_POINTER);
        }
        // SAFETY: non-null identity metadata remains readable by the raw contract.
        let identity = unsafe { &*identity };
        if identity.iid != expected.iid {
            return Err(crate::NC_E_NOINTERFACE);
        }
        if identity.abi_signature != expected.abi_signature {
            return Err(crate::NC_E_ABI_VERSION);
        }
        Ok(())
    }
    /// Validates T's exact contract through the common table prefix.
    ///
    /// # Safety
    /// Present pairs must have a readable common prefix for this core ABI.
    #[inline(always)]
    pub unsafe fn validate_interface<T: ComInterface + ?Sized>(raw: ComRaw<T>) -> Result<(), crate::NcStatus> {
        // SAFETY: the interface trait certifies the common table prefix.
        unsafe { __nc_validate_iunknown_interface(raw.erase(), &T::INTERFACE_ID) }
    }
    /// Retains a present borrowed pair; absent/malformed null shapes return zero.
    ///
    /// # Safety
    /// Present pairs must identify a live allocation and matching lifecycle table.
    pub unsafe fn add_ref<T: ComInterface + ?Sized>(raw: ComRaw<T>) -> u32 {
        if !raw.is_present() {
            return 0;
        }
        let raw = raw.erase();
        // SAFETY: caller guarantees the matching live object and table.
        unsafe { ((*raw.vtable).add_ref)(raw.object) }
    }
    /// Consumes one owned reference; never accesses the object after final release.
    ///
    /// # Safety
    /// Present pairs must transfer one reference to their matching lifecycle table.
    pub unsafe fn release<T: ComInterface + ?Sized>(raw: ComRaw<T>) -> u32 {
        if !raw.is_present() {
            return 0;
        }
        let raw = raw.erase();
        // SAFETY: caller transfers one reference and the table owns destruction.
        unsafe { ((*raw.vtable).release)(raw.object) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::{
        ffi::c_void,
        sync::atomic::{AtomicU32, Ordering},
    };
    use crate::{IUnknown, IUnknownVTable, NcInterfaceId, NC_E_INVALID_ARG, NC_OK};

    /// Stack-owned foreign fixture; counts track logical owners without freeing storage.
    /// Tests keep it alive until every logical COM owner has been released.
    struct Probe {
        /// Live logical references, including transient successful query results.
        references: AtomicU32,
        /// Optional substitute allocation simulating a broken QueryInterface.
        destination: *mut c_void,
    }
    /// Publishes a matching root table, optionally paired with the wrong object.
    unsafe extern "C" fn query(object: *mut c_void, _: *const NcInterfaceId, out: *mut ComRaw<IUnknown>) -> NcStatus {
        // SAFETY: tests supply a live Probe and writable output record.
        let source = unsafe { &*object.cast::<Probe>() };
        let destination = match source.destination.is_null() {
            true => object,
            false => source.destination,
        };
        // SAFETY: destination is another live Probe; AddRef transfers one logical owner.
        unsafe {
            retain(destination);
            *out = ComRaw { object: destination, vtable: &TABLE };
        }
        NC_OK
    }
    /// Counts the exact extra ownership created by a successful query.
    unsafe extern "C" fn retain(object: *mut c_void) -> u32 {
        // SAFETY: tests retain stack storage throughout all logical ownership.
        unsafe { (*object.cast::<Probe>()).references.fetch_add(1, Ordering::Relaxed) + 1 }
    }
    /// Balances logical ownership; the enclosing test owns the actual stack storage.
    unsafe extern "C" fn release(object: *mut c_void) -> u32 {
        // SAFETY: each caller transfers one existing logical reference.
        unsafe { (*object.cast::<Probe>()).references.fetch_sub(1, Ordering::Relaxed) - 1 }
    }
    /// Valid root contract makes object-identity rejection independent of table checks.
    static TABLE: IUnknownVTable = IUnknownVTable {
        interface_id: &IUnknown::INTERFACE_ID,
        query_interface: query,
        add_ref: retain,
        release,
        downgrade: None,
    };

    /// Typed and discovered queries reject substitution and release its extra owner.
    #[test]
    fn query_rejects_another_object_without_leaking_its_reference() {
        let mut destination = Probe {
            references: AtomicU32::new(0),
            destination: core::ptr::null_mut(),
        };
        let mut source = Probe {
            references: AtomicU32::new(1),
            destination: core::ptr::from_mut(&mut destination).cast(),
        };
        let raw = ComRaw {
            object: core::ptr::from_mut(&mut source).cast(),
            vtable: &TABLE,
        };
        // SAFETY: source lives through Drop and its table supplies balanced lifecycle calls.
        let owner = unsafe { ComPtr::<IUnknown>::from_raw_owned(raw) }.unwrap();
        assert_eq!(owner.query::<IUnknown>().unwrap_err(), NC_E_INVALID_ARG);
        assert_eq!(destination.references.load(Ordering::Relaxed), 0);
        assert_eq!(owner.supports_interface_id(&IUnknown::INTERFACE_ID), Err(NC_E_INVALID_ARG));
        assert_eq!(destination.references.load(Ordering::Relaxed), 0);
        drop(owner);
        assert_eq!(source.references.load(Ordering::Relaxed), 0);
    }

    /// A wrong returned contract is rejected through its common lifecycle table.
    #[test]
    fn wrong_identity_releases_owned_pair_and_null_shapes_are_rejected() {
        let mut source = Probe {
            references: AtomicU32::new(1),
            destination: core::ptr::null_mut(),
        };
        let wrong_table = IUnknownVTable {
            interface_id: &NcInterfaceId::NONE,
            ..TABLE
        };
        let raw = ComRaw {
            object: core::ptr::from_mut(&mut source).cast(),
            vtable: &wrong_table,
        };
        // SAFETY: the wrong identity still supplies a live common lifecycle prefix.
        assert!(unsafe { ComPtr::<IUnknown>::__nc_from_raw_owned_checked(raw) }.is_err());
        assert_eq!(source.references.load(Ordering::Relaxed), 0);
        for malformed in [
            ComRaw::null(),
            ComRaw {
                object: raw.object,
                vtable: core::ptr::null(),
            },
            ComRaw {
                object: core::ptr::null_mut(),
                vtable: &TABLE,
            },
        ] {
            // SAFETY: shape validation rejects these records without reading their pointers.
            assert_eq!(unsafe { ComPtr::<IUnknown>::__nc_from_raw_owned_checked(malformed) }.unwrap_err(), NC_E_POINTER);
        }
        assert_eq!(source.references.load(Ordering::Relaxed), 0);
    }
}
