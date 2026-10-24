//! Counts lifecycle calls on a foreign-style object to distinguish a borrowed
//! dispatch from an AddRef/call/Release sequence with the same final count.

#[allow(unused_imports, unused_macros)]
mod test_contracts {
    include!(concat!(env!("OUT_DIR"), "/test_contracts.rs"));
}
use neocogi_com::{ComInterface, ComPtr, IUnknown, IUnknownVTable, NcInterfaceId, NcStatus, NC_E_NOINTERFACE, NC_OK};
use std::{cell::Cell, ffi::c_void, rc::Rc};

extern crate self as fixture_borrowed_calls;
test_contracts::borrowed_calls!();

#[derive(Default)]
/// Observations kept alive after the foreign object's final release.
struct Counts {
    /// Every AddRef, including transient ownership that would hide in a net count.
    additions: Cell<u32>,
    /// Every Release, including the final destruction.
    releases: Cell<u32>,
}

#[repr(C)]
/// Heap-owned test object with a real reference-counted lifetime.
struct Counter {
    /// Strong references held by clients of this test object.
    references: Cell<u32>,
    /// Shared observations remain private to the test implementation binary.
    counts: Rc<Counts>,
}

/// Recovers test storage while the caller keeps its interface reference alive.
unsafe fn storage<'a>(this: *mut c_void) -> &'a Counter {
    // SAFETY: The paired table accepts this live Counter allocation directly.
    unsafe { &*(this.cast::<Counter>()) }
}

/// This probe has no additional interface projection; failure transfers nothing.
unsafe extern "C" fn query(_this: *mut c_void, _id: *const NcInterfaceId, out: *mut neocogi_com::ComRaw<IUnknown>) -> NcStatus {
    // SAFETY: The test's raw callers provide a writable pointer output slot.
    unsafe { *out = neocogi_com::ComRaw::null() };
    NC_E_NOINTERFACE
}

/// Records every ownership acquisition and keeps its allocation alive.
unsafe extern "C" fn add_ref(this: *mut c_void) -> u32 {
    // SAFETY: The caller holds a live reference for this lifecycle call.
    let value = unsafe { storage(this) };
    value.counts.additions.set(value.counts.additions.get() + 1);
    value.references.set(value.references.get() + 1);
    value.references.get()
}

/// Consumes one reference and destroys the allocation exactly at zero.
unsafe extern "C" fn release(this: *mut c_void) -> u32 {
    // SAFETY: The caller transfers one live reference to this function.
    let value = unsafe { storage(this) };
    value.counts.releases.set(value.counts.releases.get() + 1);
    let remaining = value.references.get() - 1;
    value.references.set(remaining);
    if remaining == 0 {
        // SAFETY: This is the last reference to the original Box allocation.
        unsafe { drop(Box::from_raw(this.cast::<Counter>())) };
    }
    remaining
}

/// Reads observations without touching COM ownership.
unsafe extern "C" fn additions(this: *mut c_void, out: *mut u32) -> NcStatus {
    // SAFETY: The generated client supplies a live allocation and writable output.
    unsafe { *out = storage(this.cast()).counts.additions.get() };
    NC_OK
}

/// Simulates a foreign implementation that transfers ownership but returns an
/// invalid boolean byte. The wrapper must release the transfer when decoding fails.
unsafe extern "C" fn malformed_output(
    this: *mut c_void,
    _offset: u64,
    len: u64,
    output: *mut u8,
    written: *mut u64,
    out: *mut neocogi_com::ComRaw<IDerivedCounterAbi>,
) -> NcStatus {
    assert_eq!(len, 1);
    // SAFETY: test calls supply one initialized writable byte and an output slot.
    // AddRef transfers a real reference even though the separate byte is malformed.
    unsafe {
        add_ref(this.cast());
        *output = 2;
        *written = 1;
        *out = neocogi_com::ComRaw { object: this, vtable: &VTABLE };
    }
    NC_OK
}

/// Immutable table whose prefix describes this exact projected interface.
static VTABLE: IDerivedCounterVTable = IDerivedCounterVTable {
    base: ICounterVTable {
        unknown: IUnknownVTable {
            interface_id: &IDerivedCounterAbi::INTERFACE_ID,
            query_interface: query,
            add_ref,
            release,
            downgrade: None,
        },
        additions,
        malformed_output,
    },
};

#[test]
fn borrowed_generated_calls_perform_no_lifecycle_operations() {
    let counts = Rc::new(Counts::default());
    let allocation = Box::into_raw(Box::new(Counter {
        references: Cell::new(1),
        counts: counts.clone(),
    }));
    // SAFETY: The allocation is private until its matching object/table pair is published.
    let owner = unsafe {
        ComPtr::<IDerivedCounterAbi>::from_raw_owned(neocogi_com::ComRaw {
            object: allocation.cast(),
            vtable: &VTABLE,
        })
        .unwrap()
    };
    let borrowed = owner.as_com_ref();
    assert_eq!(borrowed.additions(), Ok(0));
    assert_eq!(owner.additions(), Ok(0));
    assert_eq!(counts.releases.get(), 0);
    // A failed acquisition must preserve the safe bool and release the returned
    // reference immediately. The original caller still owns the object afterward.
    let mut destination = neocogi_com::NcOutputBuffer::<bool>::new();
    assert_eq!(borrowed.malformed_output(destination.output(0, 1)).unwrap_err(), neocogi_com::NC_E_INVALID_ARG);
    assert!(destination.is_empty());
    assert_eq!(counts.additions.get(), 1);
    assert_eq!(counts.releases.get(), 1);
    let retained = borrowed.try_add_ref().unwrap();
    assert_eq!(borrowed.additions(), Ok(2));
    drop(retained);
    assert_eq!(counts.releases.get(), 2);
    drop(owner);
    assert_eq!(counts.releases.get(), 3);
}

#[test]
fn borrowed_builtin_clients_share_copy_and_utf8_validation() {
    use neocogi_com::{NcString, INcStringComPtrExt, INcBytesComPtrExt, INcBytesAbi};
    let string = NcString::from_utf8_str("snowman: ☃");
    let borrowed = string.as_com_ref();
    assert_eq!(borrowed.to_string().unwrap(), "snowman: ☃");
    let bytes = string.query::<INcBytesAbi>().unwrap();
    assert_eq!(bytes.as_com_ref().to_vec().unwrap(), "snowman: ☃".as_bytes());
    let mut out = [0; 3];
    borrowed.copy_to(9, &mut out).unwrap();
    assert_eq!(&out, "☃".as_bytes());
}
