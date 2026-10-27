//! Real byte/string provider entrypoints share the exact strong-only counter contract.

use super::*;
use core::sync::atomic::Ordering;

#[cfg(all(unix, not(miri)))]
#[path = "retain_abort.rs"]
mod retain_abort;

/// Closed inventory of the five independent successful query-retain branches.
/// All calls use the authentic primary source table, never a typed same-ID query.
#[derive(Clone, Copy, Debug)]
enum QueryCase {
    /// Bytes allocation negotiating its canonical unknown view.
    BytesUnknown,
    /// Bytes allocation negotiating its byte-sequence view.
    BytesBytes,
    /// String allocation negotiating its canonical unknown view.
    StringUnknown,
    /// String allocation negotiating its UTF-8 view.
    StringString,
    /// String allocation negotiating its distinct byte-sequence view.
    StringBytes,
}
impl QueryCase {
    /// Each success branch retains independently even when it shares a callback.
    const ALL: [Self; 5] = [Self::BytesUnknown, Self::BytesBytes, Self::StringUnknown, Self::StringString, Self::StringBytes];

    /// Stable subprocess selector; a case names the provider and requested contract.
    #[cfg(all(unix, not(miri)))]
    fn name(self) -> &'static str {
        match self {
            Self::BytesUnknown => "bytes-query-unknown",
            Self::BytesBytes => "bytes-query-bytes",
            Self::StringUnknown => "string-query-unknown",
            Self::StringString => "string-query-string",
            Self::StringBytes => "string-query-bytes",
        }
    }

    /// Allocates valid private storage and returns its common ABI prefix plus
    /// counter address. The allocation remains live until its final Release.
    fn allocate(self) -> (ComRaw<IUnknown>, *const AtomicU32) {
        match self {
            Self::BytesUnknown | Self::BytesBytes => {
                let raw = BytesObject::allocate(Vec::from(b"bytes"));
                // SAFETY: this is the matching fresh concrete provider allocation.
                let count = unsafe { &(*raw.object.cast::<BytesObject>()).ref_count };
                (raw.cast(), count)
            }
            Self::StringUnknown | Self::StringString | Self::StringBytes => {
                let raw = StringObject::allocate(String::from("text"));
                // SAFETY: this is the matching fresh concrete provider allocation.
                let count = unsafe { &(*raw.object.cast::<StringObject>()).ref_count };
                (raw.cast(), count)
            }
        }
    }

    /// Exact requested contract for this raw QueryInterface success branch.
    fn contract(self) -> &'static NcInterfaceId {
        match self {
            Self::BytesUnknown | Self::StringUnknown => &IUnknown::INTERFACE_ID,
            Self::BytesBytes | Self::StringBytes => &INcBytesAbi::INTERFACE_ID,
            Self::StringString => &INcStringAbi::INTERFACE_ID,
        }
    }

    /// Proves that the original primary interface remains callable after query.
    /// The caller owns the original allocation and has not performed final release.
    unsafe fn assert_original_usable(self, raw: ComRaw<IUnknown>) {
        let mut length = 0;
        // SAFETY: the closed case determines the real primary table type; both
        // output slots are initialized writable u64 storage for the call.
        let status = unsafe {
            match self {
                Self::BytesUnknown | Self::BytesBytes => ((*raw.cast::<INcBytesAbi>().vtable).len)(raw.object, &mut length),
                Self::StringUnknown | Self::StringString | Self::StringBytes => ((*raw.cast::<INcStringAbi>().vtable).byte_len)(raw.object, &mut length),
            }
        };
        assert_eq!(status, NC_OK);
        assert_eq!(
            length,
            match self {
                Self::BytesUnknown | Self::BytesBytes => 5,
                _ => 4,
            }
        );
    }
}

