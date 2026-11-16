//! Raw admission tests own real references and observe allocator-local cleanup.
use super::*;
use crate::{ComRaw, ComValue, NcWeakRaw, ValueBudget, WeakComBox, WeakComPtr, NC_E_ABI_VERSION, NC_E_NOINTERFACE};

/// This second zero-method table has valid boxed callbacks for the same concrete
/// storage, but is intentionally omitted from that allocation's canonical list.
static UNLISTED_BOX_TABLE: IUnknownVTable = IUnknownVTable {
    interface_id: &AlternateInterface::INTERFACE_ID,
    ..EXCLUSIVE_UNKNOWN_VTABLE
};
/// A readable matching-provider revision differing only in its final digest byte.
static STALE_UNKNOWN_BYTES: [u8; 32] = {
    // SAFETY: the intrinsic IUnknown digest is permanent initialized metadata.
    let mut bytes = unsafe { *IUnknown::ABI_SIGNATURE.as_digest().unwrap() };
    bytes[31] ^= 1;
    bytes
};
/// Identity borrows the fixture's permanent stale digest storage.
static STALE_UNKNOWN_ID: NcInterfaceId = NcInterfaceId::new(crate::IID_IUNKNOWN, crate::NcAbiDigest::from_static(&STALE_UNKNOWN_BYTES));
/// Methods and lifecycle are valid for boxed storage; only revision is stale.
static STALE_BOX_TABLE: IUnknownVTable = IUnknownVTable {
    interface_id: &STALE_UNKNOWN_ID,
    ..EXCLUSIVE_UNKNOWN_VTABLE
};

/// Incoming identity, canonical membership and canonicalization have different
/// responsibilities; none may be replaced by table-address equality.
#[test]
fn boxed_weak_wire_preserves_both_identity_authorities_without_retaining_errors() {
    let (control, observed) = boxed_allocation();
    unsafe { (*control).weak_retain() };
    for expired in [false, true] {
        let before = unsafe { (*control).weak_count.load(Ordering::Relaxed) };
        for (table, error) in [(&UNLISTED_BOX_TABLE, NC_E_NOINTERFACE), (&STALE_BOX_TABLE, NC_E_ABI_VERSION)] {
            let wire = ComRaw::<IUnknown> { object: control.cast(), vtable: table };
            // SAFETY: both fixture tables match boxed storage/callbacks. The
            // expected identity is deliberately different, with readable backing.
            assert_eq!(
                unsafe { WeakComBox::<IUnknown>::decode_value(&wire, &mut ValueBudget::default()) }.unwrap_err(),
                error
            );
            unsafe { (*control).weak_retain() };
            assert_eq!(unsafe { WeakComBox::<IUnknown>::__nc_from_raw_owned_checked(wire) }.unwrap_err(), error);
            assert_eq!(unsafe { (*control).weak_count.load(Ordering::Relaxed) }, before);
        }
        // The incoming identity matches AlternateInterface, but canonical
        // metadata omits it. Its zero-method table still matches this storage.
        let unlisted = ComRaw::<AlternateInterface> {
            object: control.cast(),
            vtable: &UNLISTED_BOX_TABLE,
        };
        assert_eq!(
            unsafe { WeakComBox::<AlternateInterface>::decode_value(&unlisted, &mut ValueBudget::default()) }.unwrap_err(),
            NC_E_NOINTERFACE
        );
        unsafe { (*control).weak_retain() };
        assert_eq!(
            unsafe { WeakComBox::<AlternateInterface>::__nc_from_raw_owned_checked(unlisted) }.unwrap_err(),
            NC_E_NOINTERFACE
        );
        assert_eq!(unsafe { (*control).weak_count.load(Ordering::Relaxed) }, before);
        let copied = IUnknownVTable { ..BOX_ROOT };
        let wire = ComRaw::<IUnknown> { object: control.cast(), vtable: &copied };
        let acquired = unsafe { WeakComBox::<IUnknown>::decode_value(&wire, &mut ValueBudget::default()) }.unwrap();
        assert_eq!(acquired.as_raw().vtable, &BOX_ROOT as *const _);
        unsafe { (*control).weak_retain() };
        let adopted = unsafe { WeakComBox::<IUnknown>::__nc_from_raw_owned_checked(wire) }.unwrap();
        assert_eq!(adopted.as_raw().vtable, &BOX_ROOT as *const _);
        drop((acquired, adopted));
        assert_eq!(unsafe { (*control).weak_count.load(Ordering::Relaxed) }, before);
        if !expired {
            unsafe { NcBoxedLifetimeState::release(control) };
        }
    }
    unsafe { NcBoxedLifetimeState::weak_release(control) };
    assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
}

