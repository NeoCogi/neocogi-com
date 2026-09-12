//! Unit tests for the facade crate's fundamental ownership and value types.
//!
//! These tests exercise atomic COM reference counts, GUID and status
//! ergonomics, owned byte/string implementations, bulk-copy bounds, threading
//! markers, static UTF-8 metadata, and plugin ABI-version validation. Runtime
//! integration and macro expansion have their own dedicated test targets, so
//! this module stays focused on behavior implemented directly by `neocogi-com`.

use super::*;
use core::ffi::c_void;
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::panic::{AssertUnwindSafe, catch_unwind};

const UNKNOWN_TEST_STATUS_CODE: i32 = -77;
const UNKNOWN_TEST_STATUS: NcStatus = NcStatus::from_code(UNKNOWN_TEST_STATUS_CODE);

com_bit_enum! {
    /// Typed flags used to exercise generated bit-set behavior.
    bitenum TestOptions: u16 {
        /// Empty option set.
        NONE = 0,
        /// First independent behavior.
        FIRST = 1,
        /// Second independent behavior.
        SECOND = 2,
        /// Declared alias combining both behaviors.
        BOTH = 3,
    }
}

#[test]
fn bit_enum_preserves_typed_combinations_and_unknown_bits() {
    let mut options = TestOptions::FIRST | TestOptions::SECOND;

    assert_eq!(options, TestOptions::BOTH);
    assert!(options.contains(TestOptions::FIRST));
    assert!(options.intersects(TestOptions::SECOND));
    assert!(options.is_valid());
    options &= TestOptions::FIRST;
    assert_eq!(options.raw(), 1);
    options ^= TestOptions::BOTH;
    assert_eq!(options, TestOptions::SECOND);
    assert!(TestOptions::NONE.is_empty());
    assert!(!TestOptions::from_raw(4).is_valid());
}

#[test]
fn add_ref_count_increments() {
    let ref_count = AtomicU32::new(1);

    assert_eq!(__com_add_ref_count(&ref_count), 2);
    assert_eq!(ref_count.load(Ordering::Relaxed), 2);
}

#[test]
fn release_ref_count_decrements() {
    let ref_count = AtomicU32::new(2);

    assert_eq!(__com_release_ref_count(&ref_count), Some(1));
    assert_eq!(ref_count.load(Ordering::Relaxed), 1);
}

#[test]
fn add_ref_count_does_not_wrap_at_u32_max() {
    let ref_count = AtomicU32::new(u32::MAX);
    let result = catch_unwind(AssertUnwindSafe(|| __com_add_ref_count(&ref_count)));

    if cfg!(debug_assertions) {
        assert!(result.is_err());
    } else {
        assert_eq!(result.unwrap(), u32::MAX);
    }
    assert_eq!(ref_count.load(Ordering::Relaxed), u32::MAX);
}

#[test]
fn release_ref_count_does_not_underflow_at_zero() {
    let ref_count = AtomicU32::new(0);
    let result = catch_unwind(AssertUnwindSafe(|| __com_release_ref_count(&ref_count)));

    if cfg!(debug_assertions) {
        assert!(result.is_err());
    } else {
        assert_eq!(result.unwrap(), None);
    }
    assert_eq!(ref_count.load(Ordering::Relaxed), 0);
}

#[test]
fn nc_guid_formats_canonical_uuid_text() {
    let guid = NcGuid::from_fields(0x7b3f_b46c, 0x83ab, 0x49e1, [0xa5, 0xc8, 0x2b, 0xc6, 0xd7, 0x38, 0x0d, 0x2e]);

    assert_eq!(guid.to_string(), "7b3fb46c-83ab-49e1-a5c8-2bc6d7380d2e");
}

#[test]
fn nc_guid_parses_lowercase_and_uppercase_uuid_text() {
    let expected = NcGuid::from_fields(0x7b3f_b46c, 0x83ab, 0x49e1, [0xa5, 0xc8, 0x2b, 0xc6, 0xd7, 0x38, 0x0d, 0x2e]);

    assert_eq!("7b3fb46c-83ab-49e1-a5c8-2bc6d7380d2e".parse::<NcGuid>().unwrap(), expected);
    assert_eq!("7B3FB46C-83AB-49E1-A5C8-2BC6D7380D2E".parse::<NcGuid>().unwrap(), expected);
}

