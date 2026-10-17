//! Real byte/string provider entrypoints share the exact strong-only counter contract.

use super::*;
use core::sync::atomic::Ordering;

#[cfg(all(unix, not(miri)))]
#[path = "../../../test-support/retain_abort.rs"]
mod retain_abort;

/// Cloning and interface negotiation must both stop before publishing an uncounted owner.
#[cfg(all(unix, not(miri)))]
#[test]
fn retain_exhaustion_aborts_in_real_data_entrypoints() {
    for case in ["bytes-clone", "bytes-query", "string-clone", "string-query"] {
        retain_abort::assert_aborts("data::tests::retain_exhaustion_aborts_in_real_data_entrypoints", case, || {
            let raw = match case {
                "bytes-clone" | "bytes-query" => {
                    let raw = BytesObject::allocate(Vec::from(b"bytes"));
                    // SAFETY: a fresh, initialized allocation has not escaped.
                    // Inject only its counter, without inventing foreign storage.
                    unsafe { (*raw.object.cast::<BytesObject>()).ref_count.store(u32::MAX, Ordering::Relaxed) };
                    raw.cast::<crate::IUnknown>()
                }
                "string-clone" | "string-query" => {
                    let raw = StringObject::allocate(String::from("text"));
                    // SAFETY: same exclusive test setup, with the real string layout.
                    unsafe { (*raw.object.cast::<StringObject>()).ref_count.store(u32::MAX, Ordering::Relaxed) };
                    raw.cast::<crate::IUnknown>()
                }
                _ => unreachable!("closed set of provider cases"),
            };
            // SAFETY: each primary table starts with IUnknown and owns this storage.
            let owner = unsafe { ComPtr::<IUnknown>::from_raw_owned(raw) }.unwrap();
            match case {
                "bytes-clone" | "string-clone" => drop(owner.clone()),
                "bytes-query" | "string-query" => drop(owner.query::<IUnknown>().unwrap()),
                _ => unreachable!("closed set of provider operations"),
            }
        });
    }
}

/// Both provider kinds balance clone/query owners and release their allocation at zero.
#[test]
fn data_entrypoints_balance_owners_through_final_release() {
    for raw in [
        BytesObject::allocate(Vec::from(b"bytes")).cast::<crate::IUnknown>(),
        StringObject::allocate(String::from("text")).cast::<crate::IUnknown>(),
    ] {
        // SAFETY: both initialized allocations expose this common lifecycle prefix.
        // Every release below consumes one successful retain/query or initial owner;
        // nothing accesses provider storage after its final release.
        unsafe {
            assert_eq!(((*raw.vtable).add_ref)(raw.object), 2);
            let mut projection = ComRaw::null();
            assert_eq!(((*raw.vtable).query_interface)(raw.object, &IUnknown::INTERFACE_ID, &mut projection), NC_OK);
            assert_eq!(projection.object, raw.object);
            assert_eq!(((*raw.vtable).release)(raw.object), 2);
            assert_eq!(((*projection.vtable).release)(projection.object), 1);
            assert_eq!(((*raw.vtable).release)(raw.object), 0);
        }
    }
}

/// The last available count really increments through C and remains normally releasable.
#[test]
fn data_entrypoints_count_the_last_owner() {
    let raw = BytesObject::allocate(Vec::new());
    // SAFETY: this initialized provider remains privately owned throughout the test.
    // Synthetic owners are removed before releasing its one real allocation owner.
    unsafe {
        let count = &(*raw.object.cast::<BytesObject>()).ref_count;
        count.store(u32::MAX - 1, Ordering::Relaxed);
        assert_eq!(((*raw.vtable).unknown.add_ref)(raw.object), u32::MAX);
        assert_eq!(count.load(Ordering::Relaxed), u32::MAX);
        assert_eq!(((*raw.vtable).unknown.release)(raw.object), u32::MAX - 1);
        count.store(1, Ordering::Relaxed);
        assert_eq!(((*raw.vtable).unknown.release)(raw.object), 0);
    }
}

/// A generic borrow negotiates optional weak support without assuming provider storage.
/// Both immutable providers deliberately expose only their private strong counter.
#[test]
fn borrowed_data_rejects_unsupported_weak_lifetimes() {
    let bytes = NcBytes::from_vec(vec![1, 2, 3]);
    let text = NcString::from_utf8_str("text");
    for value in [bytes.query::<IUnknown>().unwrap(), text.query::<IUnknown>().unwrap()] {
        assert_eq!(value.downgrade().unwrap_err(), NC_E_NOINTERFACE);
        assert_eq!(value.as_com_ref().downgrade().unwrap_err(), NC_E_NOINTERFACE);
    }
    // Failed capability acquisition must leave the original values usable.
    assert_eq!(bytes.to_vec().unwrap(), vec![1, 2, 3]);
    assert_eq!(text.to_string().unwrap(), "text");
}