/// Rejected boxed input slots retain their owner; rejected boxed output identity
/// consumes precisely that same statically known owner without policy dispatch.
#[test]
fn boxed_owner_identity_rejection_preserves_inputs_and_cleans_outputs() {
    /// Each rejection checks a real boxed allocation and consumes only its output token.
    fn reject<T: ComInterface<VTable = IUnknownVTable>>(table: &'static IUnknownVTable, expected: NcStatus) {
        let (control, observed) = boxed_allocation();
        let mut raw = ComRaw::<T> { object: control.cast(), vtable: table };
        let original = raw;
        // SAFETY: tables and complete boxed storage remain readable. Identity
        // mismatch is intentional; the input check must not consume its owner.
        unsafe {
            assert_eq!(crate::ComBox::<T>::__nc_validate_raw_input(&mut raw), Err(expected));
            assert_eq!(raw, original);
            assert_eq!((*control).owner_and_borrows.load(Ordering::Relaxed), BOX_OWNER);
            assert_eq!((*control).weak_count.load(Ordering::Relaxed), 1);
            assert_eq!(observed.destroyed.load(Ordering::SeqCst), 0);
            assert_eq!(crate::ComBox::<T>::__nc_from_raw_owned_checked(raw).unwrap_err(), expected);
        }
        assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
        assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
    }
    reject::<IUnknown>(&UNLISTED_BOX_TABLE, NC_E_NOINTERFACE);
    reject::<IUnknown>(&STALE_BOX_TABLE, NC_E_ABI_VERSION);
    // Incoming identity matches, but cannot invent allocation membership.
    reject::<AlternateInterface>(&UNLISTED_BOX_TABLE, NC_E_NOINTERFACE);
    reject::<AlternateInterface>(&BOX_ROOT, NC_E_NOINTERFACE);
}

/// Copied-table owner results select canonical dispatch without another reference.
#[test]
fn boxed_owner_results_cache_the_canonical_table_and_destroy_once() {
    let (control, observed) = boxed_allocation();
    let copied = IUnknownVTable { ..BOX_ROOT };
    assert_ne!(&copied as *const _, &BOX_ROOT as *const _);
    let raw = ComRaw::<IUnknown> { object: control.cast(), vtable: &copied };
    // SAFETY: the copied table matches this genuine boxed allocation. Both
    // independent identity authorities are readable through checked adoption.
    let owner = unsafe { crate::ComBox::<IUnknown>::__nc_from_raw_owned_checked(raw) }.unwrap();
    assert_eq!(owner.as_raw().object, control.cast());
    assert_eq!(owner.as_raw().vtable, &BOX_ROOT as *const _);
    assert_eq!(unsafe { (*control).weak_count.load(Ordering::Relaxed) }, 1);
    assert_eq!(unsafe { (*control).owner_and_borrows.load(Ordering::Relaxed) }, BOX_OWNER);
    let weak = owner.downgrade();
    drop(owner);
    assert!(!weak.is_alive());
    assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observed.freed.load(Ordering::SeqCst), 0);
    drop(weak);
    assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
}

/// A rejected expired shared result releases the final shared weak exactly once.
#[test]
fn shared_identity_rejection_frees_the_final_expired_weak() {
    let (control, observed) = allocation();
    unsafe {
        (*control).weak_retain();
        NcSharedLifetimeState::release(control);
        assert_eq!(
            WeakComPtr::<crate::INcBytesAbi>::__nc_from_raw_owned_checked(NcWeakRaw { control }).unwrap_err(),
            NC_E_NOINTERFACE
        );
    }
    assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
}

/// A rejected expired boxed result releases the final boxed weak exactly once.
#[test]
fn boxed_identity_rejection_frees_the_final_expired_weak() {
    let (control, observed) = boxed_allocation();
    unsafe {
        (*control).weak_retain();
        NcBoxedLifetimeState::release(control);
        let raw = ComRaw::<AlternateInterface> {
            object: control.cast(),
            vtable: &UNLISTED_BOX_TABLE,
        };
        assert_eq!(
            WeakComBox::<AlternateInterface>::__nc_from_raw_owned_checked(raw).unwrap_err(),
            NC_E_NOINTERFACE
        );
    }
    assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
}

/// Non-present two-word shapes transfer no token, so neither dangling component
/// may be inspected while returning the precise pointer error.
#[test]
fn absent_and_half_null_boxed_weak_shapes_never_read_either_pointer() {
    let object = core::ptr::NonNull::<u8>::dangling().as_ptr().cast::<c_void>();
    let table = core::ptr::NonNull::<IUnknownVTable>::dangling().as_ptr();
    for raw in [
        ComRaw::null(),
        ComRaw { object, vtable: core::ptr::null() },
        ComRaw {
            object: core::ptr::null_mut(),
            vtable: table,
        },
    ] {
        // SAFETY: malformed non-present shapes explicitly transfer nothing.
        // The initialized records are readable; their components are never read.
        assert_eq!(unsafe { WeakComBox::<IUnknown>::__nc_from_raw_owned_checked(raw) }.unwrap_err(), NC_E_POINTER);
        assert_eq!(
            unsafe { WeakComBox::<IUnknown>::decode_value(&raw, &mut ValueBudget::default()) }.unwrap_err(),
            NC_E_POINTER
        );
    }
}