#[test]
fn nc_guid_rejects_non_canonical_uuid_text() {
    assert_eq!(
        "7b3fb46c83ab49e1a5c82bc6d7380d2e".parse::<NcGuid>().unwrap_err(),
        ParseGuidError::InvalidLength { len: 32 }
    );
    assert_eq!(
        "7b3fb46c_83ab-49e1-a5c8-2bc6d7380d2e".parse::<NcGuid>().unwrap_err(),
        ParseGuidError::InvalidHyphen { index: 8 }
    );
    assert_eq!(
        "7b3fb46c-83ab-49e1-a5c8-2bc6d7380d2x".parse::<NcGuid>().unwrap_err(),
        ParseGuidError::InvalidHex { index: 35 }
    );
}

#[test]
fn nc_guid_uuid_bytes_are_canonical_not_native_memory_order() {
    let bytes = [0x7b, 0x3f, 0xb4, 0x6c, 0x83, 0xab, 0x49, 0xe1, 0xa5, 0xc8, 0x2b, 0xc6, 0xd7, 0x38, 0x0d, 0x2e];
    let guid = NcGuid::from_uuid_bytes(bytes);

    assert_eq!(guid.fields(), (0x7b3f_b46c, 0x83ab, 0x49e1, [0xa5, 0xc8, 0x2b, 0xc6, 0xd7, 0x38, 0x0d, 0x2e]));
    assert_eq!(guid.to_uuid_bytes(), bytes);
    assert_eq!(guid.to_string(), "7b3fb46c-83ab-49e1-a5c8-2bc6d7380d2e");
}

#[test]
fn nc_guid_nil_is_all_zeroes() {
    assert_eq!(NcGuid::nil(), NC_GUID_NIL);
    assert!(NcGuid::nil().is_nil());
    assert_eq!(NcGuid::nil().to_string(), "00000000-0000-0000-0000-000000000000");
}

#[test]
fn iunknown_iid_constant_matches_com_interface_iid() {
    assert_eq!(IID_IUNKNOWN, <IUnknown as ComInterface>::IID);
    assert_eq!(IID_IUNKNOWN.to_string(), "00000000-0000-0000-c000-000000000046");
    assert_eq!("00000000-0000-0000-c000-000000000046".parse::<NcGuid>().unwrap(), IID_IUNKNOWN);
}

#[test]
fn nc_status_exposes_raw_code_without_exposing_mutable_storage() {
    let status = UNKNOWN_TEST_STATUS;

    assert_eq!(status.code(), UNKNOWN_TEST_STATUS_CODE);
    assert!(status.is_err());
    assert!(!status.is_ok());
    assert!(!status.is_known());
    assert_eq!(status.name(), "NC_STATUS_UNKNOWN");
}

#[test]
fn nc_status_names_builtin_status_constants() {
    assert_eq!(NC_OK.code(), 0);
    assert!(NC_OK.is_ok());
    assert!(!NC_OK.is_err());
    assert!(NC_OK.is_known());
    assert_eq!(NC_OK.name(), "NC_OK");

    assert!(NC_E_POINTER.is_err());
    assert!(!NC_E_POINTER.is_ok());
    assert!(NC_E_POINTER.is_known());
    assert_eq!(NC_E_NOINTERFACE.name(), "NC_E_NOINTERFACE");
    assert_eq!(NC_E_POINTER.name(), "NC_E_POINTER");
    assert_eq!(NC_E_INVALID_TAG.name(), "NC_E_INVALID_TAG");
    assert_eq!(NC_E_INVALID_ARG.name(), "NC_E_INVALID_ARG");
    assert_eq!(NC_E_ABI_VERSION.name(), "NC_E_ABI_VERSION");
    assert_eq!(NC_E_OUT_OF_RANGE.name(), "NC_E_OUT_OF_RANGE");
    assert_eq!(NC_E_NOT_FOUND.name(), "NC_E_NOT_FOUND");
    assert_eq!(NC_E_INSUFFICIENT_BUFFER.name(), "NC_E_INSUFFICIENT_BUFFER");
    assert_eq!(NC_E_LOAD_FAILED.name(), "NC_E_LOAD_FAILED");
    assert_eq!(NC_E_OUT_OF_MEMORY.name(), "NC_E_OUT_OF_MEMORY");
}

