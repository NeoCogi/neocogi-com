//! Construction-backed typed access to an owned COM implementation.

use core::marker::PhantomData;
use crate::{ComBox, ComExclusiveObject, ComInterface, ComMut, ComObject, ComPtr, ComRef, IUnknown, NcStatus};

/// An owned COM allocation whose concrete Rust implementation is known locally.
///
/// Construct this handle with the generated `Implementation::into_local(value)`
/// method. Query interfaces for foreign consumers and lend shared access to the
/// concrete value from this strong owner. Mutation uses T's interior state.
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

/// A non-owning handle to a concrete, construction-proven shared COM implementation.
///
/// Created only by [`LocalCom::downgrade`], this retains lifetime bookkeeping,
/// not `T`. Promotion acquires a strong owner before exposing local access and
/// returns `None` after final strong release. The promoted owner lends shared
/// access to the concrete value.
/// Both the original threading capability and `T`'s Rust traits are preserved.
/// An arbitrary weak interface cannot be converted into this concrete handle.
pub struct WeakLocalCom<T: ComObject> {
    /// Canonical weak identity for the exact allocation originally constructed as T.
    observer: crate::WeakComPtr<IUnknown, T::Threading>,
    /// Preserves the concrete construction proof and its Send/Sync restrictions.
    implementation: PhantomData<T>,
}

impl<T: ComObject> LocalCom<T> {
    #[doc(hidden)]
    /// Transfers an allocation of this exact implementation into typed ownership.
    ///
    /// # Safety
    ///
    /// `owner` must be the canonical IUnknown produced by T's generated object
    /// storage. Its object address must satisfy T's `ComObject` contract for
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

    /// Retains a typed weak identity without extending the implementation's lifetime.
    ///
    /// This uses the same weak protocol as interface pointers and performs no
    /// value borrow, so it is also valid while a local or COM call is active.
    pub fn downgrade(&self) -> Result<WeakLocalCom<T>, NcStatus> {
        Ok(WeakLocalCom {
            observer: self.owner.downgrade()?,
            implementation: PhantomData,
        })
    }

    /// Lends shared access to the concrete value for the owner's borrow.
    pub fn get(&self) -> &T {
        // SAFETY: construction proved exact T storage. This strong owner
        // retains it, and shared backing exposes only shared access.
        unsafe { T::borrow_abi_ref(self.owner.as_raw().object) }.expect("live local shared owner")
    }
}

impl<T: ComObject> Clone for LocalCom<T> {
    /// Retains the same concrete allocation.
    fn clone(&self) -> Self {
        Self {
            owner: self.owner.clone(),
            implementation: PhantomData,
        }
    }
}

impl<T: ComObject> WeakLocalCom<T> {
    /// Acquires one local strong owner, or returns `None` after final release.
    ///
    /// Lifetime-provider errors remain errors, as with `WeakComPtr::upgrade`.
    pub fn upgrade(&self) -> Result<Option<LocalCom<T>>, NcStatus> {
        Ok(self.observer.upgrade()?.map(|owner| LocalCom {
            // Downgrade preserved the construction proof; promotion can only
            // return that exact allocation and has already acquired its owner.
            owner,
            implementation: PhantomData,
        }))
    }
}

impl<T: ComObject> Clone for WeakLocalCom<T> {
    /// Retains the same control block without promoting or inspecting the value.
    fn clone(&self) -> Self {
        Self {
            observer: self.observer.clone(),
            implementation: PhantomData,
        }
    }
}

impl<T: ComObject> core::fmt::Debug for WeakLocalCom<T> {
    /// Reports weak identity without requiring T: Debug or promoting an expired value.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("WeakLocalCom").field(&self.observer).finish()
    }
}

/// Typed access to the uniquely owned concrete value in a COM allocation.
pub struct LocalBox<T: ComExclusiveObject> {
    /// Sole owner of the exact T allocation.
    owner: ComBox<IUnknown, T::Threading>,
    /// Preserves the construction proof and concrete Rust thread constraints.
    implementation: PhantomData<T>,
}

impl<T: ComExclusiveObject> core::fmt::Debug for LocalBox<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("LocalBox").field(&self.owner).finish()
    }
}

impl<T: ComExclusiveObject> LocalBox<T> {
    /// Adopts the exclusive allocation generated for this exact implementation.
    ///
    /// # Safety
    /// The owner must come from T's generated storage with its matching
    /// threading capability.
    #[doc(hidden)]
    pub unsafe fn __from_owned(owner: ComBox<IUnknown, T::Threading>) -> Self {
        Self { owner, implementation: PhantomData }
    }

    /// Lends shared access to an exact interface of this owner.
    pub fn query_ref<I: ComInterface>(&self) -> Result<ComRef<'_, I, T::Threading>, NcStatus> {
        self.owner.query_ref()
    }

    /// Lends exclusive access to an exact interface of this owner.
    pub fn query_mut<I: ComInterface>(&mut self) -> Result<ComMut<'_, I, T::Threading>, NcStatus> {
        self.owner.query_mut()
    }

    /// Transfers the owner or returns the unchanged local owner on rejection.
    pub fn into_interface<I: ComInterface>(self) -> Result<ComBox<I, T::Threading>, (NcStatus, Self)> {
        self.owner
            .query()
            .map_err(|(status, owner)| (status, Self { owner, implementation: PhantomData }))
    }

    /// Lends the concrete value for the shared owner borrow.
    pub fn get(&self) -> &T {
        // SAFETY: construction proved exact boxed T storage.
        unsafe { T::borrow_abi_ref(self.owner.as_raw().object) }.expect("live local boxed owner")
    }

    /// Lends the concrete value for the exclusive owner borrow.
    pub fn get_mut(&mut self) -> &mut T {
        // SAFETY: this exclusive borrow of the sole owner authorizes access
        // to the exact initialized value for the returned lifetime.
        unsafe { T::borrow_abi_mut(self.owner.as_raw().object) }.expect("live local boxed owner")
    }
}