/// Clone and the actual C QueryInterface slots both stop before publishing an
/// uncounted owner. Full synthetic counts never escape their isolated children.
#[cfg(all(unix, not(miri)))]
#[test]
fn retain_exhaustion_aborts_in_real_data_entrypoints() {
    const TEST: &str = "data::tests::retain_exhaustion_aborts_in_real_data_entrypoints";
    retain_abort::assert_aborts(TEST, "bytes-clone", || {
        let raw = BytesObject::allocate(Vec::from(b"bytes"));
        // SAFETY: exclusively initialized real bytes storage; its authentic typed
        // owner keeps the selected primary table. The child must never return.
        unsafe { (*raw.object.cast::<BytesObject>()).ref_count.store(u32::MAX, Ordering::Relaxed) };
        let owner = unsafe { ComPtr::<INcBytesAbi>::from_raw_owned(raw) }.unwrap();
        drop(owner.clone());
    });
    retain_abort::assert_aborts(TEST, "string-clone", || {
        let raw = StringObject::allocate(String::from("text"));
        // SAFETY: same setup using the real string allocation and primary table.
        unsafe { (*raw.object.cast::<StringObject>()).ref_count.store(u32::MAX, Ordering::Relaxed) };
        let owner = unsafe { ComPtr::<INcStringAbi>::from_raw_owned(raw) }.unwrap();
        drop(owner.clone());
    });
    for case in QueryCase::ALL {
        retain_abort::assert_aborts(TEST, case.name(), || {
            let (raw, count) = case.allocate();
            // SAFETY: fresh initialized provider storage and readable exact
            // contract; the initialized output is writable for one raw pair.
            // Calling the C slot explicitly cannot take ComPtr's AddRef shortcut.
            unsafe {
                (*count).store(u32::MAX, Ordering::Relaxed);
                let mut projection = ComRaw::null();
                let status = ((*raw.vtable).query_interface)(raw.object, case.contract(), &mut projection);
                assert_eq!(status, NC_OK); // returning is still failure in assert_aborts
            }
        });
    }
}

/// Every ordinary query success branch owns exactly one new count.
#[test]
fn data_query_branches_retain_exactly_one_owner() {
    query_branches(1);
}

/// Independently selected last-count control remains mutation-sensitive even
/// when the ordinary control has already failed in the same test executable.
#[test]
fn data_query_branches_count_the_last_owner() {
    query_branches(u32::MAX - 1);
}

/// Exercises one count boundary, restoring synthetic excess before destruction.
fn query_branches(initial: u32) {
    for case in QueryCase::ALL {
        let (raw, count) = case.allocate();
        // SAFETY: initialized, private provider storage lives through all
        // counter reads/calls. No potentially unretained mutant projection
        // is adopted/released until the increment has actually been checked.
        unsafe {
            (*count).store(initial, Ordering::Relaxed);
            let mut projection = ComRaw::null();
            assert_eq!(((*raw.vtable).query_interface)(raw.object, case.contract(), &mut projection), NC_OK, "{case:?}");
            assert_eq!((*count).load(Ordering::Relaxed), initial + 1, "{case:?}");
            assert_eq!(projection.object, raw.object, "{case:?}");
            assert!(!projection.vtable.is_null());
            assert_eq!(*(*projection.vtable).interface_id, *case.contract(), "{case:?}");
            assert_eq!(((*projection.vtable).release)(projection.object), initial, "{case:?}");
            case.assert_original_usable(raw);
            (*count).store(1, Ordering::Relaxed);
            assert_eq!(((*raw.vtable).release)(raw.object), 0, "{case:?}");
            // No reads after final Release, including through count.
        }
    }
}

/// Both actual C AddRef callbacks count the last owner and remain releasable.
#[test]
fn data_entrypoints_count_the_last_owner() {
    for case in [QueryCase::BytesBytes, QueryCase::StringString] {
        let (raw, count) = case.allocate();
        // SAFETY: private initialized allocation; restore the one real owner
        // after consuming the synthetic retain and before final release.
        unsafe {
            (*count).store(u32::MAX - 1, Ordering::Relaxed);
            assert_eq!(((*raw.vtable).add_ref)(raw.object), u32::MAX);
            assert_eq!((*count).load(Ordering::Relaxed), u32::MAX);
            assert_eq!(((*raw.vtable).release)(raw.object), u32::MAX - 1);
            (*count).store(1, Ordering::Relaxed);
            assert_eq!(((*raw.vtable).release)(raw.object), 0);
        }
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
