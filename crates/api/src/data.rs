//! Owned COM implementations and safe clients for byte strings and UTF-8 text.
//!
//! [`NcBytes`] and [`NcString`] allocate immutable Rust values behind
//! reference-counted ABI headers. A string exposes both `INcStringAbi` and
//! `INcBytesAbi`, with every header pointing back to one allocation so
//! `QueryInterface`, `AddRef`, and `Release` share the same identity and
//! lifetime. The extension traits turn raw vtable calls into checked Rust
//! results and place explicit limits on allocations requested by foreign
//! objects. The lower half of the module contains the concrete storage,
//! vtables, C entry points, and range-checked copy machinery.

use core::ffi::c_void;
use core::mem::{ManuallyDrop, MaybeUninit};
use core::sync::atomic::AtomicU32;

use neocogi_com_core::{
    ComInterface, INcBytesAbi, INcBytesVTable, INcStringAbi, INcStringVTable, IUnknown,
    IUnknownVTable, NC_E_ABI_VERSION, NC_E_INVALID_ARG, NC_E_NOINTERFACE, NC_E_OUT_OF_MEMORY,
    NC_E_OUT_OF_RANGE, NC_E_POINTER, NC_OK, NcInterfaceId, NcStatus,
};

use crate::__alloc::boxed::Box;
use crate::__alloc::string::String;
use crate::__alloc::vec::Vec;
use crate::object::{__com_add_ref_count, __com_release_ref_count};
use crate::{ComPtr, ComRef, ThreadSafe, ThreadingModel};

/// Default maximum allocation performed by an owned COM byte/string conversion.
///
/// [`INcBytesComPtrExt::to_vec`], [`INcBytesComPtrExt::to_i8_vec`],
/// [`INcStringComPtrExt::to_bytes`], and [`INcStringComPtrExt::to_string`]
/// reject larger reported lengths with [`NC_E_OUT_OF_RANGE`]. Use their
/// `_bounded` variants to select a different application-specific maximum, or
/// copy chunks into caller-owned storage with `copy_to`.
pub const NC_COM_DEFAULT_OWNED_COPY_MAX_BYTES: usize = 64 * 1024 * 1024;

/// Constructors for COM byte-sequence objects backed by Rust-owned vectors.
///
/// The returned COM object exposes [`INcBytesAbi`]. It owns its vector privately
/// and releases that allocation on final COM `Release`.
pub struct NcBytes;

/// Constructors for COM UTF-8 string objects backed by Rust-owned strings.
///
/// The returned COM object exposes [`INcStringAbi`] and can also be queried for
/// [`INcBytesAbi`] to inspect its UTF-8 bytes. [`INcStringAbi`] itself provides
/// bounded bulk copying; the secondary interface is not required for efficient
/// conversion.
pub struct NcString;

#[repr(C)]
/// Heap allocation backing an [`INcBytesAbi`] COM object.
///
/// The interface headers store a back-pointer to this allocation. Field order
/// is fixed for internal pointer recovery, although this structure is never
/// passed across the public ABI as a value.
struct BytesObject {
    /// Shared strong-reference count for every interface header.
    ref_count: AtomicU32,
    /// Canonical identity header returned for `IUnknown` queries.
    unknown: IUnknown,
    /// Byte-sequence interface header returned to callers.
    bytes: INcBytesAbi,
    /// Immutable bytes owned until the final COM release.
    value: Vec<u8>,
}

#[repr(C)]
/// Heap allocation backing a UTF-8 string and its byte-sequence projection.
struct StringObject {
    /// Shared strong-reference count for all three interface headers.
    ref_count: AtomicU32,
    /// Canonical identity header returned for `IUnknown` queries.
    unknown: IUnknown,
    /// Primary UTF-8 string interface header.
    string: INcStringAbi,
    /// Secondary byte-sequence header over the same UTF-8 storage.
    bytes: INcBytesAbi,
    /// Immutable valid UTF-8 storage owned until final release.
    value: String,
}

impl NcBytes {
    /// Wraps a `Vec<u8>` as a thread-local COM byte sequence.
    pub fn from_vec(value: Vec<u8>) -> ComPtr<INcBytesAbi> {
        // SAFETY: `BytesObject::allocate` returns a non-null owned bytes interface
        // pointer with initial refcount 1.
        unsafe { ComPtr::from_raw_owned(BytesObject::allocate(value)) }
            .expect("new COM bytes object produced a null pointer")
    }

    /// Wraps a `Vec<u8>` as a thread-safe COM byte sequence.
    pub fn from_vec_thread_safe(value: Vec<u8>) -> ComPtr<INcBytesAbi, ThreadSafe> {
        // SAFETY: `BytesObject` is immutable after construction, so calls
        // through its exposed methods are safe from any thread.
        unsafe { ComPtr::from_raw_owned(BytesObject::allocate(value)) }
            .expect("new COM bytes object produced a null pointer")
    }

    /// Copies a byte slice into a thread-local COM byte sequence.
    pub fn from_slice(value: &[u8]) -> ComPtr<INcBytesAbi> {
        Self::from_vec(value.to_vec())
    }

