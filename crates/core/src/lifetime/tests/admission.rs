//! Raw admission tests own real references and observe allocator-local cleanup.
use super::*;
use crate::{ComRaw, ComValue, NcWeakRaw, ValueBudget, WeakComBox, WeakComPtr, NC_E_ABI_VERSION, NC_E_NOINTERFACE};

/// Both transferred policies reject the other target before constructing an owner,
/// including when the rejected token is the final reference to expired storage.
#[test]
fn owned_wrong_policy_weak_rejection_consumes_exactly_one_actual_token() {
    for ownership in [NC_OWNERSHIP_SHARED, NC_OWNERSHIP_BOXED] {
        for expired in [false, true] {
            for final_token in [false, true] {
                let (object, observed, table) = match ownership {
                    NC_OWNERSHIP_SHARED => {
                        let (control, observed) = allocation();
                        (control.cast::<c_void>(), observed, &ROOT)
                    }
                    NC_OWNERSHIP_BOXED => {
                        let (control, observed) = boxed_allocation();
                        (control.cast::<c_void>(), observed, &BOX_ROOT)
                    }
                    _ => unreachable!(),
                };
                // SAFETY: real fixture headers, retained matching metadata and
                // exact lifecycle tables; every added token has one consumer.
                unsafe {
                    weak_retain(object); // Transferred token, not a borrowed copy.
                    if !final_token {
                        weak_retain(object);
                    }
                    if expired {
                        (table.release)(object);
                    }
                    let count = || match ownership {
                        NC_OWNERSHIP_SHARED => (*object.cast::<NcSharedLifetimeState>()).weak_count.load(Ordering::Relaxed),
                        NC_OWNERSHIP_BOXED => (*object.cast::<NcBoxedLifetimeState>()).weak_count.load(Ordering::Relaxed),
                        _ => unreachable!(),
                    };
                    let before = count();
                    let status = match ownership {
                        NC_OWNERSHIP_SHARED => WeakComBox::<IUnknown>::__nc_from_raw_owned_checked(ComRaw { object, vtable: table }).unwrap_err(),
                        NC_OWNERSHIP_BOXED => WeakComPtr::<IUnknown>::__nc_from_raw_owned_checked(NcWeakRaw { control: object }).unwrap_err(),
                        _ => unreachable!(),
                    };
                    assert_eq!(status, NC_E_INVALID_ARG);
                    if expired && final_token {
                        assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
                    } else {
                        assert_eq!(count(), before - 1);
                        assert_eq!(observed.freed.load(Ordering::SeqCst), 0);
                        if !final_token {
                            weak_release(object);
                        }
                        if !expired {
                            (table.release)(object);
                        }
                    }
                }
                assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
                assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
            }
        }
    }
}

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
    unsafe { weak_retain(control.cast()) };
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
            unsafe { weak_retain(control.cast()) };
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
        unsafe { weak_retain(control.cast()) };
        assert_eq!(
            unsafe { WeakComBox::<AlternateInterface>::__nc_from_raw_owned_checked(unlisted) }.unwrap_err(),
            NC_E_NOINTERFACE
        );
        assert_eq!(unsafe { (*control).weak_count.load(Ordering::Relaxed) }, before);
        let copied = IUnknownVTable { ..BOX_ROOT };
        let wire = ComRaw::<IUnknown> { object: control.cast(), vtable: &copied };
        let acquired = unsafe { WeakComBox::<IUnknown>::decode_value(&wire, &mut ValueBudget::default()) }.unwrap();
        assert_eq!(acquired.as_raw().vtable, &BOX_ROOT as *const _);
        unsafe { weak_retain(control.cast()) };
        let adopted = unsafe { WeakComBox::<IUnknown>::__nc_from_raw_owned_checked(wire) }.unwrap();
        assert_eq!(adopted.as_raw().vtable, &BOX_ROOT as *const _);
        drop((acquired, adopted));
        assert_eq!(unsafe { (*control).weak_count.load(Ordering::Relaxed) }, before);
        if !expired {
            unsafe { NcBoxedLifetimeState::release(control) };
        }
    }
    unsafe { weak_release(control.cast()) };
    assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
}
