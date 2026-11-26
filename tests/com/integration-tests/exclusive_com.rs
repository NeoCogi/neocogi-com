//! Unique boxed owners and receiver-bound loans through generated ABI calls.
#[allow(unused_imports, unused_macros)]
mod test_contracts {
    include!(concat!(env!("OUT_DIR"), "/test_contracts.rs"));
}
use neocogi_com_sdk::{com_object, ComInterface, ComBox, ComRef, ComMut, ThreadSafe, ThreadSend, IUnknown, NcStatus, NC_E_BUSY, NC_E_INVALID_ARG};
use std::cell::{Cell, RefCell};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

extern crate self as fixture_exclusive_com;
test_contracts::exclusive_com!();
// Independent revisions deliberately share an IID and negotiate by signature.
extern crate self as fixture_exclusive_com_other_revision;
test_contracts::exclusive_com_other_revision!();

#[com_object(interfaces(IValue, ITransferCell), thread_safe, ownership = exclusive)]
/// Test value with external destruction observations.
struct Value {
    /// Plain state exposed through the owner's shared and exclusive borrows.
    value: u32,
    /// Counts concrete value destruction without owning the value.
    drops: Arc<AtomicUsize>,
}
impl IValue for Value {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.value)
    }
    fn set(&mut self, value: u32) -> Result<(), NcStatus> {
        self.value = value;
        Ok(())
    }
}
impl ITransferCell for Value {
    fn value(&self) -> Result<u32, NcStatus> {
        IValue::value(self)
    }
    fn set(&mut self, value: u32) -> Result<(), NcStatus> {
        IValue::set(self, value)
    }
}
impl Drop for Value {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

fn value() -> Value {
    Value {
        value: 42,
        drops: Arc::new(AtomicUsize::new(0)),
    }
}

thread_local! {
    /// Distinguishes copied dispatch from identical promoted constants on this thread.
    static COPIED_VALUE_CALLS: Cell<usize> = const { Cell::new(0) };
}
/// Forwards the exact generated checked call while observing use of copied dispatch.
unsafe extern "C" fn copied_value_call(object: *mut core::ffi::c_void, output: *mut u32) -> NcStatus {
    COPIED_VALUE_CALLS.set(COPIED_VALUE_CALLS.get() + 1);
    // SAFETY: this matching Value allocation/output satisfies the original thunk's contract.
    unsafe { (IValueVtblFor::<Value>::VTABLE.value)(object, output) }
}
/// An equivalent method table with observable forwarding, rather than pointer aliases.
static COPIED_VALUE_TABLE: IValueVTable = IValueVTable {
    value: copied_value_call,
    ..IValueVtblFor::<Value>::VTABLE
};
/// The supplied revision is stale only in its final byte; all method storage is valid.
static STALE_VALUE_DIGEST: [u8; 32] = {
    // SAFETY: generated interface identity has permanent initialized digest bytes.
    let mut bytes = unsafe { *IValueAbi::ABI_SIGNATURE.as_digest().unwrap() };
    bytes[31] ^= 1;
    bytes
};
/// Exact IID with an intentionally different expected contract revision.
static STALE_VALUE_ID: neocogi_com_sdk::NcInterfaceId =
    neocogi_com_sdk::NcInterfaceId::new(IValueAbi::IID, neocogi_com_sdk::NcAbiDigest::from_static(&STALE_VALUE_DIGEST));
/// Matching Value entry points with stale incoming identity for raw rejection tests.
static STALE_VALUE_TABLE: IValueVTable = IValueVTable {
    unknown: neocogi_com_sdk::IUnknownVTable {
        interface_id: &STALE_VALUE_ID,
        ..COPIED_VALUE_TABLE.unknown
    },
    ..COPIED_VALUE_TABLE
};

/// Transfers a genuine Value owner into its equivalent noncanonical table.
fn copied_value_owner(state: Value) -> ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe> {
    let expected = state.value;
    let raw = Value::into_box(state).query::<IValueAbi>().unwrap().into_raw();
    assert_ne!(raw.vtable, &COPIED_VALUE_TABLE as *const _);
    // SAFETY: object, threading, identity and methods match this concrete Value.
    // Move its existing sole owner; the copied table and provider code are static.
    let owner: ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe> = unsafe {
        ComBox::from_raw_owned(neocogi_com_sdk::ComRaw {
            object: raw.object,
            vtable: &COPIED_VALUE_TABLE,
        })
    }
    .unwrap();
    let calls = COPIED_VALUE_CALLS.get();
    assert_eq!(owner.value(), Ok(expected));
    assert_eq!(COPIED_VALUE_CALLS.get(), calls + 1);
    owner
}

/// Existing shared-ownership protocols must not bypass the exclusive owner.
#[test]
fn borrowed_arguments_and_raw_queries_cannot_create_shared_owners() {
    let owner = Value::into_box(value()).query::<IValueAbi>().unwrap();
    let borrow = owner.as_com_ref();
    let raw = borrow.as_raw();
    let mut out = neocogi_com_sdk::ComRaw::null();
    // SAFETY: exact live common prefix and distinct writable output; the call
    // attempts a prohibited ownership acquisition, never malformed storage.
    unsafe {
        let table = &*raw.vtable.cast::<neocogi_com_sdk::IUnknownVTable>();
        assert_eq!((table.add_ref)(raw.object), 0);
        assert_eq!((table.query_interface)(raw.object, &IValueAbi::INTERFACE_ID, &mut out), NC_E_INVALID_ARG);
        assert!(!out.is_present());
        let mut weak = neocogi_com_sdk::NcSharedWeakRaw::null();
        assert_eq!((table.downgrade.unwrap())(raw.object, &mut weak), NC_E_INVALID_ARG);
    }
    assert_eq!(borrow.value(), Ok(42));
}

/// A borrowed worker runs only inside the owner's scope; a later owning
/// handoff can move that same value independently and destroy it on the worker.
#[test]
fn thread_safe_owner_and_scoped_loans_preserve_lifetime() {
    let state = value();
    let drops = state.drops.clone();
    let mut owner = Value::into_box(state).query::<IValueAbi>().unwrap();
    std::thread::scope(|scope| {
        let loan = owner.as_com_ref();
        let first = scope.spawn(move || loan.value());
        let second = scope.spawn(move || loan.value());
        assert_eq!(first.join().unwrap(), Ok(42));
        assert_eq!(second.join().unwrap(), Ok(42));
    });
    std::thread::scope(|scope| {
        let mut loan = owner.as_com_mut();
        scope.spawn(move || loan.set(53)).join().unwrap().unwrap();
    });
    assert_eq!(owner.value(), Ok(53));
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    std::thread::spawn(move || drop(owner)).join().unwrap();
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn owner_and_borrowed_handles_are_two_words() {
    assert_eq!(size_of::<ComBox<IUnknown>>(), 2 * size_of::<usize>());
    assert_eq!(size_of::<Option<ComBox<IUnknown>>>(), 2 * size_of::<usize>());
    assert_eq!(size_of::<ComRef<'_, IUnknown>>(), 2 * size_of::<usize>());
    assert_eq!(size_of::<ComMut<'_, IUnknown>>(), 2 * size_of::<usize>());
}

#[com_object(IContainer, ownership = exclusive)]
/// The receiver's borrow governs its ordinary owned state.
struct Container {
    /// The only strong owner, transferred into this field by insert.
    child: Option<ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe>>,
}
impl IContainer for Container {
    /// Validation failure leaves the input for caller-side Drop after this call.
    fn insert(
        &mut self,
        child: neocogi_com_sdk::ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe>,
        reject: bool,
    ) -> Result<(), (NcStatus, neocogi_com_sdk::ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe>)> {
        // Admission must give the implementation canonical dispatch, even when
        // the caller supplies a distinct equivalent table or this method rejects.
        assert_eq!(child.as_raw().vtable, child.query_ref::<IValueAbi>().unwrap().as_raw().vtable);
        if reject {
            return Err((NC_E_INVALID_ARG, child));
        }
        self.child = Some(child);
        Ok(())
    }
    /// The returned child keeps this receiver borrowed after native return.
    fn child(&self) -> Option<ComRef<'_, IValueAbi, ThreadSafe>> {
        self.child.as_ref().map(ComBox::as_com_ref)
    }
    /// The returned child keeps this receiver exclusively borrowed.
    fn child_mut(&mut self) -> Option<ComMut<'_, IValueAbi, ThreadSafe>> {
        self.child.as_mut().map(ComBox::as_com_mut)
    }
    fn child_read(&mut self) -> Option<ComRef<'_, IValueAbi, ThreadSafe>> {
        self.child.as_ref().map(ComBox::as_com_ref)
    }
    fn remove(&mut self) -> Result<(), NcStatus> {
        self.child = None;
        Ok(())
    }
}

#[test]
fn abi_container_transfer_and_removal_preserve_exclusive_ownership() {
    let mut container = Container::into_box(Container { child: None }).query::<IContainerAbi>().unwrap();
    assert!(container.child().is_none());
    let state = value();
    let drops = state.drops.clone();
    let child = Value::into_box(state).query::<IValueAbi>().unwrap();
    let original = child.as_raw();
    let (status, child) = container.insert(child, true).unwrap_err();
    assert_eq!(status, NC_E_INVALID_ARG);
    assert_eq!(child.as_raw(), original);
    assert_eq!(child.value(), Ok(42));
    container.insert(child, false).unwrap();

    let first = container.child().unwrap();
    let second = container.child().unwrap();
    assert_eq!(first.as_raw(), original);
    assert!(first.same_object(second));
    let mut child = container.child_mut().unwrap();
    std::thread::scope(|scope| {
        let mut loan = child.as_com_mut();
        scope.spawn(move || loan.set(57)).join().unwrap().unwrap();
    });
    assert_eq!(child.value(), Ok(57));
    assert_eq!(container.child_read().unwrap().value(), Ok(57));
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    container.remove().unwrap();
    assert!(container.child_mut().is_none());
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

/// Raw malformed inputs fail before transfer and leave the caller's owner balanced.
#[test]
fn raw_transfer_rejection_preserves_the_input_slot() {
    let mut container = Container::into_box(Container { child: None }).query::<IContainerAbi>().unwrap();
    let child = Value::into_box(value()).query::<IValueAbi>().unwrap();
    let raw = child.into_raw();
    let mut slot = raw;
    let target = container.as_raw();
    // SAFETY: both objects, the writable slot, and raw bool storage are valid.
    // Bool 2 is deliberately rejected by the checked codec, not read as Rust bool.
    unsafe {
        assert_eq!(((*target.vtable).insert)(target.object, &mut slot, 2), NC_E_INVALID_ARG);
        assert_eq!(slot.object, raw.object);
        assert_eq!(slot.vtable, raw.vtable);
        let child = ComBox::<IValueAbi, ThreadSafe>::from_raw_owned(slot).unwrap();
        assert_eq!(child.value(), Ok(42));
    }
    assert_eq!(container.remove(), Ok(()));
}

/// Metadata lookup precedes admission, while canonical adoption waits for the
/// complete call. Every rejection preserves both original caller pointer words.
#[test]
fn copied_boxed_input_tables_preserve_rejections_and_commit_canonical_dispatch() {
    let mut container = Container::into_box(Container { child: None }).query::<IContainerAbi>().unwrap();
    let state = value();
    let drops = state.drops.clone();
    let child = copied_value_owner(state);
    let canonical = child.query_ref::<IValueAbi>().unwrap().as_raw().vtable;
    let original = child.as_raw();
    let mut slot = child.into_raw();
    let target = container.as_raw();
    // SAFETY: real retained objects, distinct writable slot and matching method
    // entry points. Only identity or an initialized raw bool is malformed.
    unsafe {
        slot.vtable = &STALE_VALUE_TABLE;
        let stale = slot;
        assert_eq!(((*target.vtable).insert)(target.object, &mut slot, 0), neocogi_com_sdk::NC_E_ABI_VERSION);
        assert_eq!(slot, stale);
        slot = original;
        assert_eq!(((*target.vtable).insert)(target.object, &mut slot, 2), NC_E_INVALID_ARG);
        assert_eq!(slot, original);
        assert_eq!(((*target.vtable).insert)(target.object, &mut slot, 1), NC_E_INVALID_ARG);
        assert_eq!(slot, original);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        // Rebuild the same live owner only after proving the slot was returned.
        let child = ComBox::<IValueAbi, neocogi_com_sdk::ThreadSafe>::from_raw_owned(slot).unwrap();
        let (status, child) = container.insert(child, true).unwrap_err();
        assert_eq!(status, NC_E_INVALID_ARG);
        assert_eq!(child.as_raw(), original);
        container.insert(child, false).unwrap();
    }
    assert_eq!(container.child().unwrap().as_raw().vtable, canonical);
    container.remove().unwrap();
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

/// Concrete access and interface projections borrow the same unique owner.
#[test]
fn local_box_access_and_borrowed_projection_follow_the_owner() {
    let state = value();
    let drops = state.drops.clone();
    let mut local = Value::into_local_box(state);
    local.get_mut().value = 17;
    assert_eq!(local.get().value, 17);
    assert_eq!(local.query_ref::<IValueAbi>().unwrap().value(), Ok(17));
    let mut owner = local.into_interface::<IValueAbi>().unwrap();
    owner.as_com_mut().set(19).unwrap();
    assert_eq!(owner.value(), Ok(19));
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[com_object(IPair, ownership = exclusive)]
/// Ordinary fields own accepted values; dispatch admission belongs to COM.
struct Pair {
    /// Both inputs commit together, or neither is retained.
    values: Option<[ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe>; 2]>,
}
impl IPair for Pair {
    /// Match expresses the whole rejection/commit decision without transfer flags.
    fn offer(
        &mut self,
        left: ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe>,
        right: ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe>,
        accept: bool,
    ) -> Result<
        u32,
        (
            NcStatus,
            ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe>,
            ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe>,
        ),
    > {
        for owner in [&left, &right] {
            assert_eq!(owner.as_raw().vtable, owner.query_ref::<IValueAbi>().unwrap().as_raw().vtable);
        }
        match accept {
            false => Err((neocogi_com_sdk::NC_OK, left, right)),
            true => {
                self.values = Some([left, right]);
                Ok(2)
            }
        }
    }
}

/// Rejection returns both original owners and successful retry transfers them.
#[test]
fn multiple_owner_inputs_survive_rejection_and_retry() {
    let mut pair = Pair::into_box(Pair { values: None }).query::<IPairAbi>().unwrap();
    let left = value();
    let left_drops = left.drops.clone();
    let right = value();
    let right_drops = right.drops.clone();
    let left = Value::into_box(left).query::<IValueAbi>().unwrap();
    let right = Value::into_box(right).query::<IValueAbi>().unwrap();
    let originals = [left.as_raw(), right.as_raw()];
    let (status, left, right) = pair.offer(left, right, false).unwrap_err();
    assert_eq!(status, NC_E_INVALID_ARG);
    assert_eq!(left.as_raw(), originals[0]);
    assert_eq!(right.as_raw(), originals[1]);
    assert_eq!(pair.offer(left, right, true).unwrap(), 2);
    assert_eq!(left_drops.load(Ordering::SeqCst), 0);
    assert_eq!(right_drops.load(Ordering::SeqCst), 0);
    drop(pair);
    assert_eq!(left_drops.load(Ordering::SeqCst), 1);
    assert_eq!(right_drops.load(Ordering::SeqCst), 1);
}

/// A later bad input must leave earlier copied tables untouched; body rejection
/// restores every original pair after seeing canonical owners, then retry commits.
#[test]
fn copied_multi_owner_inputs_restore_each_original_table_before_retry() {
    let mut pair = Pair::into_box(Pair { values: None }).query::<IPairAbi>().unwrap();
    let left_state = value();
    let left_drops = left_state.drops.clone();
    let right_state = value();
    let right_drops = right_state.drops.clone();
    let left = copied_value_owner(left_state);
    let right = copied_value_owner(right_state);
    let originals = [left.as_raw(), right.as_raw()];
    let mut left_slot = left.into_raw();
    let mut right_slot = right.into_raw();
    let target = pair.as_raw();
    let mut output = 17;
    // SAFETY: both retained Value allocations/tables and all scalar/slot storage
    // are valid. The second incoming identity is deliberately stale, then repaired.
    let (left, right) = unsafe {
        right_slot.vtable = &STALE_VALUE_TABLE;
        let stale = right_slot;
        assert_eq!(
            ((*target.vtable).offer)(target.object, &mut left_slot, &mut right_slot, 1, &mut output),
            neocogi_com_sdk::NC_E_ABI_VERSION
        );
        assert_eq!(left_slot, originals[0]);
        assert_eq!(right_slot, stale);
        assert_eq!(output, 17);
        right_slot = originals[1];
        (
            ComBox::<IValueAbi, neocogi_com_sdk::ThreadSafe>::from_raw_owned(left_slot).unwrap(),
            ComBox::<IValueAbi, neocogi_com_sdk::ThreadSafe>::from_raw_owned(right_slot).unwrap(),
        )
    };
    let (status, left, right) = pair.offer(left, right, false).unwrap_err();
    assert_eq!(status, NC_E_INVALID_ARG);
    assert_eq!(left.as_raw(), originals[0]);
    assert_eq!(right.as_raw(), originals[1]);
    assert_eq!(left.value(), Ok(42));
    assert_eq!(right.value(), Ok(42));
    assert_eq!(pair.offer(left, right, true).unwrap(), 2);
    drop(pair);
    assert_eq!(left_drops.load(Ordering::SeqCst), 1);
    assert_eq!(right_drops.load(Ordering::SeqCst), 1);
}

/// Failed consuming interface selection also returns the original allocation,
/// so a convenience wrapper cannot accidentally discard a rejected ancestor.
#[test]
fn rejected_owner_query_returns_the_same_live_owner() {
    let owner = Value::into_box(value()).query::<IValueAbi>().unwrap();
    let (status, owner) = owner.query::<IContainerAbi>().unwrap_err();
    assert_eq!(status, neocogi_com_sdk::NC_E_NOINTERFACE);
    assert_eq!(owner.value(), Ok(42));
    drop(owner);
}

/// An identity projection moves the sole token without changing its table or
/// adding ownership; destruction still runs exactly once.
#[test]
fn owner_identity_projection_preserves_cached_table_and_destroys_once() {
    let state = value();
    let drops = state.drops.clone();
    let owner = Value::into_box(state).query::<IValueAbi>().unwrap();
    let original = owner.as_raw();
    let mut owner = owner.query::<IValueAbi>().unwrap();
    assert_eq!(owner.as_raw(), original);
    owner.set(57).unwrap();
    assert_eq!(owner.value(), Ok(57));
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

/// Comparing only GUIDs would incorrectly allow this revision to bypass lookup.
#[test]
fn owner_projection_checks_the_complete_revision_before_reusing_a_table() {
    let owner = Value::into_box(value()).query::<IValueAbi>().unwrap();
    let original = owner.as_raw();
    assert_eq!(IValueAbi::IID, IValueOtherRevisionAbi::IID);
    let (status, owner) = owner.query::<IValueOtherRevisionAbi>().unwrap_err();
    assert_eq!(status, neocogi_com_sdk::NC_E_ABI_VERSION);
    assert_eq!(owner.as_raw(), original);
    assert_eq!(owner.value(), Ok(42));
}

#[com_object(interfaces(), ownership = exclusive)]
#[repr(align(128))]
/// Generic over-aligned state exercises typed exclusive allocation.
struct AlignedBox<T: 'static> {
    /// Concrete payload survives local/interface projection changes.
    marker: T,
    /// Counts destruction independently of the owner.
    drops: Arc<AtomicUsize>,
}
impl<T> Drop for AlignedBox<T> {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

/// Provider-local storage preserves alignment and drops the exact payload.
#[test]
fn generic_exclusive_storage_preserves_alignment() {
    let drops = Arc::new(AtomicUsize::new(0));
    let local = AlignedBox::into_local_box(AlignedBox { marker: 29u32, drops: drops.clone() });
    let unknown = local.query_ref::<IUnknown>().unwrap();
    assert_eq!(unknown.as_raw().object as usize % 128, 0);
    let state = local.get();
    assert_eq!(state.marker, 29);
    assert_eq!(state as *const _ as usize % 128, 0);
    drop(local.into_interface::<IUnknown>().unwrap());
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

/// Borrowed projections preserve identity and reject an unrelated table or revision.
#[test]
fn borrowed_interface_queries_preserve_identity_and_errors() {
    let mut owner = Value::into_box(value()).query::<IValueAbi>().unwrap();
    let projection = owner.query_ref::<IValueAbi>().unwrap();
    assert_eq!(projection.as_raw(), owner.as_raw());
    assert_eq!(projection.value(), Ok(42));
    let mut edit = owner.as_com_mut();
    assert_eq!(edit.set(53), Ok(()));
    let projection = edit.as_com_ref();
    std::thread::scope(|scope| {
        scope.spawn(move || assert_eq!(projection.value(), Ok(53))).join().unwrap();
    });
    assert!(matches!(owner.query_ref::<IContainerAbi>(), Err(neocogi_com_sdk::NC_E_NOINTERFACE)));
    assert!(matches!(owner.query_ref::<IValueOtherRevisionAbi>(), Err(neocogi_com_sdk::NC_E_ABI_VERSION)));
    assert_eq!(owner.value(), Ok(53));
}

/// Root and generated projections reuse only the exclusive callback family.
#[test]
fn generated_exclusive_projections_select_their_policy_prefix() {
    use neocogi_com_sdk::lifetime::{SHARED_UNKNOWN_VTABLE, EXCLUSIVE_UNKNOWN_VTABLE};
    let state = value();
    let drops = state.drops.clone();
    let owner = Value::into_box(state);
    let root = owner.as_raw();
    let interface = owner.query::<IValueAbi>().unwrap();
    for raw in [root, interface.as_raw().cast()] {
        // SAFETY: interface retains the allocation and immutable tables for both views.
        let table = unsafe { &*raw.vtable };
        assert!(std::ptr::fn_addr_eq(table.release, EXCLUSIVE_UNKNOWN_VTABLE.release));
        assert!(std::ptr::fn_addr_eq(table.add_ref, EXCLUSIVE_UNKNOWN_VTABLE.add_ref));
        assert!(std::ptr::fn_addr_eq(table.query_interface, EXCLUSIVE_UNKNOWN_VTABLE.query_interface));
        assert!(std::ptr::fn_addr_eq(table.downgrade.unwrap(), EXCLUSIVE_UNKNOWN_VTABLE.downgrade.unwrap()));
        assert!(!std::ptr::fn_addr_eq(table.release, SHARED_UNKNOWN_VTABLE.release));
    }
    drop(interface);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[com_object(ITransferCell, ownership = exclusive)]
/// A Cell proves transfer support does not require shared access.
struct TransferCell(std::cell::Cell<u32>);

impl ITransferCell for TransferCell {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.0.get())
    }

    fn set(&mut self, value: u32) -> Result<(), NcStatus> {
        self.0.set(value);
        Ok(())
    }
}

#[com_object(ILocalLock, ownership = exclusive)]
/// The owner keeps a non-Send lock guard on its acquiring thread.
struct LocalLock(std::sync::MutexGuard<'static, u32>);

impl ILocalLock for LocalLock {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(*self.0)
    }
}

#[test]
fn boxed_transfer_and_sharing_follow_independent_provider_evidence() {
    let mut owner = TransferCell::into_box(TransferCell(std::cell::Cell::new(7)))
        .query::<ITransferCellAbi>()
        .unwrap();
    std::thread::scope(|scope| {
        let mut loan = owner.as_com_mut();
        scope.spawn(move || loan.set(42)).join().unwrap().unwrap();
    });
    assert_eq!(std::thread::spawn(move || owner.value()).join().unwrap(), Ok(42));

    static LOCK: std::sync::Mutex<u32> = std::sync::Mutex::new(19);
    let owner = LocalLock::into_box(LocalLock(LOCK.lock().unwrap())).query::<ILocalLockAbi>().unwrap();
    std::thread::scope(|scope| {
        let read = owner.as_com_ref();
        assert_eq!(scope.spawn(move || read.value()).join().unwrap(), Ok(19));
    });
    drop(owner);
    assert_eq!(*LOCK.try_lock().unwrap(), 19);
}

#[com_object(IGuardedCell)]
/// The local provider protects its replaceable child with a field-level borrow.
struct GuardedCell {
    child: RefCell<Option<ComBox<ITransferCellAbi, ThreadSend>>>,
}

impl IGuardedCell for GuardedCell {
    fn replace(&self, child: ComBox<ITransferCellAbi, ThreadSend>) -> Result<(), (NcStatus, ComBox<ITransferCellAbi, ThreadSend>)> {
        let Ok(mut field) = self.child.try_borrow_mut() else {
            return Err((NC_E_BUSY, child));
        };
        let old = field.replace(child);
        drop(field);
        drop(old);
        Ok(())
    }

    fn child_mut<R>(&self, lend: impl for<'loan> FnOnce(Result<Option<ComMut<'loan, ITransferCellAbi, ThreadSend>>, NcStatus>) -> R) -> R {
        match self.child.try_borrow_mut() {
            Ok(mut field) => lend(Ok(field.as_mut().map(ComBox::as_com_mut))),
            Err(_) => lend(Err(NC_E_BUSY)),
        }
    }
}

/// Only the visitor has the mutable loan. Its worker joins before the guard
/// ends, and a reentrant replacement returns the caller's original owner.
#[test]
fn scoped_mutable_result_holds_field_guard_through_joined_worker() {
    let provider = GuardedCell::from_value(GuardedCell { child: RefCell::new(None) })
        .query::<IGuardedCellAbi>()
        .unwrap();
    assert_eq!(IGuardedCellAbi::with_child_mut(provider.as_com_ref(), |_| ()), Ok(None));

    let first = value();
    let drops = first.drops.clone();
    let mut child = Value::into_box(first).query::<ITransferCellAbi>().unwrap();
    let initiator = std::thread::current().id();
    std::thread::scope(|scope| {
        let mut loan = child.as_com_mut().into_threading::<ThreadSend>();
        scope.spawn(move || loan.set(43)).join().unwrap().unwrap();
    });
    let first = child.as_raw();
    provider.replace(child.into_threading::<ThreadSend>()).unwrap();

    let second = Value::into_box(value()).query::<ITransferCellAbi>().unwrap().into_threading::<ThreadSend>();
    let second_raw = second.as_raw();
    let returned = IGuardedCellAbi::with_child_mut(provider.as_com_ref(), |mut loan| {
        assert_eq!(std::thread::current().id(), initiator);
        assert_eq!(loan.as_raw(), first);
        assert_eq!(loan.value(), Ok(43));
        let (status, returned) = provider.replace(second).unwrap_err();
        assert_eq!(status, NC_E_BUSY);
        assert_eq!(returned.as_raw(), second_raw);
        assert!(matches!(IGuardedCellAbi::with_child_mut(provider.as_com_ref(), |_| ()), Err(NC_E_BUSY)));
        std::thread::scope(|scope| {
            let mut loan = loan.as_com_mut();
            scope.spawn(move || loan.set(84)).join().unwrap().unwrap();
        });
        assert_eq!(loan.value(), Ok(84));
        returned
    })
    .unwrap()
    .unwrap();
    assert_eq!(IGuardedCellAbi::with_child_mut(provider.as_com_ref(), |loan| loan.value()), Ok(Some(Ok(84))));
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    provider.replace(returned).unwrap();
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