    /// Copies a byte slice into a thread-safe COM byte sequence.
    pub fn from_slice_thread_safe(value: &[u8]) -> ComPtr<INcBytesAbi, ThreadSafe> {
        Self::from_vec_thread_safe(value.to_vec())
    }

    /// Wraps a `Vec<i8>` as a bit-preserving COM byte sequence.
    pub fn from_i8_vec(value: Vec<i8>) -> ComPtr<INcBytesAbi> {
        Self::from_vec(i8_vec_into_bytes(value))
    }

    /// Wraps a `Vec<i8>` as a bit-preserving thread-safe COM byte sequence.
    pub fn from_i8_vec_thread_safe(value: Vec<i8>) -> ComPtr<INcBytesAbi, ThreadSafe> {
        Self::from_vec_thread_safe(i8_vec_into_bytes(value))
    }
}

impl NcString {
    /// Wraps a `String` as a thread-local COM UTF-8 string.
    pub fn from_string(value: String) -> ComPtr<INcStringAbi> {
        // SAFETY: `StringObject::allocate` returns a non-null owned string interface
        // pointer with initial refcount 1.
        unsafe { ComPtr::from_raw_owned(StringObject::allocate(value)) }
            .expect("new COM string object produced a null pointer")
    }

    /// Wraps a `String` as a thread-safe COM UTF-8 string.
    pub fn from_string_thread_safe(value: String) -> ComPtr<INcStringAbi, ThreadSafe> {
        // SAFETY: `StringObject` is immutable after construction, so calls
        // through its exposed methods are safe from any thread.
        unsafe { ComPtr::from_raw_owned(StringObject::allocate(value)) }
            .expect("new COM string object produced a null pointer")
    }

    /// Copies a string slice into a thread-local COM UTF-8 string.
    pub fn from_utf8_str(value: &str) -> ComPtr<INcStringAbi> {
        Self::from_string(String::from(value))
    }

    /// Copies a string slice into a thread-safe COM UTF-8 string.
    pub fn from_utf8_str_thread_safe(value: &str) -> ComPtr<INcStringAbi, ThreadSafe> {
        Self::from_string_thread_safe(String::from(value))
    }
}

/// Safe call helpers for owned and borrowed `INcBytesAbi` pointers.
pub trait INcBytesComPtrExt {
    /// Returns the byte length.
    fn len(&self) -> Result<u64, NcStatus>;

    /// Returns whether the byte sequence is empty.
    fn is_empty(&self) -> Result<bool, NcStatus>;

    /// Returns the byte at `index`.
    fn byte_at(&self, index: u64) -> Result<u8, NcStatus>;

    /// Copies bytes into `out` starting at `offset`.
    ///
    /// A failure leaves `out` unchanged.
    fn copy_to(&self, offset: u64, out: &mut [u8]) -> Result<(), NcStatus>;

    /// Copies the COM byte sequence into a Rust `Vec<u8>`.
    ///
    /// The default allocation limit is
    /// [`NC_COM_DEFAULT_OWNED_COPY_MAX_BYTES`]. Returns
    /// [`NC_E_OUT_OF_RANGE`] when the object reports a larger or
    /// Rust-vector-unrepresentable length and [`NC_E_OUT_OF_MEMORY`] when an
    /// in-range allocation fails.
    fn to_vec(&self) -> Result<Vec<u8>, NcStatus>;

    /// Copies at most `max_len` bytes into a Rust `Vec<u8>`.
    ///
    /// Returns [`NC_E_OUT_OF_RANGE`] before copying when the object reports a
    /// larger or Rust-vector-unrepresentable length and
    /// [`NC_E_OUT_OF_MEMORY`] when the allocation fails.
    fn to_vec_bounded(&self, max_len: usize) -> Result<Vec<u8>, NcStatus>;

    /// Copies the COM byte sequence into a bit-preserving Rust `Vec<i8>`.
    fn to_i8_vec(&self) -> Result<Vec<i8>, NcStatus>;

    /// Copies at most `max_len` bytes into a bit-preserving Rust `Vec<i8>`.
    fn to_i8_vec_bounded(&self, max_len: usize) -> Result<Vec<i8>, NcStatus>;
}

