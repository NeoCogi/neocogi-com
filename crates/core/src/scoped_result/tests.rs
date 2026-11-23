//! Continuation protocol tests with real common ABI backing.

use super::*;
use crate::serialization::scope;
use crate::{INcStringAbi, IUnknown, IUnknownVTable, NC_E_ABI_VERSION, NC_E_BUSY, NC_E_NOINTERFACE, NC_E_POINTER, NC_OK, NcAbiDigest, NcInterfaceId, ThreadLocal};

/// One valid thread-local scoped table. The lender supplies a live stack object.
static SCOPED: IUnknownVTable = IUnknownVTable {
    interface_id: &IUnknown::INTERFACE_ID,
    query_interface: scope::query,
    add_ref: scope::retain,
    release: scope::release,
    downgrade: None,
};

/// A foreign slot can deliver valid common storage with the wrong interface IID.
static OTHER_INTERFACE: IUnknownVTable = IUnknownVTable {
    interface_id: &INcStringAbi::INTERFACE_ID,
    ..SCOPED
};

/// The same IID with a different exact signature is independently rejected.
static OTHER_REVISION: IUnknownVTable = IUnknownVTable {
    interface_id: &NcInterfaceId::new(IUnknown::IID, NcAbiDigest::from_static(&[7; 32])),
    ..SCOPED
};

/// Raw callback shapes for the stack lender used by each protocol test.
fn view(lender: &scope::ScopedComProvider<u8>, vtable: &'static IUnknownVTable) -> ComRaw<IUnknown> {
    ComRaw {
        object: ptr::from_ref(lender).cast_mut().cast(),
        vtable,
    }
}

/// Tests one native outcome and optional delivery without adopting an owner.
fn deliver(raw: ComRaw<IUnknown>, status: NcStatus, visits: &Cell<usize>) -> Result<Option<*mut c_void>, NcStatus> {
    // SAFETY: Tests call this only with live scoped storage, a checked null shape,
    // or a readable common table with a mismatching identity.
    unsafe {
        __nc_with_scoped_result::<IUnknown, ThreadLocal, _>(
            |loan| {
                visits.set(visits.get() + 1);
                loan.as_raw().object
            },
            |context, callback| {
                callback(context, raw);
                status
            },
        )
    }
}

/// Some and None complete successfully, including a nonzero success status.
#[test]
fn present_and_absent_deliveries_are_distinct() {
    let lender = scope::ScopedComProvider::new(31u8);
    let raw = view(&lender, &SCOPED);
    let visits = Cell::new(0);
    assert_eq!(deliver(raw, NcStatus::from_code(3), &visits), Ok(Some(raw.object)));
    assert_eq!(deliver(ComRaw::null(), NC_OK, &visits), Ok(None));
    assert_eq!(visits.get(), 1);
}

/// Both half-null forms, a different IID and a different digest avoid user code.
#[test]
fn malformed_deliveries_do_not_invoke_the_visitor() {
    let lender = scope::ScopedComProvider::new(31u8);
    let raw = view(&lender, &SCOPED);
    let visits = Cell::new(0);
    for (pair, expected) in [
        (ComRaw { object: ptr::null_mut(), ..raw }, NC_E_POINTER),
        (ComRaw { vtable: ptr::null(), ..raw }, NC_E_POINTER),
        (view(&lender, &OTHER_INTERFACE), NC_E_NOINTERFACE),
        (view(&lender, &OTHER_REVISION), NC_E_ABI_VERSION),
    ] {
        assert_eq!(deliver(pair, NC_OK, &visits), Err(expected));
    }
    assert_eq!(visits.get(), 0);
}

