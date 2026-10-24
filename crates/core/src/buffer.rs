//! Borrowed native-value views for readable interface arguments.
//!
//! This view groups a logical offset with an already-valid shared slice. Generated
//! clients retain direct or converted wire storage in [`crate::EncodedInput`];
//! providers use [`crate::DecodedInput`] to validate raw input before lending its
//! native values to an implementation. The guard remains alive throughout the call.
//! Output uses the separate `NcOutput` and `NcOutputWriter` types.

/// Call-scoped readable buffer parameter with an explicit source offset.
///
/// `NcCopyFromBufferOffset<'a, T>` is the safe Rust parameter form for the
/// typed `CopyFromBufferOffset<T>` passing mode. Generated raw methods receive
/// an offset, a count and a pointer to `T::Wire`, whose representation may differ
/// from these native elements. Checked input guards own that conversion.
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

    /// Returns the complete readable slice without applying the logical offset.
    ///
    /// For `T = u8`, `core::str::from_utf8(self.as_slice())` validates and borrows
    /// text without allocation. The buffer itself permits arbitrary bytes.
    pub const fn as_slice(&self) -> &'a [T] {
        self.data
    }

    /// Returns a pointer to the borrowed native elements.
    ///
    /// This is not necessarily an ABI wire pointer. Generated clients obtain
    /// their wire pointer from [`crate::EncodedInput`] instead.
    pub const fn as_ptr(&self) -> *const T {
        self.data.as_ptr()
    }
}

impl<T> Copy for NcCopyFromBufferOffset<'_, T> {}

impl<T> Clone for NcCopyFromBufferOffset<'_, T> {
    /// Copies the shared slice and offset pair without copying its elements.
    fn clone(&self) -> Self {
        *self
    }
}