// Share the complete client lowering between owned and borrowed pointers.
// Each pointer type already establishes a live interface for the call lifetime.
macro_rules! impl_bytes_client {
    ($pointer:ty) => {
impl<M: ThreadingModel> INcBytesComPtrExt for $pointer {
    /// Calls the raw `len` slot and reads its initialized output on success.
    fn len(&self) -> Result<u64, NcStatus> {
        let mut out = MaybeUninit::<u64>::uninit();
        let this = self.as_mut_ptr();
        // SAFETY: `self` holds a live bytes interface reference and `out`
        // remains valid until the call returns.
        let status = unsafe { ((*(*this).vtable).len)(this, out.as_mut_ptr()) };
        if status.is_ok() {
            // SAFETY: A successful ABI call initializes the out slot.
            Ok(unsafe { out.assume_init() })
        } else {
            Err(status)
        }
    }

    /// Derives emptiness from the ABI-reported length.
    fn is_empty(&self) -> Result<bool, NcStatus> {
        Ok(self.len()? == 0)
    }

    /// Calls the raw indexed-byte slot and validates its output status.
    fn byte_at(&self, index: u64) -> Result<u8, NcStatus> {
        let mut out = MaybeUninit::<u8>::uninit();
        let this = self.as_mut_ptr();
        // SAFETY: `self` holds a live bytes interface reference and `out`
        // remains valid until the call returns.
        let status = unsafe { ((*(*this).vtable).byte_at)(this, index, out.as_mut_ptr()) };
        if status.is_ok() {
            // SAFETY: A successful ABI call initializes the out slot.
            Ok(unsafe { out.assume_init() })
        } else {
            Err(status)
        }
    }

    /// Calls the raw bulk-copy slot with a checked fixed-width slice length.
    fn copy_to(&self, offset: u64, out: &mut [u8]) -> Result<(), NcStatus> {
        let len = u64::try_from(out.len()).map_err(|_| NC_E_OUT_OF_RANGE)?;
        copy_to_raw(self.as_mut_ptr(), offset, len, out.as_mut_ptr())
    }

    /// Copies using the crate's default owned-allocation ceiling.
    fn to_vec(&self) -> Result<Vec<u8>, NcStatus> {
        self.to_vec_bounded(NC_COM_DEFAULT_OWNED_COPY_MAX_BYTES)
    }

    /// Allocates a bounded vector and fills it with one bulk ABI call.
    fn to_vec_bounded(&self, max_len: usize) -> Result<Vec<u8>, NcStatus> {
        let len = self.len()?;
        copy_owned_bytes(len, max_len, |out| copy_to_raw(self.as_mut_ptr(), 0, len, out))
    }

    /// Copies bytes and reinterprets their bits as signed eight-bit values.
    fn to_i8_vec(&self) -> Result<Vec<i8>, NcStatus> {
        self.to_i8_vec_bounded(NC_COM_DEFAULT_OWNED_COPY_MAX_BYTES)
    }

    /// Performs the bounded byte copy before changing element signedness.
    fn to_i8_vec_bounded(&self, max_len: usize) -> Result<Vec<i8>, NcStatus> {
        Ok(bytes_into_i8_vec(self.to_vec_bounded(max_len)?))
    }
}

    };
}

