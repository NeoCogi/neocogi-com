//! Generated method lookup preserves the lender's pair, lifetime and token kind.

#[allow(unused_imports, unused_macros)]
mod test_contracts {
    include!(concat!(env!("OUT_DIR"), "/test_contracts.rs"));
}
extern crate self as fixture_weak_com;
test_contracts::weak_com!();

use neocogi_com::{com_object, ComPtr, NcStatus, ThreadSafe, NC_E_BUSY, NC_E_CLOSED, NC_E_DROPPED};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

/// Ordinary provider state; only destruction observation is shared externally.
struct State {
    /// Explicit logical closure is independent of object ownership.
    closed: bool,
    /// Survives final release so each ownership mode's destruction is observable.
    drops: Arc<AtomicUsize>,
}
impl Drop for State {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

/// Shared ownership lends the same unsized client as an ordinary call borrow.
#[com_object(IDerived)]
struct SharedValue {
    state: State,
}

/// Exclusive weak calls use the same facade but retain their own token policy.
#[com_object(IDerived, ownership = exclusive)]
struct ExclusiveValue {
    state: State,
}

/// Implements the same semantic operations for two intentionally different owners.
macro_rules! value_methods {
    ($provider:ty) => {
        impl IValue for $provider {
            fn value(&self) -> Result<u32, NcStatus> {
                if self.state.closed { Err(NC_E_CLOSED) } else { Ok(42) }
            }
            fn close(&mut self) -> Result<(), NcStatus> {
                self.state.closed = true;
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
    unsafe { neocogi_com::lifetime::nc_strong_retain(this) }
}
/// Forwards consumption of the instrumented projection's actual owner token.
unsafe extern "C" fn release(this: *mut core::ffi::c_void) -> u32 {
    RELEASES.fetch_add(1, Ordering::SeqCst);
    // SAFETY: each caller consumes exactly one retained SharedValue token.
    unsafe { neocogi_com::lifetime::nc_strong_release(this) }
}
/// Forwards weak acquisition without changing the provider's canonical control.
unsafe extern "C" fn downgrade(this: *mut core::ffi::c_void, out: *mut neocogi_com::NcWeakRaw) -> NcStatus {
    DOWNGRADES.fetch_add(1, Ordering::SeqCst);
    // SAFETY: the provider is live and out is the caller's writable token slot.
    unsafe { neocogi_com::lifetime::nc_downgrade(this, out) }
}

/// A real generated table with only lifecycle observation interposed.
static COUNTED_TABLE: IDerivedVTable = {
    let table = IDerivedVtblFor::<SharedValue>::VTABLE;
    IDerivedVTable {
        base: IValueVTable {
            unknown: neocogi_com::IUnknownVTable {
                add_ref: retain,
                release,
                downgrade: Some(downgrade),
                ..table.base.unknown
            },
            ..table.base
        },
    }
};

/// Borrowing and inherited mutable lookup never acquire another lifecycle token.
#[test]
fn method_view_reborrows_have_no_lifecycle_traffic() {
    let drops = Arc::new(AtomicUsize::new(0));
    let owner = SharedValue::from_value(SharedValue {
        state: State { closed: false, drops: drops.clone() },
    })
    .query::<IDerivedAbi>()
    .unwrap();
    let raw = owner.into_raw();
    // SAFETY: this table has the exact generated identity, method thunks and
    // allocation type. It consumes the original owner token without retaining.
    let owner = unsafe {
        ComPtr::<IDerivedAbi>::from_raw_owned(neocogi_com::ComRaw {
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
    {
        let mut borrowed = owner.as_com_ref();
        let base: &mut IValueClient = &mut borrowed;
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

/// Positive cross-thread marker proof for direct unsized client references.
#[com_object(IShared)]
struct SharedCapability;
impl IShared for SharedCapability {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(7)
    }
}

/// Every mutable inherited call crosses the actual generated ABI and call gate.
#[test]
fn mutable_inherited_views_preserve_identity_and_lifecycle() {
    let drops = Arc::new(AtomicUsize::new(0));
    let local = SharedValue::into_local(SharedValue {
        state: State { closed: false, drops: drops.clone() },
    });
    let mut owner = local.query::<IDerivedAbi>().unwrap();
    let original = owner.as_raw();
    assert_eq!(owner.value(), Ok(42));
    {
        let held = local.try_borrow().unwrap();
        assert_eq!(owner.close(), Err(NC_E_BUSY));
        drop(held);
        let held = local.try_borrow_mut().unwrap();
        assert_eq!(owner.value(), Err(NC_E_BUSY));
        drop(held);
    }
    let mut borrowed = owner.as_com_ref();
    borrowed.close().unwrap();
    assert_eq!(borrowed.value(), Err(NC_E_CLOSED));
    assert_eq!(owner.as_raw(), original);
    assert_eq!(borrowed.as_raw(), original);
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
            closed: false,
            drops: exclusive_drops.clone(),
        },
    })
    .query::<IDerivedAbi>()
    .unwrap();
    let original = exclusive.as_raw();
    let mut weak = exclusive.downgrade();
    assert_eq!(weak.value(), Ok(42));
    weak.close().unwrap();
    assert_eq!(exclusive.value(), Err(NC_E_CLOSED));
    exclusive.close().unwrap();
    assert_eq!(exclusive.as_raw(), original);
    assert_eq!(weak.as_raw(), original);
    drop(exclusive);
    assert_eq!(exclusive_drops.load(Ordering::SeqCst), 1);
    assert_eq!(weak.value(), Err(NC_E_DROPPED));
    assert_eq!(weak.close(), Err(NC_E_DROPPED));
    drop(weak);
    assert_eq!(exclusive_drops.load(Ordering::SeqCst), 1);
}

/// Swapping complete owners is still ordinary valid ownership transfer.
#[test]
fn whole_owner_swaps_and_thread_safe_client_references_remain_valid() {
    fn shared<T: ?Sized + Send + Sync>() {}
    shared::<ISharedClient<ThreadSafe>>();
    let shared_owner = SharedCapability::from_value(SharedCapability).query::<ISharedAbi>().unwrap();
    assert_eq!(shared_owner.value(), Ok(7));

    let drops = Arc::new(AtomicUsize::new(0));
    let new = || {
        SharedValue::from_value(SharedValue {
            state: State { closed: false, drops: drops.clone() },
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