#[test]
fn nc_status_display_includes_name_and_code() {
    assert_eq!(NC_E_POINTER.to_string(), "NC_E_POINTER(-2)");
    assert_eq!(UNKNOWN_TEST_STATUS.to_string(), format!("NC_STATUS_UNKNOWN({UNKNOWN_TEST_STATUS_CODE})"));
}

#[test]
fn nc_bytes_wraps_vec_u8_and_vec_i8() {
    assert_eq!(NC_COM_DEFAULT_OWNED_COPY_MAX_BYTES, 64 * 1024 * 1024);

    let bytes = NcBytes::from_vec(vec![0, 127, 128, 255]);

    assert_eq!(bytes.len().unwrap(), 4);
    assert!(!bytes.is_empty().unwrap());
    assert_eq!(bytes.byte_at(2).unwrap(), 128);
    assert_eq!(bytes.byte_at(4).unwrap_err(), NC_E_OUT_OF_RANGE);
    let mut slice = [0; 2];
    bytes.copy_to(1, &mut slice).unwrap();
    assert_eq!(slice, [127, 128]);
    let mut empty = [];
    bytes.copy_to(4, &mut empty).unwrap();
    assert_eq!(bytes.copy_to(3, &mut [0; 2]).unwrap_err(), NC_E_OUT_OF_RANGE);
    assert_eq!(bytes.to_vec().unwrap(), vec![0, 127, 128, 255]);
    assert_eq!(bytes.to_vec_bounded(4).unwrap(), vec![0, 127, 128, 255]);
    assert_eq!(bytes.to_vec_bounded(3).unwrap_err(), NC_E_OUT_OF_RANGE);
    assert_eq!(bytes.to_i8_vec().unwrap(), vec![0, 127, -128, -1]);
    assert_eq!(bytes.to_i8_vec_bounded(3).unwrap_err(), NC_E_OUT_OF_RANGE);

    let signed = NcBytes::from_i8_vec(vec![-1, 0, 1]);
    assert_eq!(signed.to_vec().unwrap(), vec![255, 0, 1]);
    assert_eq!(signed.to_i8_vec().unwrap(), vec![-1, 0, 1]);
}

#[test]
fn nc_string_wraps_utf8_and_queries_as_bytes() {
    let text = NcString::from_utf8_str("hi \u{2603}");

    assert_eq!(text.byte_len().unwrap(), 6);
    assert_eq!(text.to_string().unwrap(), "hi \u{2603}");
    assert_eq!(text.to_string_bounded(6).unwrap(), "hi \u{2603}");
    assert_eq!(text.to_bytes_bounded(5).unwrap_err(), NC_E_OUT_OF_RANGE);
    assert_eq!(text.byte_at(3).unwrap(), 0xe2);
    assert_eq!(text.byte_at(6).unwrap_err(), NC_E_OUT_OF_RANGE);
    let mut snowman = [0; 3];
    text.copy_to(3, &mut snowman).unwrap();
    assert_eq!(snowman, [0xe2, 0x98, 0x83]);
    let mut empty = [];
    text.copy_to(6, &mut empty).unwrap();
    let mut unchanged = [0xaa; 2];
    assert_eq!(text.copy_to(5, &mut unchanged).unwrap_err(), NC_E_OUT_OF_RANGE);
    assert_eq!(unchanged, [0xaa; 2]);
    let raw = text.as_mut_ptr();
    // SAFETY: `raw` is a live string interface. A zero-length copy permits a
    // null output pointer.
    assert_eq!(unsafe { ((*(*raw).vtable).copy_to)(raw, 6, 0, core::ptr::null_mut()) }, NC_OK);
    // SAFETY: `raw` is live; the call rejects the null nonempty output before
    // attempting a write.
    assert_eq!(unsafe { ((*(*raw).vtable).copy_to)(raw, 0, 1, core::ptr::null_mut()) }, NC_E_POINTER);
    let mut overflow_unchanged = [0xbb; 2];
    // SAFETY: `raw` is live and `overflow_unchanged` is valid for two writes;
    // the overflowing range is rejected before writing.
    assert_eq!(
        unsafe { ((*(*raw).vtable).copy_to)(raw, u64::MAX, 2, overflow_unchanged.as_mut_ptr()) },
        NC_E_OUT_OF_RANGE
    );
    assert_eq!(overflow_unchanged, [0xbb; 2]);

    let bytes = text.query::<INcBytesAbi>().unwrap();
    assert_eq!(bytes.to_vec().unwrap(), "hi \u{2603}".as_bytes());
}