impl_bytes_client!(ComPtr<INcBytesAbi, M>);
impl_bytes_client!(ComRef<'_, INcBytesAbi, M>);

/// Safe call helpers for owned and borrowed `INcStringAbi` pointers.
pub trait INcStringComPtrExt {
    /// Returns the UTF-8 byte length.
    fn byte_len(&self) -> Result<u64, NcStatus>;

    /// Returns whether the string is empty.
    fn is_empty(&self) -> Result<bool, NcStatus>;

    /// Returns the UTF-8 byte at `index`.
    fn byte_at(&self, index: u64) -> Result<u8, NcStatus>;

    /// Copies UTF-8 bytes into `out` starting at `offset`.
    ///
    /// A failure leaves `out` unchanged. A copied subrange may split a UTF-8
    /// code point because offsets and lengths are measured in bytes.
    fn copy_to(&self, offset: u64, out: &mut [u8]) -> Result<(), NcStatus>;

    /// Copies the COM string into UTF-8 bytes.
    ///
    /// The default allocation limit is
    /// [`NC_COM_DEFAULT_OWNED_COPY_MAX_BYTES`]. Returns
    /// [`NC_E_OUT_OF_RANGE`] when the object reports a larger or
    /// Rust-vector-unrepresentable length and [`NC_E_OUT_OF_MEMORY`] when an
    /// in-range allocation fails.
    fn to_bytes(&self) -> Result<Vec<u8>, NcStatus>;

    /// Copies at most `max_len` UTF-8 bytes into a Rust vector.
    fn to_bytes_bounded(&self, max_len: usize) -> Result<Vec<u8>, NcStatus>;

    /// Copies the COM string into a Rust `String`.
    fn to_string(&self) -> Result<String, NcStatus>;

    /// Copies at most `max_len` UTF-8 bytes into a Rust `String`.
    fn to_string_bounded(&self, max_len: usize) -> Result<String, NcStatus>;
}

// Share the complete client lowering between owned and borrowed pointers.
// Each pointer type already establishes a live interface for the call lifetime.
macro_rules! impl_string_client {
    ($pointer:ty) => {
impl<M: ThreadingModel> INcStringComPtrExt for $pointer {
    /// Calls the raw byte-length slot and reads its output on success.
    fn byte_len(&self) -> Result<u64, NcStatus> {
        let mut out = MaybeUninit::<u64>::uninit();
        let this = self.as_mut_ptr();
        // SAFETY: `self` holds a live string interface reference and `out`
        // remains valid until the call returns.
        let status = unsafe { ((*(*this).vtable).byte_len)(this, out.as_mut_ptr()) };
        if status.is_ok() {
            // SAFETY: A successful ABI call initializes the out slot.
            Ok(unsafe { out.assume_init() })
        } else {
            Err(status)
        }
    }

    /// Derives emptiness from the ABI-reported UTF-8 byte length.
    fn is_empty(&self) -> Result<bool, NcStatus> {
        Ok(self.byte_len()? == 0)
    }

    /// Calls the raw indexed-byte slot without imposing character boundaries.
    fn byte_at(&self, index: u64) -> Result<u8, NcStatus> {
        let mut out = MaybeUninit::<u8>::uninit();
        let this = self.as_mut_ptr();
        // SAFETY: `self` holds a live string interface reference and `out`
        // remains valid until the call returns.
        let status = unsafe { ((*(*this).vtable).byte_at)(this, index, out.as_mut_ptr()) };
        if status.is_ok() {
            // SAFETY: A successful ABI call initializes the out slot.
            Ok(unsafe { out.assume_init() })
        } else {
            Err(status)
        }
    }

    /// Calls the string's raw bulk-copy slot with a checked slice length.
    fn copy_to(&self, offset: u64, out: &mut [u8]) -> Result<(), NcStatus> {
        let len = u64::try_from(out.len()).map_err(|_| NC_E_OUT_OF_RANGE)?;
        copy_string_to_raw(self.as_mut_ptr(), offset, len, out.as_mut_ptr())
    }

    /// Copies UTF-8 bytes using the default owned-allocation ceiling.
    fn to_bytes(&self) -> Result<Vec<u8>, NcStatus> {
        self.to_bytes_bounded(NC_COM_DEFAULT_OWNED_COPY_MAX_BYTES)
    }

    /// Allocates a bounded byte vector and fills it with one ABI call.
    fn to_bytes_bounded(&self, max_len: usize) -> Result<Vec<u8>, NcStatus> {
        let len = self.byte_len()?;
        copy_owned_bytes(len, max_len, |out| copy_string_to_raw(self.as_mut_ptr(), 0, len, out))
    }

    /// Copies and validates UTF-8 using the default allocation ceiling.
    fn to_string(&self) -> Result<String, NcStatus> {
        self.to_string_bounded(NC_COM_DEFAULT_OWNED_COPY_MAX_BYTES)
    }

    /// Copies at most `max_len` bytes and rejects invalid foreign UTF-8.
    fn to_string_bounded(&self, max_len: usize) -> Result<String, NcStatus> {
        String::from_utf8(self.to_bytes_bounded(max_len)?).map_err(|_| NC_E_INVALID_ARG)
    }
}

    };
}

impl_string_client!(ComPtr<INcStringAbi, M>);
impl_string_client!(ComRef<'_, INcStringAbi, M>);

/// Allocates an exactly sized byte vector and lets one ABI operation fill it.
///
/// The vector length is published only after `copy` succeeds, so failure drops
/// an empty vector and never exposes uninitialized spare capacity.
fn copy_owned_bytes(
    len: u64,
    max_len: usize,
    copy: impl FnOnce(*mut u8) -> Result<(), NcStatus>,
) -> Result<Vec<u8>, NcStatus> {
    let capacity = usize::try_from(len).map_err(|_| NC_E_OUT_OF_RANGE)?;
    if capacity > max_len || capacity > isize::MAX as usize {
        return Err(NC_E_OUT_OF_RANGE);
    }

    let mut out = Vec::new();
    out.try_reserve_exact(capacity)
        .map_err(|_| NC_E_OUT_OF_MEMORY)?;
    if capacity == 0 {
        return Ok(out);
    }

    copy(out.as_mut_ptr())?;
    // SAFETY: `copy` initialized exactly `capacity` bytes in the vector's
    // spare allocation before returning success.
    unsafe { out.set_len(capacity) };
    Ok(out)
}

/// Reuses a byte vector allocation while changing only its element signedness.
fn bytes_into_i8_vec(value: Vec<u8>) -> Vec<i8> {
    let mut value = ManuallyDrop::new(value);
    let (pointer, len, capacity) = (value.as_mut_ptr(), value.len(), value.capacity());
    // SAFETY: `u8` and `i8` have identical size and alignment. The allocation,
    // length, and capacity therefore remain valid after changing only the
    // element signedness, and `ManuallyDrop` prevents the source from freeing it.
    unsafe { Vec::from_raw_parts(pointer.cast::<i8>(), len, capacity) }
}

impl BytesObject {
    /// Allocates byte storage, installs its back-pointers, and returns ownership
    /// through the primary bytes interface with an initial count of one.
    fn allocate(value: Vec<u8>) -> *mut INcBytesAbi {
        let object = Box::new(Self {
            ref_count: AtomicU32::new(1),
            unknown: IUnknown {
                vtable: &BYTES_UNKNOWN_VTABLE,
                object: core::ptr::null_mut(),
            },
            bytes: INcBytesAbi {
                vtable: &BYTES_VTABLE,
                object: core::ptr::null_mut(),
            },
            value,
        });
        let raw = Box::into_raw(object);
        // SAFETY: `raw` was just allocated and is uniquely initialized before
        // any COM interface pointer is published.
        unsafe {
            (*raw).unknown.object = raw.cast::<c_void>();
            (*raw).bytes.object = raw.cast::<c_void>();
            core::ptr::addr_of_mut!((*raw).bytes)
        }
    }

    /// Validates and borrows byte storage from an interface's opaque pointer.
    ///
    /// # Safety
    ///
    /// A non-null `object` must be the back-pointer installed by
    /// [`Self::allocate`] and must remain live for the returned borrow.
    unsafe fn from_object<'a>(object: *mut c_void) -> Result<&'a Self, NcStatus> {
        let object = object.cast::<Self>();
        if object.is_null() {
            return Err(NC_E_POINTER);
        }
        // SAFETY: The caller supplied the back-pointer installed in this
        // object's interface headers.
        Ok(unsafe { &*object })
    }

    /// Increments this allocation's shared COM reference count.
    ///
    /// # Safety
    ///
    /// A non-null `object` must point to live [`BytesObject`] storage.
    unsafe fn add_ref(object: *mut c_void) -> u32 {
        if object.is_null() {
            return 0;
        }
        let object = object.cast::<Self>();
        // SAFETY: `object` is a live bytes storage pointer whose refcount stays
        // valid until final Release.
        unsafe { __com_add_ref_count(&(*object).ref_count) }
    }

    /// Decrements the shared count and frees the allocation at zero.
    ///
    /// # Safety
    ///
    /// A non-null `object` must represent one currently owned COM reference to
    /// live [`BytesObject`] storage.
    unsafe fn release(object: *mut c_void) -> u32 {
        if object.is_null() {
            return 0;
        }
        let object = object.cast::<Self>();
        // SAFETY: `object` is a live bytes storage pointer. A zero remaining
        // count grants this call exclusive final-drop rights.
        let Some(remaining) = (unsafe { __com_release_ref_count(&(*object).ref_count) }) else {
            return 0;
        };
        if remaining == 0 {
            // SAFETY: The allocation was originally leaked by `Box::into_raw`
            // in `allocate`; final Release rebuilds and drops it exactly once.
            unsafe { drop(Box::from_raw(object)) };
        }
        remaining
    }
}

impl StringObject {
    /// Allocates UTF-8 storage and initializes all interface back-pointers.
    fn allocate(value: String) -> *mut INcStringAbi {
        let object = Box::new(Self {
            ref_count: AtomicU32::new(1),
            unknown: IUnknown {
                vtable: &STRING_UNKNOWN_VTABLE,
                object: core::ptr::null_mut(),
            },
            string: INcStringAbi {
                vtable: &STRING_VTABLE,
                object: core::ptr::null_mut(),
            },
            bytes: INcBytesAbi {
                vtable: &STRING_BYTES_VTABLE,
                object: core::ptr::null_mut(),
            },
            value,
        });
        let raw = Box::into_raw(object);
        // SAFETY: `raw` was just allocated and is uniquely initialized before
        // any COM interface pointer is published.
        unsafe {
            (*raw).unknown.object = raw.cast::<c_void>();
            (*raw).string.object = raw.cast::<c_void>();
            (*raw).bytes.object = raw.cast::<c_void>();
            core::ptr::addr_of_mut!((*raw).string)
        }
    }

    /// Validates and borrows string storage from an interface's opaque pointer.
    ///
    /// # Safety
    ///
    /// A non-null `object` must be the back-pointer installed by
    /// [`Self::allocate`] and remain live for the returned borrow.
    unsafe fn from_object<'a>(object: *mut c_void) -> Result<&'a Self, NcStatus> {
        let object = object.cast::<Self>();
        if object.is_null() {
            return Err(NC_E_POINTER);
        }
        // SAFETY: The caller supplied the back-pointer installed in this
        // object's interface headers.
        Ok(unsafe { &*object })
    }

    /// Increments the reference count shared by all string projections.
    ///
    /// # Safety
    ///
    /// A non-null `object` must point to live [`StringObject`] storage.
    unsafe fn add_ref(object: *mut c_void) -> u32 {
        if object.is_null() {
            return 0;
        }
        let object = object.cast::<Self>();
        // SAFETY: `object` is a live string storage pointer whose refcount
        // stays valid until final Release.
        unsafe { __com_add_ref_count(&(*object).ref_count) }
    }

    /// Decrements the shared count and frees string storage at zero.
    ///
    /// # Safety
    ///
    /// A non-null `object` must represent one currently owned COM reference to
    /// live [`StringObject`] storage.
    unsafe fn release(object: *mut c_void) -> u32 {
        if object.is_null() {
            return 0;
        }
        let object = object.cast::<Self>();
        // SAFETY: `object` is a live string storage pointer. A zero remaining
        // count grants this call exclusive final-drop rights.
        let Some(remaining) = (unsafe { __com_release_ref_count(&(*object).ref_count) }) else {
            return 0;
        };
        if remaining == 0 {
            // SAFETY: The allocation was originally leaked by `Box::into_raw`
            // in `allocate`; final Release rebuilds and drops it exactly once.
            unsafe { drop(Box::from_raw(object)) };
        }
        remaining
    }
}

/// Standalone `IUnknown` dispatch table for byte objects.
static BYTES_UNKNOWN_VTABLE: IUnknownVTable = IUnknownVTable {
    interface_id: &IUnknown::INTERFACE_ID,
    query_interface: bytes_query_interface,
    add_ref: bytes_unknown_add_ref,
    release: bytes_unknown_release,
};

/// Primary byte-sequence dispatch table for byte objects.
static BYTES_VTABLE: INcBytesVTable = INcBytesVTable {
    unknown: IUnknownVTable {
        interface_id: &INcBytesAbi::INTERFACE_ID,
        query_interface: bytes_query_interface,
        add_ref: bytes_unknown_add_ref,
        release: bytes_unknown_release,
    },
    len: bytes_len,
    byte_at: bytes_byte_at,
    copy_to: bytes_copy_to,
};

/// Standalone `IUnknown` dispatch table for string objects.
static STRING_UNKNOWN_VTABLE: IUnknownVTable = IUnknownVTable {
    interface_id: &IUnknown::INTERFACE_ID,
    query_interface: string_query_interface,
    add_ref: string_unknown_add_ref,
    release: string_unknown_release,
};

/// Primary UTF-8 string dispatch table for string objects.
static STRING_VTABLE: INcStringVTable = INcStringVTable {
    unknown: IUnknownVTable {
        interface_id: &INcStringAbi::INTERFACE_ID,
        query_interface: string_query_interface,
        add_ref: string_unknown_add_ref,
        release: string_unknown_release,
    },
    byte_len: string_byte_len,
    byte_at: string_byte_at,
    copy_to: string_copy_to,
};

/// Byte-sequence projection dispatch table for string objects.
static STRING_BYTES_VTABLE: INcBytesVTable = INcBytesVTable {
    unknown: IUnknownVTable {
        interface_id: &INcBytesAbi::INTERFACE_ID,
        query_interface: string_query_interface,
        add_ref: string_unknown_add_ref,
        release: string_unknown_release,
    },
    len: string_bytes_len,
    byte_at: string_bytes_byte_at,
    copy_to: string_bytes_copy_to,
};

/// Implements ABI identity and interface negotiation for a byte object.
unsafe extern "C" fn bytes_query_interface(
    this: *mut IUnknown,
    interface_id: *const NcInterfaceId,
    out: *mut *mut c_void,
) -> NcStatus {
    if this.is_null() || interface_id.is_null() || out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: `out` was checked non-null and is valid for one pointer write by
    // the raw ABI contract.
    unsafe { *out = core::ptr::null_mut() };
    // SAFETY: `this` was checked non-null and is an IUnknown-compatible header.
    let object = unsafe { (*this).object };
    let storage = match unsafe { BytesObject::from_object(object) } {
        Ok(storage) => storage,
        Err(status) => return status,
    };
    // SAFETY: `interface_id` was checked non-null and is readable for this call.
    let interface_id = unsafe { &*interface_id };
    if interface_id.iid == IUnknown::IID {
        if *interface_id != IUnknown::INTERFACE_ID {
            return NC_E_ABI_VERSION;
        }
        // SAFETY: The storage pointer came from a live bytes object.
        unsafe { BytesObject::add_ref(object) };
        // SAFETY: `out` was checked non-null.
        unsafe {
            *out = core::ptr::addr_of!(storage.unknown)
                .cast_mut()
                .cast::<c_void>()
        };
        NC_OK
    } else if interface_id.iid == <INcBytesAbi as ComInterface>::IID {
        if *interface_id != <INcBytesAbi as ComInterface>::INTERFACE_ID {
            return NC_E_ABI_VERSION;
        }
        // SAFETY: The storage pointer came from a live bytes object.
        unsafe { BytesObject::add_ref(object) };
        // SAFETY: `out` was checked non-null.
        unsafe {
            *out = core::ptr::addr_of!(storage.bytes)
                .cast_mut()
                .cast::<c_void>()
        };
        NC_OK
    } else {
        NC_E_NOINTERFACE
    }
}

/// Forwards `IUnknown::AddRef` from a byte header to its shared storage.
unsafe extern "C" fn bytes_unknown_add_ref(this: *mut IUnknown) -> u32 {
    if this.is_null() {
        return 0;
    }
    // SAFETY: `this` was checked non-null and is an IUnknown-compatible header.
    unsafe { BytesObject::add_ref((*this).object) }
}

/// Forwards `IUnknown::Release` from a byte header to its shared storage.
unsafe extern "C" fn bytes_unknown_release(this: *mut IUnknown) -> u32 {
    if this.is_null() {
        return 0;
    }
    // SAFETY: `this` was checked non-null and is an IUnknown-compatible header.
    unsafe { BytesObject::release((*this).object) }
}

/// Writes the immutable byte vector's length to an ABI output slot.
unsafe extern "C" fn bytes_len(this: *mut INcBytesAbi, out: *mut u64) -> NcStatus {
    if this.is_null() || out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: `this` was checked non-null and is a bytes interface header.
    let object = unsafe { (*this).object };
    let storage = match unsafe { BytesObject::from_object(object) } {
        Ok(storage) => storage,
        Err(status) => return status,
    };
    // SAFETY: `out` was checked non-null and is valid for one `u64` write.
    unsafe { *out = storage.value.len() as u64 };
    NC_OK
}

/// Writes one checked element of a byte object to an ABI output slot.
unsafe extern "C" fn bytes_byte_at(this: *mut INcBytesAbi, index: u64, out: *mut u8) -> NcStatus {
    if this.is_null() || out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: `this` was checked non-null and is a bytes interface header.
    let object = unsafe { (*this).object };
    let storage = match unsafe { BytesObject::from_object(object) } {
        Ok(storage) => storage,
        Err(status) => return status,
    };
    let Ok(index) = usize::try_from(index) else {
        return NC_E_OUT_OF_RANGE;
    };
    let Some(byte) = storage.value.get(index).copied() else {
        return NC_E_OUT_OF_RANGE;
    };
    // SAFETY: `out` was checked non-null and is valid for one `u8` write.
    unsafe { *out = byte };
    NC_OK
}

/// Copies a checked byte-object range into caller-owned ABI memory.
unsafe extern "C" fn bytes_copy_to(
    this: *mut INcBytesAbi,
    offset: u64,
    len: u64,
    out: *mut u8,
) -> NcStatus {
    if this.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: `this` was checked non-null and is a bytes interface header.
    let object = unsafe { (*this).object };
    let storage = match unsafe { BytesObject::from_object(object) } {
        Ok(storage) => storage,
        Err(status) => return status,
    };
    copy_slice_to_raw(&storage.value, offset, len, out)
}

/// Negotiates the identity, string, and byte views of a string object.
unsafe extern "C" fn string_query_interface(
    this: *mut IUnknown,
    interface_id: *const NcInterfaceId,
    out: *mut *mut c_void,
) -> NcStatus {
    if this.is_null() || interface_id.is_null() || out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: `out` was checked non-null and is valid for one pointer write by
    // the raw ABI contract.
    unsafe { *out = core::ptr::null_mut() };
    // SAFETY: `this` was checked non-null and is an IUnknown-compatible header.
    let object = unsafe { (*this).object };
    let storage = match unsafe { StringObject::from_object(object) } {
        Ok(storage) => storage,
        Err(status) => return status,
    };
    // SAFETY: `interface_id` was checked non-null and is readable for this call.
    let interface_id = unsafe { &*interface_id };
    if interface_id.iid == IUnknown::IID {
        if *interface_id != IUnknown::INTERFACE_ID {
            return NC_E_ABI_VERSION;
        }
        // SAFETY: The storage pointer came from a live string object.
        unsafe { StringObject::add_ref(object) };
        // SAFETY: `out` was checked non-null.
        unsafe {
            *out = core::ptr::addr_of!(storage.unknown)
                .cast_mut()
                .cast::<c_void>()
        };
        NC_OK
    } else if interface_id.iid == <INcStringAbi as ComInterface>::IID {
        if *interface_id != <INcStringAbi as ComInterface>::INTERFACE_ID {
            return NC_E_ABI_VERSION;
        }
        // SAFETY: The storage pointer came from a live string object.
        unsafe { StringObject::add_ref(object) };
        // SAFETY: `out` was checked non-null.
        unsafe {
            *out = core::ptr::addr_of!(storage.string)
                .cast_mut()
                .cast::<c_void>()
        };
        NC_OK
    } else if interface_id.iid == <INcBytesAbi as ComInterface>::IID {
        if *interface_id != <INcBytesAbi as ComInterface>::INTERFACE_ID {
            return NC_E_ABI_VERSION;
        }
        // SAFETY: The storage pointer came from a live string object.
        unsafe { StringObject::add_ref(object) };
        // SAFETY: `out` was checked non-null.
        unsafe {
            *out = core::ptr::addr_of!(storage.bytes)
                .cast_mut()
                .cast::<c_void>()
        };
        NC_OK
    } else {
        NC_E_NOINTERFACE
    }
}

/// Forwards `IUnknown::AddRef` from a string header to shared storage.
unsafe extern "C" fn string_unknown_add_ref(this: *mut IUnknown) -> u32 {
    if this.is_null() {
        return 0;
    }
    // SAFETY: `this` was checked non-null and is an IUnknown-compatible header.
    unsafe { StringObject::add_ref((*this).object) }
}

/// Forwards `IUnknown::Release` from a string header to shared storage.
unsafe extern "C" fn string_unknown_release(this: *mut IUnknown) -> u32 {
    if this.is_null() {
        return 0;
    }
    // SAFETY: `this` was checked non-null and is an IUnknown-compatible header.
    unsafe { StringObject::release((*this).object) }
}

/// Writes the UTF-8 storage length through the primary string interface.
unsafe extern "C" fn string_byte_len(this: *mut INcStringAbi, out: *mut u64) -> NcStatus {
    if this.is_null() || out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: `this` was checked non-null and is a string interface header.
    let object = unsafe { (*this).object };
    let storage = match unsafe { StringObject::from_object(object) } {
        Ok(storage) => storage,
        Err(status) => return status,
    };
    // SAFETY: `out` was checked non-null and is valid for one `u64` write.
    unsafe { *out = storage.value.len() as u64 };
    NC_OK
}

/// Reads one UTF-8 byte through the primary string interface.
unsafe extern "C" fn string_byte_at(this: *mut INcStringAbi, index: u64, out: *mut u8) -> NcStatus {
    if this.is_null() || out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: `this` was checked non-null and is a string interface header.
    let object = unsafe { (*this).object };
    let storage = match unsafe { StringObject::from_object(object) } {
        Ok(storage) => storage,
        Err(status) => return status,
    };
    string_byte_from_storage(storage, index, out)
}

/// Copies a UTF-8 byte range through the primary string interface.
unsafe extern "C" fn string_copy_to(
    this: *mut INcStringAbi,
    offset: u64,
    len: u64,
    out: *mut u8,
) -> NcStatus {
    if this.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: `this` was checked non-null and is a string interface header.
    let object = unsafe { (*this).object };
    let storage = match unsafe { StringObject::from_object(object) } {
        Ok(storage) => storage,
        Err(status) => return status,
    };
    copy_slice_to_raw(storage.value.as_bytes(), offset, len, out)
}

/// Writes the UTF-8 storage length through its byte-sequence projection.
unsafe extern "C" fn string_bytes_len(this: *mut INcBytesAbi, out: *mut u64) -> NcStatus {
    if this.is_null() || out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: `this` was checked non-null and is a bytes interface header for a
    // string object.
    let object = unsafe { (*this).object };
    let storage = match unsafe { StringObject::from_object(object) } {
        Ok(storage) => storage,
        Err(status) => return status,
    };
    // SAFETY: `out` was checked non-null and is valid for one `u64` write.
    unsafe { *out = storage.value.len() as u64 };
    NC_OK
}

/// Reads one UTF-8 byte through the byte-sequence projection.
unsafe extern "C" fn string_bytes_byte_at(
    this: *mut INcBytesAbi,
    index: u64,
    out: *mut u8,
) -> NcStatus {
    if this.is_null() || out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: `this` was checked non-null and is a bytes interface header for a
    // string object.
    let object = unsafe { (*this).object };
    let storage = match unsafe { StringObject::from_object(object) } {
        Ok(storage) => storage,
        Err(status) => return status,
    };
    string_byte_from_storage(storage, index, out)
}

/// Copies UTF-8 bytes through the string object's byte-sequence projection.
unsafe extern "C" fn string_bytes_copy_to(
    this: *mut INcBytesAbi,
    offset: u64,
    len: u64,
    out: *mut u8,
) -> NcStatus {
    if this.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: `this` was checked non-null and is a bytes interface header for a
    // string object.
    let object = unsafe { (*this).object };
    let storage = match unsafe { StringObject::from_object(object) } {
        Ok(storage) => storage,
        Err(status) => return status,
    };
    copy_slice_to_raw(storage.value.as_bytes(), offset, len, out)
}

/// Implements the checked byte lookup shared by both string interfaces.
fn string_byte_from_storage(storage: &StringObject, index: u64, out: *mut u8) -> NcStatus {
    let Ok(index) = usize::try_from(index) else {
        return NC_E_OUT_OF_RANGE;
    };
    let Some(byte) = storage.value.as_bytes().get(index).copied() else {
        return NC_E_OUT_OF_RANGE;
    };
    // SAFETY: Callers check `out` for null before reaching this helper.
    unsafe { *out = byte };
    NC_OK
}

/// Calls a byte interface's bulk-copy slot and maps its status to `Result`.
fn copy_to_raw(
    this: *mut INcBytesAbi,
    offset: u64,
    len: u64,
    out: *mut u8,
) -> Result<(), NcStatus> {
    // SAFETY: The caller owns a live bytes interface and supplies an output
    // buffer valid for `len` writes, except when `len` is zero.
    let status = unsafe { ((*(*this).vtable).copy_to)(this, offset, len, out) };
    if status.is_ok() { Ok(()) } else { Err(status) }
}

/// Calls a string interface's bulk-copy slot and maps its status to `Result`.
fn copy_string_to_raw(
    this: *mut INcStringAbi,
    offset: u64,
    len: u64,
    out: *mut u8,
) -> Result<(), NcStatus> {
    // SAFETY: The caller owns a live string interface and supplies an output
    // buffer valid for `len` writes, except when `len` is zero.
    let status = unsafe { ((*(*this).vtable).copy_to)(this, offset, len, out) };
    if status.is_ok() { Ok(()) } else { Err(status) }
}

/// Validates an ABI range and copies it from an immutable Rust byte slice.
fn copy_slice_to_raw(value: &[u8], offset: u64, len: u64, out: *mut u8) -> NcStatus {
    let Ok(len) = usize::try_from(len) else {
        return NC_E_OUT_OF_RANGE;
    };
    if len != 0 && out.is_null() {
        return NC_E_POINTER;
    }
    let Ok(offset) = usize::try_from(offset) else {
        return NC_E_OUT_OF_RANGE;
    };
    let Some(end) = offset.checked_add(len) else {
        return NC_E_OUT_OF_RANGE;
    };
    let Some(bytes) = value.get(offset..end) else {
        return NC_E_OUT_OF_RANGE;
    };
    if bytes.is_empty() {
        return NC_OK;
    }
    // SAFETY: `bytes` is in-bounds for `len` reads and the ABI contract
    // requires non-null `out` to be valid for `len` writes and non-overlapping
    // with the source range.
    unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len()) };
    NC_OK
}

/// Reuses a signed-byte vector allocation as an unsigned byte vector.
fn i8_vec_into_bytes(value: Vec<i8>) -> Vec<u8> {
    let mut value = ManuallyDrop::new(value);
    let (pointer, len, capacity) = (value.as_mut_ptr(), value.len(), value.capacity());
    // SAFETY: `i8` and `u8` have identical size and alignment. The allocation,
    // length, and capacity therefore remain valid after changing only the
    // element signedness, and `ManuallyDrop` prevents the source from freeing it.
    unsafe { Vec::from_raw_parts(pointer.cast::<u8>(), len, capacity) }
}