/// Valid pre-delivery rejection differs from missing or late native failure.
#[test]
fn status_and_delivery_must_indicate_one_completion() {
    for (status, expected) in [(NC_E_BUSY, NC_E_BUSY), (NC_OK, NC_E_INVALID_ARG)] {
        // SAFETY: The native stand-in returns before calling the continuation.
        let result = unsafe { __nc_with_scoped_result::<IUnknown, ThreadLocal, ()>(|_| panic!("unexpected visitor entry"), |_, _| status) };
        assert_eq!(result, Err(expected));
    }
    let lender = scope::ScopedComProvider::new(31u8);
    let visits = Cell::new(0);
    assert_eq!(deliver(view(&lender, &SCOPED), NC_E_BUSY, &visits), Err(NC_E_INVALID_ARG));
    assert_eq!(visits.get(), 1);
}

/// Returning an application Result preserves it as caller-owned data.
#[test]
fn visitor_result_is_independent_of_native_status() {
    let lender = scope::ScopedComProvider::new(31u8);
    // SAFETY: The callback uses a live exact scoped pair on the invoking thread.
    let result = unsafe {
        __nc_with_scoped_result::<IUnknown, ThreadLocal, _>(
            |_| Err::<(), _>(NC_E_BUSY),
            |context, callback| {
                callback(context, view(&lender, &SCOPED));
                NC_OK
            },
        )
    };
    assert_eq!(result, Ok(Some(Err(NC_E_BUSY))));
}

/// Counts destruction while checking native code has released its borrowed state.
struct DropProbe<'a> {
    /// Whether the native entry still owns an active invocation.
    in_call: &'a Cell<bool>,
    /// Number of destructors completed.
    drops: &'a Cell<usize>,
}

impl Drop for DropProbe<'_> {
    fn drop(&mut self) {
        assert!(!self.in_call.get());
        self.drops.set(self.drops.get() + 1);
    }
}

/// Duplicate delivery under reentry must preserve R through native return.
#[test]
fn a_reentrant_duplicate_is_terminal_and_drops_the_result_after_return() {
    let lender = scope::ScopedComProvider::new(31u8);
    let reentry = Cell::new(None::<(*mut c_void, NcScopedResultCallback<IUnknown>)>);
    let in_call = Cell::new(false);
    let drops = Cell::new(0);
    let visits = Cell::new(0);
    // SAFETY: Both deliveries are on this thread with the exact live loan and
    // freshly installed context. Duplicate delivery is the tested protocol error.
    let result = unsafe {
        __nc_with_scoped_result::<IUnknown, ThreadLocal, _>(
            |_| {
                visits.set(visits.get() + 1);
                let (context, callback) = reentry.get().unwrap();
                callback(context, view(&lender, &SCOPED));
                DropProbe { in_call: &in_call, drops: &drops }
            },
            |context, callback| {
                in_call.set(true);
                reentry.set(Some((context, callback)));
                callback(context, view(&lender, &SCOPED));
                callback(context, view(&lender, &SCOPED));
                reentry.set(None);
                assert_eq!(drops.get(), 0);
                in_call.set(false);
                NC_OK
            },
        )
    };
    assert!(matches!(result, Err(NC_E_INVALID_ARG)));
    assert_eq!(visits.get(), 1);
    assert_eq!(drops.get(), 1);
}

/// An absent first delivery consumes the slot without consuming the Rust visitor.
#[test]
fn a_duplicate_after_absence_drops_the_unused_visitor_after_return() {
    let lender = scope::ScopedComProvider::new(31u8);
    let in_call = Cell::new(false);
    let drops = Cell::new(0);
    let probe = DropProbe { in_call: &in_call, drops: &drops };
    // SAFETY: Both deliveries use the same live context and synchronous thread.
    let result = unsafe {
        __nc_with_scoped_result::<IUnknown, ThreadLocal, _>(
            move |_| drop(probe),
            |context, callback| {
                in_call.set(true);
                callback(context, ComRaw::null());
                callback(context, view(&lender, &SCOPED));
                assert_eq!(drops.get(), 0);
                in_call.set(false);
                NC_OK
            },
        )
    };
    assert_eq!(result, Err(NC_E_INVALID_ARG));
    assert_eq!(drops.get(), 1);
}