struct BulkStringProbe {
    value: &'static [u8],
    reported_len: u64,
    query_calls: AtomicUsize,
    byte_at_calls: AtomicUsize,
    copy_to_calls: AtomicUsize,
}

unsafe extern "C" fn probe_string_query_interface(this: *mut IUnknown, interface_id: *const NcInterfaceId, out: *mut *mut c_void) -> NcStatus {
    if this.is_null() || interface_id.is_null() || out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: `this` is the IUnknown-compatible prefix of the live test string
    // interface and carries the installed probe object pointer.
    let probe = unsafe { &*((*this).object.cast::<BulkStringProbe>()) };
    probe.query_calls.fetch_add(1, Ordering::Relaxed);
    // SAFETY: `out` was checked non-null and the test caller provides one
    // writable pointer slot.
    unsafe { *out = core::ptr::null_mut() };
    // SAFETY: `interface_id` was checked non-null and is readable for the call.
    let interface_id = unsafe { &*interface_id };
    if interface_id.iid == IUnknown::IID {
        if *interface_id != IUnknown::INTERFACE_ID {
            return NC_E_ABI_VERSION;
        }
    } else if interface_id.iid == INcStringAbi::IID {
        if *interface_id != INcStringAbi::INTERFACE_ID {
            return NC_E_ABI_VERSION;
        }
    } else {
        return NC_E_NOINTERFACE;
    }
    // SAFETY: The test object uses the same header address for IUnknown and
    // INcStringAbi and remains live for the returned owned test reference.
    unsafe { *out = this.cast::<c_void>() };
    NC_OK
}

unsafe extern "C" fn probe_string_add_ref(_this: *mut IUnknown) -> u32 {
    1
}

unsafe extern "C" fn probe_string_release(_this: *mut IUnknown) -> u32 {
    0
}

unsafe extern "C" fn probe_string_len(this: *mut INcStringAbi, out: *mut u64) -> NcStatus {
    if this.is_null() || out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: The test installs a live `BulkStringProbe` as the object pointer.
    let probe = unsafe { &*((*this).object.cast::<BulkStringProbe>()) };
    // SAFETY: `out` was checked non-null and is writable for this call.
    unsafe { *out = probe.reported_len };
    NC_OK
}

unsafe extern "C" fn probe_string_byte_at(this: *mut INcStringAbi, index: u64, out: *mut u8) -> NcStatus {
    if this.is_null() || out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: The test installs a live `BulkStringProbe` as the object pointer.
    let probe = unsafe { &*((*this).object.cast::<BulkStringProbe>()) };
    probe.byte_at_calls.fetch_add(1, Ordering::Relaxed);
    let Ok(index) = usize::try_from(index) else {
        return NC_E_OUT_OF_RANGE;
    };
    let Some(value) = probe.value.get(index) else {
        return NC_E_OUT_OF_RANGE;
    };
    // SAFETY: `out` was checked non-null and is writable for this call.
    unsafe { *out = *value };
    NC_OK
}

unsafe extern "C" fn probe_string_copy_to(this: *mut INcStringAbi, offset: u64, len: u64, out: *mut u8) -> NcStatus {
    if this.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: The test installs a live `BulkStringProbe` as the object pointer.
    let probe = unsafe { &*((*this).object.cast::<BulkStringProbe>()) };
    probe.copy_to_calls.fetch_add(1, Ordering::Relaxed);
    let (Ok(offset), Ok(len)) = (usize::try_from(offset), usize::try_from(len)) else {
        return NC_E_OUT_OF_RANGE;
    };
    if len != 0 && out.is_null() {
        return NC_E_POINTER;
    }
    let Some(end) = offset.checked_add(len) else {
        return NC_E_OUT_OF_RANGE;
    };
    let Some(value) = probe.value.get(offset..end) else {
        return NC_E_OUT_OF_RANGE;
    };
    if !value.is_empty() {
        // SAFETY: The range was validated and `out` is writable for `len`
        // bytes under the test ABI call contract.
        unsafe { core::ptr::copy_nonoverlapping(value.as_ptr(), out, len) };
    }
    NC_OK
}

