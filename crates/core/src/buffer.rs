//! Borrowed buffer arguments used by generated interface methods.
//!
//! The ABI represents a buffer operation as three independent values: a
//! logical offset, an element count, and a pointer. This module groups those
//! values into call-scoped Rust types so generated safe methods can accept
//! slices while their raw thunks still expose the stable C representation.
//! Constructors used by generated code validate pointer/count combinations and
//! integer conversions before creating references. The writable form remains
//! non-`Copy` so safe Rust cannot duplicate its exclusive slice.

use core::slice;

use crate::{NC_E_OUT_OF_RANGE, NC_E_POINTER, NcStatus};

/// Call-scoped readable buffer parameter with an explicit source offset.
///
/// `NcCopyFromBufferOffset<'a, T>` is the safe Rust parameter form for the
/// typed `CopyFromBufferOffset<T>` ABI passing mode. The generated raw vtable
/// lowers it to `(offset: u64, len: u64, ptr: *const T)`.
pub struct NcCopyFromBufferOffset<'a, T> {
    /// Logical source offset interpreted by the receiving interface method.
    offset: u64,
    /// Elements made readable for the duration of the ABI call.
    data: &'a [T],
}

impl<'a, T> NcCopyFromBufferOffset<'a, T> {
    /// Creates a readable buffer argument.
    pub const fn new(offset: u64, data: &'a [T]) -> Self {
        Self { offset, data }
    }

    /// Returns the logical buffer offset.
    pub const fn offset(&self) -> u64 {
        self.offset
    }

    /// Returns the number of elements in the buffer.
    pub const fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns whether the buffer is empty.
    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the readable slice.
    pub const fn as_slice(&self) -> &'a [T] {
        self.data
    }

    /// Returns the raw readable ABI pointer.
    pub const fn as_ptr(&self) -> *const T {
        self.data.as_ptr()
    }

    #[doc(hidden)]
    /// Converts the slice length to the fixed-width ABI count type.
    pub fn __nc_len_u64(&self) -> Result<u64, NcStatus> {
        u64::try_from(self.data.len()).map_err(|_| NC_E_OUT_OF_RANGE)
    }

    #[doc(hidden)]
    /// Reconstructs a checked borrowed input buffer from its ABI components.
    ///
    /// # Safety
    ///
    /// When `len` is nonzero, `ptr` must be valid for `len` readable `T`
    /// elements for the duration of the generated ABI call.
    pub unsafe fn __nc_from_raw_parts(offset: u64, ptr: *const T, len: u64) -> Result<Self, NcStatus> {
        let len = usize::try_from(len).map_err(|_| NC_E_OUT_OF_RANGE)?;
        if len != 0 && ptr.is_null() {
            return Err(NC_E_POINTER);
        }
        let data = if len == 0 {
            &[]
        } else {
            // SAFETY: The raw ABI caller guarantees that `ptr` is valid for
            // `len` readable elements for the duration of the call.
            unsafe { slice::from_raw_parts(ptr, len) }
        };
        Ok(Self { offset, data })
    }
}

impl<T> Copy for NcCopyFromBufferOffset<'_, T> {}

impl<T> Clone for NcCopyFromBufferOffset<'_, T> {
    /// Copies the shared slice and offset pair without copying its elements.
    fn clone(&self) -> Self {
        *self
    }
}

/// Call-scoped writable buffer parameter with an explicit destination offset.
///
/// `NcCopyToBufferOffset<'a, T>` is the safe Rust parameter form for the typed
/// `CopyToBufferOffset<T>` ABI passing mode. The generated raw vtable lowers it
/// to `(offset: u64, len: u64, ptr: *mut T)`.
pub struct NcCopyToBufferOffset<'a, T> {
    /// Logical destination offset interpreted by the receiving method.
    offset: u64,
    /// Elements made exclusively writable for the duration of the ABI call.
    data: &'a mut [T],
}

impl<'a, T> NcCopyToBufferOffset<'a, T> {
    /// Creates a writable buffer argument.
    pub fn new(offset: u64, data: &'a mut [T]) -> Self {
        Self { offset, data }
    }

    /// Returns the logical buffer offset.
    pub const fn offset(&self) -> u64 {
        self.offset
    }

    /// Returns the number of elements in the buffer.
    pub const fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns whether the buffer is empty.
    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the writable slice.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.data
    }

    /// Consumes this buffer parameter and returns the writable slice.
    pub fn into_mut_slice(self) -> &'a mut [T] {
        self.data
    }

    /// Returns the raw writable ABI pointer.
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.data.as_mut_ptr()
    }

    #[doc(hidden)]
    /// Converts the slice length to the fixed-width ABI count type.
    pub fn __nc_len_u64(&self) -> Result<u64, NcStatus> {
        u64::try_from(self.data.len()).map_err(|_| NC_E_OUT_OF_RANGE)
    }

    #[doc(hidden)]
    /// Returns the pointer component passed to the generated raw ABI method.
    pub fn __nc_mut_ptr(&mut self) -> *mut T {
        self.data.as_mut_ptr()
    }

    #[doc(hidden)]
    /// Reconstructs a checked borrowed output buffer from its ABI components.
    ///
    /// # Safety
    ///
    /// When `len` is nonzero, `ptr` must be valid for `len` writable `T`
    /// elements for the duration of the generated ABI call.
    pub unsafe fn __nc_from_raw_parts(offset: u64, ptr: *mut T, len: u64) -> Result<Self, NcStatus> {
        let len = usize::try_from(len).map_err(|_| NC_E_OUT_OF_RANGE)?;
        if len != 0 && ptr.is_null() {
            return Err(NC_E_POINTER);
        }
        let data = if len == 0 {
            // SAFETY: A zero-length slice does not dereference the dangling
            // pointer. The lifetime is call-scoped by the generated thunk.
            unsafe { slice::from_raw_parts_mut(core::ptr::NonNull::<T>::dangling().as_ptr(), 0) }
        } else {
            // SAFETY: The raw ABI caller guarantees that `ptr` is valid for
            // `len` writable elements for the duration of the call.
            unsafe { slice::from_raw_parts_mut(ptr, len) }
        };
        Ok(Self { offset, data })
    }
}
