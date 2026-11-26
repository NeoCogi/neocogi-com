//! Generated method lookup preserves the lender's pair, lifetime and token kind.

#[allow(unused_imports, unused_macros)]
mod test_contracts {
    include!(concat!(env!("OUT_DIR"), "/test_contracts.rs"));
}
extern crate self as fixture_weak_com;
test_contracts::weak_com!();

use neocogi_com_sdk::{com_object, ComPtr, NcStatus, ThreadSafe, NC_E_CLOSED};
use std::cell::Cell;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

/// Ordinary provider state; only destruction observation is shared externally.
struct State {
    /// Explicit logical closure is independent of object ownership.
    closed: Cell<bool>,
    /// Survives final release so each ownership mode's destruction is observable.
    drops: Arc<AtomicUsize>,
}
impl Drop for State {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

/// Shared ownership lends the same unsized wrapper as an ordinary call borrow.
#[com_object(IDerived)]
struct SharedValue {
    state: State,
}

/// A unique box exposes the same shared receiver methods.
#[com_object(IDerived, ownership = exclusive)]
struct ExclusiveValue {
    state: State,
}

/// Implements the same semantic operations for two intentionally different owners.
macro_rules! value_methods {
    ($provider:ty) => {
        impl IValue for $provider {
            fn value(&self) -> Result<u32, NcStatus> {
                if self.state.closed.get() { Err(NC_E_CLOSED) } else { Ok(42) }
            }
            fn close(&self) -> Result<(), NcStatus> {
                self.state.closed.set(true);
                Ok(())
            }
        }
        impl IDerived for $provider {}
    };
}
value_methods!(SharedValue);
value_methods!(ExclusiveValue);

/// These counters observe only the instrumented table, independently of other tests.
static RETAINS: AtomicUsize = AtomicUsize::new(0);
/// Counts strong releases through the instrumented projection.
static RELEASES: AtomicUsize = AtomicUsize::new(0);
/// Counts weak-token acquisition through the instrumented projection.
static DOWNGRADES: AtomicUsize = AtomicUsize::new(0);

/// Forwards the original shared lifetime operation with an observable counter.
unsafe extern "C" fn retain(this: *mut core::ffi::c_void) -> u32 {
    RETAINS.fetch_add(1, Ordering::SeqCst);
    // SAFETY: the static table is used only with SharedValue allocations.
    unsafe { (neocogi_com_sdk::lifetime::SHARED_UNKNOWN_VTABLE.add_ref)(this) }
}
/// Forwards consumption of the instrumented projection's actual owner token.
unsafe extern "C" fn release(this: *mut core::ffi::c_void) -> u32 {
    RELEASES.fetch_add(1, Ordering::SeqCst);
    // SAFETY: each caller consumes exactly one retained SharedValue token.
    unsafe { (neocogi_com_sdk::lifetime::SHARED_UNKNOWN_VTABLE.release)(this) }
}
/// Forwards weak acquisition without changing the provider's canonical control.
unsafe extern "C" fn downgrade(this: *mut core::ffi::c_void, out: *mut neocogi_com_sdk::NcSharedWeakRaw) -> NcStatus {
    DOWNGRADES.fetch_add(1, Ordering::SeqCst);
    // SAFETY: the provider is live and out is the caller's writable token slot.
    unsafe { (neocogi_com_sdk::lifetime::SHARED_UNKNOWN_VTABLE.downgrade.unwrap())(this, out) }
}

/// A real generated table with only lifecycle observation interposed.
static COUNTED_TABLE: IDerivedVTable = {
    let table = IDerivedVtblFor::<SharedValue>::VTABLE;
    IDerivedVTable {
        base: IValueVTable {
            unknown: neocogi_com_sdk::IUnknownVTable {
                add_ref: retain,
                release,
                downgrade: Some(downgrade),
                ..table.base.unknown
            },
            ..table.base
        },
    }
};

/// Independent static metadata with the generated interface's exact digest contents.
static EQUIVALENT_DERIVED_DIGEST: [u8; 32] = {
    // SAFETY: generated interface metadata owns a permanent complete digest.
    match unsafe { IDerivedAbi::ABI_SIGNATURE.as_digest() } {
        Some(bytes) => *bytes,
        None => panic!("generated interfaces have a complete digest"),
    }
};
/// A rejection-only revision differs in the final digest byte, not its IID.
static OTHER_DERIVED_DIGEST: [u8; 32] = {
    let mut bytes = EQUIVALENT_DERIVED_DIGEST;
    bytes[31] ^= 1;
    bytes
};
/// The same physical contract with its digest stored independently.
enum EquivalentDerived {}
// SAFETY: identity contents, physical table and threading exactly match IDerived.
unsafe impl neocogi_com_sdk::ComInterface for EquivalentDerived {
    const INTERFACE_ID: neocogi_com_sdk::NcInterfaceId =
        neocogi_com_sdk::NcInterfaceId::new(IDerivedAbi::IID, neocogi_com_sdk::NcAbiDigest::from_static(&EQUIVALENT_DERIVED_DIGEST));
    type VTable = IDerivedVTable;
    type Threading = <IDerivedAbi as neocogi_com_sdk::ComInterface>::Threading;
}
/// Hypothetical revision requested only for rejection; no provider publishes it.
enum OtherDerivedRevision {}
// SAFETY: the hypothetical revision uses this physical table and threading;
// rejection tests never publish a provider or call through this unsupported view.
unsafe impl neocogi_com_sdk::ComInterface for OtherDerivedRevision {
    const INTERFACE_ID: neocogi_com_sdk::NcInterfaceId =
        neocogi_com_sdk::NcInterfaceId::new(IDerivedAbi::IID, neocogi_com_sdk::NcAbiDigest::from_static(&OTHER_DERIVED_DIGEST));
    type VTable = IDerivedVTable;
    type Threading = <IDerivedAbi as neocogi_com_sdk::ComInterface>::Threading;
}

/// Content-equal static identity bypasses discovery but still adds one shared owner.
#[test]
fn shared_identity_queries_retain_once_and_reject_the_last_digest_byte() {
    let drops = Arc::new(AtomicUsize::new(0));
    let root = SharedValue::from_value(SharedValue {
        state: State {
            closed: Cell::new(false),
            drops: drops.clone(),
        },
    });
    // SAFETY: COUNTED_TABLE has the exact generated IDerived identity and method
    // thunks for this concrete allocation. Move, rather than duplicate, the root owner.
    let owner = unsafe {
        ComPtr::<IDerivedAbi>::from_raw_owned(neocogi_com_sdk::ComRaw {
            object: root.into_raw().object,
            vtable: &COUNTED_TABLE,
        })
    }
    .unwrap();
    let original = owner.as_raw();
    let retains = RETAINS.load(Ordering::SeqCst);
    let releases = RELEASES.load(Ordering::SeqCst);
    // SAFETY: the generated digest is permanently readable metadata.
    assert_ne!(
        unsafe { IDerivedAbi::ABI_SIGNATURE.as_digest() }.unwrap().as_ptr(),
        EQUIVALENT_DERIVED_DIGEST.as_ptr()
    );
    assert_eq!(owner.query::<OtherDerivedRevision>().unwrap_err(), neocogi_com_sdk::NC_E_ABI_VERSION);
    assert_eq!(owner.as_raw(), original);
    assert_eq!(RETAINS.load(Ordering::SeqCst), retains);
    assert_eq!(RELEASES.load(Ordering::SeqCst), releases);
    let equivalent = owner.query::<EquivalentDerived>().unwrap();
    assert_eq!(equivalent.as_raw().cast::<IDerivedAbi>(), original);
    assert_eq!(RETAINS.load(Ordering::SeqCst), retains + 1);
    let projected = equivalent.query::<IDerivedAbi>().unwrap();
    assert_eq!(RETAINS.load(Ordering::SeqCst), retains + 2);
    assert_eq!(projected.value(), Ok(42));
    drop((owner, equivalent));
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert_eq!(projected.value(), Ok(42));
    drop(projected);
    assert_eq!(RELEASES.load(Ordering::SeqCst), releases + 3);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

/// Box identity queries move the sole owner and return it unchanged on revision failure.
#[test]
fn boxed_identity_queries_transfer_once_and_reject_the_last_digest_byte() {
    let drops = Arc::new(AtomicUsize::new(0));
    let owner = ExclusiveValue::into_box(ExclusiveValue {
        state: State {
            closed: Cell::new(false),
            drops: drops.clone(),
        },
    })
    .query::<IDerivedAbi>()
    .unwrap();
    let original = owner.as_raw();
    let (status, owner) = owner.query::<OtherDerivedRevision>().unwrap_err();
    assert_eq!(status, neocogi_com_sdk::NC_E_ABI_VERSION);
    assert_eq!(owner.as_raw(), original);
    assert_eq!(owner.value(), Ok(42));
    let equivalent = owner.query::<EquivalentDerived>().unwrap();
    assert_eq!(equivalent.as_raw().cast::<IDerivedAbi>(), original);
    let owner = equivalent.query::<IDerivedAbi>().unwrap();
    assert_eq!(owner.as_raw(), original);
    assert_eq!(owner.value(), Ok(42));
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

/// Borrowing and inherited lookup never acquire another lifecycle token.
#[test]
fn method_view_reborrows_have_no_lifecycle_traffic() {
    let drops = Arc::new(AtomicUsize::new(0));
    let owner = SharedValue::from_value(SharedValue {
        state: State {
            closed: Cell::new(false),
            drops: drops.clone(),
        },
    })
    .query::<IDerivedAbi>()
    .unwrap();
    let raw = owner.into_raw();
    // SAFETY: this table has the exact generated identity, method thunks and
    // allocation type. It consumes the original owner token without retaining.
    let owner = unsafe {
        ComPtr::<IDerivedAbi>::from_raw_owned(neocogi_com_sdk::ComRaw {
            object: raw.object,
            vtable: &COUNTED_TABLE,
        })
    }
    .unwrap();
    let pair = owner.as_raw();
    let counts = || {
        (
            RETAINS.load(Ordering::SeqCst),
            RELEASES.load(Ordering::SeqCst),
            DOWNGRADES.load(Ordering::SeqCst),
        )
    };
    let before = counts();
    let wrapper: &<IDerivedAbi as neocogi_com_sdk::ComRustInterface>::Wrapper<neocogi_com_sdk::ThreadLocal> = &owner;
    let named: &IDerivedWrapper = wrapper;
    assert_eq!(named.value(), Ok(42));
    {
        let borrowed = owner.as_com_ref();
        let base: &IValueWrapper = &borrowed;
        assert_eq!(base.value(), Ok(42));
        base.close().unwrap();
        assert_eq!(borrowed.as_raw(), pair);
    }
    assert_eq!(owner.value(), Err(NC_E_CLOSED));
    assert_eq!(owner.as_raw(), pair);
    assert_eq!(counts(), before);
    let clone = owner.clone();
    assert_eq!(RETAINS.load(Ordering::SeqCst), before.0 + 1);
    drop(clone);
    drop(owner);
    assert_eq!(RELEASES.load(Ordering::SeqCst), before.1 + 2);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

/// Positive cross-thread marker proof for direct unsized wrapper references.
#[com_object(IShared)]
struct SharedCapability;
impl IShared for SharedCapability {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(7)
    }
}

/// Shared receivers can change the same interior state behind either owner.
#[test]
fn shared_inherited_views_preserve_identity_and_lifecycle() {
    let drops = Arc::new(AtomicUsize::new(0));
    let local = SharedValue::into_local(SharedValue {
        state: State {
            closed: Cell::new(false),
            drops: drops.clone(),
        },
    });
    let owner = local.query::<IDerivedAbi>().unwrap();
    let original = owner.as_raw();
    let borrowed = owner.as_com_ref();
    assert_eq!(borrowed.as_raw(), original);
    borrowed.close().unwrap();
    assert_eq!(local.get().value(), Err(NC_E_CLOSED));
    assert_eq!(std::mem::size_of_val(&borrowed), 2 * std::mem::size_of::<usize>());
    assert_eq!(std::mem::size_of_val(&*owner), 2 * std::mem::size_of::<usize>());
    assert_eq!(std::mem::align_of_val(&*owner), std::mem::align_of::<usize>());
    assert_eq!(std::mem::size_of::<Option<ComPtr<IDerivedAbi>>>(), std::mem::size_of_val(&owner));
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(local);
    assert_eq!(drops.load(Ordering::SeqCst), 1);

    let exclusive_drops = Arc::new(AtomicUsize::new(0));
    let mut exclusive = ExclusiveValue::into_box(ExclusiveValue {
        state: State {
            closed: Cell::new(false),
            drops: exclusive_drops.clone(),
        },
    })
    .query::<IDerivedAbi>()
    .unwrap();
    let original = exclusive.as_raw();
    let borrowed = exclusive.as_com_mut();
    assert_eq!(borrowed.value(), Ok(42));
    borrowed.close().unwrap();
    assert_eq!(borrowed.as_raw(), original);
    assert_eq!(exclusive.value(), Err(NC_E_CLOSED));
    drop(exclusive);
    assert_eq!(exclusive_drops.load(Ordering::SeqCst), 1);
}

/// Swapping complete owners is still ordinary valid ownership transfer.
#[test]
fn whole_owner_swaps_and_thread_safe_wrapper_references_remain_valid() {
    fn shared<T: ?Sized + Send + Sync>() {}
    shared::<ISharedWrapper<ThreadSafe>>();
    let shared_owner = SharedCapability::from_value(SharedCapability).query::<ISharedAbi>().unwrap();
    assert_eq!(shared_owner.value(), Ok(7));

    let drops = Arc::new(AtomicUsize::new(0));
    let new = || {
        SharedValue::from_value(SharedValue {
            state: State {
                closed: Cell::new(false),
                drops: drops.clone(),
            },
        })
        .query::<IDerivedAbi>()
        .unwrap()
    };
    let mut first = new();
    let mut second = new();
    let (a, b) = (first.as_raw(), second.as_raw());
    std::mem::swap(&mut first, &mut second);
    assert_eq!((first.as_raw(), second.as_raw()), (b, a));
    first.close().unwrap();
    assert_eq!(second.value(), Ok(42));
    drop((first, second));
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}