static PROBE_STRING_VTABLE: INcStringVTable = INcStringVTable {
    unknown: IUnknownVTable {
        interface_id: &INcStringAbi::INTERFACE_ID,
        query_interface: probe_string_query_interface,
        add_ref: probe_string_add_ref,
        release: probe_string_release,
    },
    byte_len: probe_string_len,
    byte_at: probe_string_byte_at,
    copy_to: probe_string_copy_to,
};

#[test]
fn foreign_string_to_bytes_uses_one_bulk_copy_without_inc_bytes() {
    static VALUE: &[u8] = b"foreign string bulk-copy probe";
    let probe = BulkStringProbe {
        value: VALUE,
        reported_len: VALUE.len() as u64,
        query_calls: AtomicUsize::new(0),
        byte_at_calls: AtomicUsize::new(0),
        copy_to_calls: AtomicUsize::new(0),
    };
    let mut interface = INcStringAbi {
        vtable: &PROBE_STRING_VTABLE,
        object: core::ptr::from_ref(&probe).cast_mut().cast::<c_void>(),
    };
    // SAFETY: `interface` and `probe` remain live until after `string` is
    // dropped, and the probe vtable accepts the owned test reference.
    let string = unsafe { ComPtr::<INcStringAbi>::from_raw_owned(&mut interface) }.unwrap();

    assert_eq!(string.to_bytes().unwrap(), VALUE);
    assert_eq!(probe.query_calls.load(Ordering::Relaxed), 0);
    assert_eq!(probe.copy_to_calls.load(Ordering::Relaxed), 1);
    assert_eq!(probe.byte_at_calls.load(Ordering::Relaxed), 0);
}

#[test]
fn foreign_string_rejects_unrepresentable_reported_lengths_before_copying() {
    let probe = BulkStringProbe {
        value: b"",
        reported_len: usize::MAX as u64,
        query_calls: AtomicUsize::new(0),
        byte_at_calls: AtomicUsize::new(0),
        copy_to_calls: AtomicUsize::new(0),
    };
    let mut interface = INcStringAbi {
        vtable: &PROBE_STRING_VTABLE,
        object: core::ptr::from_ref(&probe).cast_mut().cast::<c_void>(),
    };
    // SAFETY: `interface` and `probe` remain live until after `string` is
    // dropped, and the probe vtable accepts the owned test reference.
    let string = unsafe { ComPtr::<INcStringAbi>::from_raw_owned(&mut interface) }.unwrap();

    assert_eq!(string.to_bytes().unwrap_err(), NC_E_OUT_OF_RANGE);
    assert_eq!(string.to_bytes_bounded(usize::MAX).unwrap_err(), NC_E_OUT_OF_RANGE);
    assert_eq!(probe.copy_to_calls.load(Ordering::Relaxed), 0);
}

#[test]
fn nc_bytes_and_strings_support_thread_safe_pointer_mode() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<ComPtr<INcBytesAbi, ThreadSafe>>();
    assert_send_sync::<ComPtr<INcStringAbi, ThreadSafe>>();

    let bytes = NcBytes::from_slice_thread_safe(b"abc");
    assert_eq!(bytes.len().unwrap(), 3);

    let text = NcString::from_utf8_str_thread_safe("abc");
    assert_eq!(text.to_string().unwrap(), "abc");
}

#[test]
fn nc_status_implements_std_error() {
    fn assert_error<T: std::error::Error>() {}

    assert_error::<NcStatus>();
}

#[test]
fn static_utf8_exposes_static_metadata() {
    let value = NcStaticUtf8::from_static("Widget");

    assert_eq!(value.len(), 6);
    assert!(!value.is_empty());
    assert_eq!(unsafe { value.as_bytes() }.unwrap(), b"Widget");
    assert_eq!(unsafe { value.as_str() }.unwrap(), "Widget");
}
