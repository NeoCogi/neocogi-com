//! Raw admission tests own real references and observe allocator-local cleanup.
use super::*;
use crate::{ComRaw, NcSharedWeakRaw, WeakComPtr, NC_E_ABI_VERSION, NC_E_NOINTERFACE};

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
    let mut raw = ComRaw::<IUnknown> { object: control.cast(), vtable: &copied };
    let original = raw;
    // SAFETY: resolution borrows the caller-owned slot without consuming it.
    let table = unsafe { crate::ComBox::<IUnknown>::__nc_validate_raw_input(&mut raw) }.unwrap();
    assert_eq!(table, &BOX_ROOT as *const _);
    assert_eq!(raw, original);
    // SAFETY: the copied table matches this genuine boxed allocation. Both
    // independent identity authorities are readable through checked adoption.
    let owner = unsafe { crate::ComBox::<IUnknown>::__nc_from_raw_owned_checked(raw) }.unwrap();
    assert_eq!(owner.as_raw().object, control.cast());
    assert_eq!(owner.as_raw().vtable, &BOX_ROOT as *const _);
    drop(owner);
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
            WeakComPtr::<crate::INcBytesAbi>::__nc_from_raw_owned_checked(NcSharedWeakRaw { control }).unwrap_err(),
            NC_E_NOINTERFACE
        );
    }
    assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
}

/// Non-present two-word shapes transfer no token, so neither dangling component
/// may be inspected while returning the precise pointer error.
#[test]
fn absent_and_half_null_boxed_owner_shapes_never_read_either_pointer() {
    let object = core::ptr::NonNull::<u8>::dangling().as_ptr().cast::<c_void>();
    let table = core::ptr::NonNull::<IUnknownVTable>::dangling().as_ptr();
    for mut raw in [
        ComRaw::null(),
        ComRaw { object, vtable: core::ptr::null() },
        ComRaw {
            object: core::ptr::null_mut(),
            vtable: table,
        },
    ] {
        // SAFETY: malformed non-present shapes explicitly transfer nothing.
        // The initialized records are readable; their components are never read.
        assert_eq!(
            unsafe { crate::ComBox::<IUnknown>::__nc_from_raw_owned_checked(raw) }.unwrap_err(),
            NC_E_POINTER
        );
        assert_eq!(unsafe { crate::ComBox::<IUnknown>::__nc_validate_raw_input(&mut raw) }, Err(NC_E_POINTER));
    }
}

/// A mutable nullable position rejects half-null pairs without touching storage.
#[test]
fn optional_mutable_loan_checks_presence_before_interface_metadata() {
    type Loan<'a> = crate::ComMut<'a, IUnknown>;
    let object = core::ptr::NonNull::<u8>::dangling().as_ptr().cast::<c_void>();
    let table = core::ptr::NonNull::<IUnknownVTable>::dangling().as_ptr();
    // SAFETY: null supplies absence and neither half-null value supplies a loan.
    unsafe {
        assert!(matches!(Loan::__nc_from_raw_optional_borrowed_checked(ComRaw::null()), Ok(None)));
        assert!(matches!(
            Loan::__nc_from_raw_optional_borrowed_checked(ComRaw { object, vtable: core::ptr::null() }),
            Err(NC_E_POINTER)
        ));
        assert!(matches!(
            Loan::__nc_from_raw_optional_borrowed_checked(ComRaw {
                object: core::ptr::null_mut(),
                vtable: table
            }),
            Err(NC_E_POINTER)
        ));
    }
}
