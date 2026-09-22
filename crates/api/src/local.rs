//! Construction-backed typed access to an owned COM implementation.
//!
//! A local handle remembers the concrete type at allocation time. It never
//! downcasts a foreign interface and never lends a caller's stack through COM.
//! Every typed borrow uses the allocation's existing COM call gate, so cloned
//! interfaces and local access enforce the same Rust aliasing rules.

use core::marker::PhantomData;
use crate::{ComInterface, ComObject, ComPtr, IUnknown, NcStatus, __ComExclusiveLease, __ComSharedLease};

/// An owned COM allocation whose concrete Rust implementation is known locally.
///
/// Construct this handle with the generated `Implementation::into_local(value)`
/// method. Query interfaces once for foreign consumers, then borrow the original
/// implementation between COM calls. A failed borrow reports `NC_E_BUSY` without
/// changing the implementation. Guards release their gate during normal return
/// and Rust unwinding, and keep the handle borrowed for their entire lifetime.
///
/// There is deliberately no safe conversion from an arbitrary `ComPtr` to this
/// type. Interface negotiation proves a contract, not a concrete Rust layout.
/// Threading follows both the object's declared COM mode and `T`'s Rust traits.
pub struct LocalCom<T: ComObject> {
    /// Owns the exact allocation created for T and keeps all borrowed state live.
    owner: ComPtr<IUnknown, T::Threading>,
    /// Preserves concrete ownership and Rust Send/Sync constraints after projection.
    implementation: PhantomData<T>,
}

/// Shared local access carrying both the borrow lifetime and COM threading mode.
pub struct LocalRef<'a, T: ComObject> {
    /// Holds the same reader lease used by generated interface thunks.
    lease: __ComSharedLease<'a, T>,
    /// Prevents even a scoped thread from bypassing a thread-local owner's mode.
    owner: PhantomData<&'a LocalCom<T>>,
}

impl<T: ComObject> core::ops::Deref for LocalRef<'_, T> {
    type Target = T;
    /// Exposes state only for the lifetime of this active shared lease.
    fn deref(&self) -> &T {
        &self.lease
    }
}

/// Exclusive local access carrying both the borrow lifetime and COM threading mode.
pub struct LocalMut<'a, T: ComObject> {
    /// Holds the same writer lease used by generated interface thunks.
    lease: __ComExclusiveLease<'a, T>,
    /// Retains the owner's lifetime and conservative thread-affinity constraints.
    owner: PhantomData<&'a LocalCom<T>>,
}

impl<T: ComObject> core::ops::Deref for LocalMut<'_, T> {
    type Target = T;
    /// Observes state while this guard excludes every competing call.
    fn deref(&self) -> &T {
        &self.lease
    }
}

impl<T: ComObject> core::ops::DerefMut for LocalMut<'_, T> {
    /// Mutates state only while this guard owns the exclusive allocation gate.
    fn deref_mut(&mut self) -> &mut T {
        &mut self.lease
    }
}

impl<T: ComObject> LocalCom<T> {
    #[doc(hidden)]
    /// Transfers an allocation of this exact implementation into typed ownership.
    ///
    /// # Safety
    ///
    /// `owner` must be the canonical IUnknown produced by T's generated object
    /// storage. Its back-pointer must satisfy T's `ComAbiObject` contract for
    /// every borrow. An interface implemented by a different Rust type is not
    /// sufficient, even if its IID and ABI signature match.
    pub unsafe fn __from_owned(owner: ComPtr<IUnknown, T::Threading>) -> Self {
        Self { owner, implementation: PhantomData }
    }

    /// Queries a foreign-facing interface while retaining this typed owner.
    ///
    /// Cache the returned projection when repeatedly dispatching through COM;
    /// ordinary local borrows perform no query or reference-count operation.
    pub fn query<I: ComInterface>(&self) -> Result<ComPtr<I, T::Threading>, NcStatus> {
        self.owner.query()
    }

    /// Borrows the concrete implementation while excluding mutable COM calls.
    pub fn try_borrow(&self) -> Result<LocalRef<'_, T>, NcStatus> {
        // SAFETY: Construction established this exact T allocation, and the
        // returned guard borrows self, retaining its strong owner until release.
        // The generated lease uses the same gate as every interface projection.
        let lease = unsafe { T::borrow_abi_ref((*self.owner.as_ptr()).object) }?;
        Ok(LocalRef { lease, owner: PhantomData })
    }

    /// Borrows the concrete implementation exclusively against every projection.
    ///
    /// Release this guard before a runtime calls this object's COM methods.
    /// Reentry while it is held reports BUSY rather than creating another borrow.
    pub fn try_borrow_mut(&self) -> Result<LocalMut<'_, T>, NcStatus> {
        // SAFETY: The constructor proves T's storage layout, self keeps the
        // allocation live, and the generated exclusive lease excludes all
        // competing local and COM borrows for the returned guard's lifetime.
        let lease = unsafe { T::borrow_abi_mut((*self.owner.as_ptr()).object) }?;
        Ok(LocalMut { lease, owner: PhantomData })
    }
}

impl<T: ComObject> Clone for LocalCom<T> {
    /// Retains the same concrete allocation; clones share its one borrow gate.
    fn clone(&self) -> Self {
        Self {
            owner: self.owner.clone(),
            implementation: PhantomData,
        }
    }
}
