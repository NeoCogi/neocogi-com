//! Counts lifecycle calls on a foreign-style object to distinguish a borrowed
//! dispatch from an AddRef/call/Release sequence with the same final count.

use neocogi_com::{com_interface, ComInterface, ComPtr, IUnknown, IUnknownVTable, NcInterfaceId, NcStatus, NC_E_NOINTERFACE, NC_OK};
use std::{cell::Cell, ffi::c_void, rc::Rc};

#[com_interface(auto_guid = "nc.tests.borrowed.Counter:v1", assigned_signature = b"00000000000000000000000000000001")]
/// Fixed interface used by owning and borrowed generated clients.
pub trait ICounter {
    /// Returns the number of AddRef calls observed so far.
    fn additions(&self) -> Result<u32, NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.borrowed.DerivedCounter:v1", assigned_signature = b"00000000000000000000000000000002")]
/// A derived contract with no local slots exercises inherited dispatch alone.
pub trait IDerivedCounter: ICounter {}

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
    /// Projected interface at offset zero; points back to this allocation.
    header: IDerivedCounterAbi,
    /// Strong references held by clients of this test object.
    references: Cell<u32>,
    /// Shared observations remain private to the test implementation binary.
    counts: Rc<Counts>,
}

/// Recovers test storage while the caller keeps its interface reference alive.
unsafe fn storage<'a>(this: *mut IUnknown) -> &'a Counter {
    // SAFETY: All test headers carry their live Counter allocation back-pointer.
    unsafe { &*((*this).object.cast::<Counter>()) }
}

/// This probe has no additional interface projection; failure transfers nothing.
unsafe extern "C" fn query(_this: *mut IUnknown, _id: *const NcInterfaceId, out: *mut *mut c_void) -> NcStatus {
    // SAFETY: The test's raw callers provide a writable pointer output slot.
    unsafe { *out = std::ptr::null_mut() };
    NC_E_NOINTERFACE
}

/// Records every ownership acquisition and keeps its allocation alive.
unsafe extern "C" fn add_ref(this: *mut IUnknown) -> u32 {
    // SAFETY: The caller holds a live reference for this lifecycle call.
    let value = unsafe { storage(this) };
    value.counts.additions.set(value.counts.additions.get() + 1);
    value.references.set(value.references.get() + 1);
    value.references.get()
}

/// Consumes one reference and destroys the allocation exactly at zero.
unsafe extern "C" fn release(this: *mut IUnknown) -> u32 {
    // SAFETY: The caller transfers one live reference to this function.
    let value = unsafe { storage(this) };
    value.counts.releases.set(value.counts.releases.get() + 1);
    let remaining = value.references.get() - 1;
    value.references.set(remaining);
    if remaining == 0 {
        // SAFETY: This is the last reference to the original Box allocation.
        unsafe { drop(Box::from_raw((*this).object.cast::<Counter>())) };
    }
    remaining
}

/// Reads observations without touching COM ownership.
unsafe extern "C" fn additions(this: *mut ICounterAbi, out: *mut u32) -> NcStatus {
    // SAFETY: The generated client supplies a live header and writable output.
    unsafe { *out = storage(this.cast()).counts.additions.get() };
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
        },
        additions,
    },
};

#[test]
fn borrowed_generated_calls_perform_no_lifecycle_operations() {
    let counts = Rc::new(Counts::default());
    let allocation = Box::into_raw(Box::new(Counter {
        header: IDerivedCounterAbi {
            vtable: &VTABLE,
            object: std::ptr::null_mut(),
        },
        references: Cell::new(1),
        counts: counts.clone(),
    }));
    // SAFETY: The allocation is private until its fully initialized header is published.
    let owner = unsafe {
        (*allocation).header.object = allocation.cast();
        ComPtr::<IDerivedCounterAbi>::from_raw_owned(&mut (*allocation).header).unwrap()
    };
    let borrowed = owner.as_com_ref();
    assert_eq!(borrowed.additions(), Ok(0));
    assert_eq!(owner.additions(), Ok(0));
    assert_eq!(counts.releases.get(), 0);
    let retained = borrowed.add_ref();
    assert_eq!(borrowed.additions(), Ok(1));
    drop(retained);
    assert_eq!(counts.releases.get(), 1);
    drop(owner);
    assert_eq!(counts.releases.get(), 2);
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
