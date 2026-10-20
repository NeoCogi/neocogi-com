//! One-way copied output: provider writers initialize raw slots, callers acquire results.
//!
//! Rust containers and writer layouts stay local to each library. Only an element
//! pointer, capacity, logical offset, and initialized-prefix count cross the ABI.
//! Old destination contents are never input. Restricted values remain raw until
//! every result has passed its recursive codec, and failed calls expose no result.

use core::{marker::PhantomData, mem::MaybeUninit};
use crate::{ComInlineValue, NcStatus, NC_E_OUT_OF_RANGE, NC_E_POINTER};
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Call-scoped provider view of fresh copied output, never readable native values.
///
/// The constructor admits the whole capacity before implementation entry. A writer
/// cannot allocate, retain caller storage, read old elements, or expose a native
/// reference into raw wire slots. Dropping it does not publish the prefix: the
/// generated thunk publishes counts only after the implementation succeeds.
pub struct NcOutputWriter<'a, T: ComInlineValue> {
    /// Method-defined source offset; independent of the raw destination address.
    offset: u64,
    /// Caller-owned storage; inactive payload and padding bytes need not be initialized.
    slots: &'a mut [MaybeUninit<T::Wire>],
    /// Thunk-local count, borrowed so a moved writer can report completed writes.
    written: &'a mut u64,
    /// Keeps this view invariant in its native element without owning such elements.
    native: PhantomData<T>,
}

impl<'a, T: ComInlineValue> NcOutputWriter<'a, T> {
    /// Checks output capacity and pointer shape without reading destination bytes.
    ///
    /// # Safety
    /// Nonempty storage is aligned, exclusively writable for `capacity` wire
    /// elements, and live for `'a`. The count is distinct from that storage.
    /// The caller must not read slots until successful count publication. Neither
    /// the elements nor their restricted fields need to be initialized on entry.
    #[doc(hidden)]
    pub unsafe fn from_raw(offset: u64, pointer: *mut T::Wire, capacity: u64, written: &'a mut u64) -> Result<Self, NcStatus> {
        crate::check_buffer_output::<T>(capacity)?;
        let count = usize::try_from(capacity).map_err(|_| NC_E_OUT_OF_RANGE)?;
        let pointer = match count {
            0 => core::ptr::NonNull::<MaybeUninit<T::Wire>>::dangling().as_ptr(),
            _ if pointer.is_null() || pointer.align_offset(core::mem::align_of::<T::Wire>()) != 0 => return Err(NC_E_POINTER),
            _ => pointer.cast(),
        };
        *written = 0;
        // SAFETY: shape is checked; the ABI caller grants the exclusive region
        // for this call. MaybeUninit introduces no initialization assumption.
        let slots = unsafe { core::slice::from_raw_parts_mut(pointer, count) };
        Ok(Self {
            offset,
            slots,
            written,
            native: PhantomData,
        })
    }

    /// Returns the source offset interpreted by the particular getter.
    pub fn offset(&self) -> u64 {
        self.offset
    }
    /// Returns the number of available result slots, including already written ones.
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }
    /// Returns the initialized prefix length; this count is not yet ABI publication.
    pub fn written(&self) -> usize {
        *self.written as usize
    }
    /// Returns the remaining slots available for additional writes.
    pub fn remaining(&self) -> usize {
        self.capacity() - self.written()
    }
    /// Returns true when the getter was supplied no output capacity.
    pub fn is_empty(&self) -> bool {
        self.capacity() == 0
    }

    /// Lends this untouched destination to another synchronous COM provider.
    /// Forwarding uses the same raw slots without scratch allocation. The inner
    /// client validates its reported prefix before updating this thunk's count.
    /// A partially filled writer cannot be forwarded as a fresh whole output.
    pub fn forward(self) -> Result<NcOutput<'a, T>, NcStatus> {
        match *self.written {
            0 => {
                let capacity = self.slots.len();
                Ok(NcOutput {
                    target: OutputTarget::Forward { slots: self.slots, count: self.written },
                    offset: self.offset,
                    capacity,
                })
            }
            _ => Err(crate::NC_E_INVALID_ARG),
        }
    }

    /// Encodes one native value directly into its next raw destination slot.
    pub fn push(&mut self, value: T) -> Result<(), NcStatus> {
        if T::BORROW_WIRE {
            // Identity admission requires no conversion budget. Individual and
            // bulk writes must use exactly the same direct-copy rule afterward.
            return self.extend_from_slice(core::slice::from_ref(&value));
        }
        let index = self.written();
        let slot = self.slots.get_mut(index).ok_or(NC_E_OUT_OF_RANGE)?;
        // Capacity admission prepaid every possible inline alternative. Content
        // still follows the same recursive typed codec as ordinary COM arguments.
        // SAFETY: ComInlineValue supplies self-contained Copy storage and its
        // MAX_COST was prepaid for every slot before implementation entry.
        let wire = unsafe { value.encode_prechecked() }?.0;
        slot.write(wire);
        *self.written += 1;
        Ok(())
    }

    /// Appends a native slice, retaining bulk-copy speed for wire-identical types.
    /// Insufficient space is rejected before any element from this slice is written.
    pub fn extend_from_slice(&mut self, values: &[T]) -> Result<(), NcStatus> {
        if values.len() > self.remaining() {
            return Err(NC_E_OUT_OF_RANGE);
        }
        match T::BORROW_WIRE {
            true => {
                // SAFETY: the codec proves identical layout, unrestricted bits,
                // and identity conversion. Source and exclusive destination do
                // not overlap; the checked prefix has enough writable capacity.
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        values.as_ptr().cast::<T::Wire>(),
                        self.slots.as_mut_ptr().add(self.written()).cast(),
                        values.len(),
                    );
                }
                *self.written += values.len() as u64;
                Ok(())
            }
            false => {
                for &value in values {
                    self.push(value)?;
                }
                Ok(())
            }
        }
    }
}

/// Caller-owned copied results with reusable native and, when needed, wire capacity.
///
/// Numeric wire-identical results allocate only native storage. Converted results
/// retain a second raw vector; it contains no resource capabilities or provider
/// allocations. Each call discards its previous results. Applications requiring
/// old/new rollback keep their old snapshot separately until the new call succeeds.
#[cfg(feature = "alloc")]
pub struct NcOutputBuffer<T: ComInlineValue> {
    /// Only fully acquired, published native results may be observed through this field.
    values: Vec<T>,
    /// Converted slots have capacity but no readable element length outside a call.
    wires: Vec<MaybeUninit<T::Wire>>,
}

#[cfg(feature = "alloc")]
impl<T: ComInlineValue> NcOutputBuffer<T> {
    /// Creates an empty result buffer without allocating.
    pub fn new() -> Self {
        Self { values: Vec::new(), wires: Vec::new() }
    }
    /// Reuses an application's native allocation while discarding its old values.
    pub fn from_vec(mut values: Vec<T>) -> Self {
        values.clear();
        Self { values, wires: Vec::new() }
    }
    /// Borrows only the results of a completely successful call.
    pub fn as_slice(&self) -> &[T] {
        &self.values
    }
    /// Returns the number of published native results.
    pub fn len(&self) -> usize {
        self.values.len()
    }
    /// Returns retained native capacity; a call still uses its explicit requested capacity.
    /// Converted wire storage may have a separately rounded allocation.
    pub fn capacity(&self) -> usize {
        self.values.capacity()
    }
    /// Returns true when there are no published results.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    /// Discards published results without releasing reusable capacity.
    pub fn clear(&mut self) {
        self.values.clear();
    }
    /// Transfers the native results; any separate wire capacity is released locally.
    pub fn into_vec(self) -> Vec<T> {
        self.values
    }
    /// Borrows this buffer for one getter with explicit source offset and capacity.
    /// Storage and limit checks occur inside the generated client before owners
    /// are lent to the provider. Constructing this argument does not allocate.
    pub fn output(&mut self, offset: u64, capacity: usize) -> NcOutput<'_, T> {
        NcOutput {
            target: OutputTarget::Buffer(self),
            offset,
            capacity,
        }
    }
}

#[cfg(feature = "alloc")]
impl<T: ComInlineValue> Default for NcOutputBuffer<T> {
    /// Default buffers have no allocation or published results.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "alloc")]
impl<T: ComInlineValue> core::ops::Deref for NcOutputBuffer<T> {
    /// Read-only access never exposes raw slots or an incomplete native acquisition.
    type Target = [T];
    /// Lends the published slice using ordinary collection operations.
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

/// Exclusive client argument for one output-only getter; no Rust layout crosses COM.
pub struct NcOutput<'a, T: ComInlineValue> {
    /// Keeps caller storage exclusively borrowed through validation and publication.
    target: OutputTarget<'a, T>,
    /// Logical source offset passed unchanged to the provider.
    offset: u64,
    /// Requested slots, independent of the allocator's possibly larger capacity.
    capacity: usize,
}

/// Caller storage alternatives share one output-only protocol, without erased types.
enum OutputTarget<'a, T: ComInlineValue> {
    /// Reusable allocation-backed results, including restricted native/wire conversion.
    #[cfg(feature = "alloc")]
    Buffer(&'a mut NcOutputBuffer<T>),
    /// Allocation-free native slots, permitted only with the identity-wire proof.
    Direct {
        /// Caller retains uninitialized storage; no native slice is manufactured.
        slots: &'a mut [MaybeUninit<T>],
        /// Published result length, kept at zero until complete call success.
        count: &'a mut usize,
    },
    /// Provider-to-provider lending uses already admitted wire slots and a local count.
    Forward {
        /// No old contents are read; the nested call produces a fresh prefix.
        slots: &'a mut [MaybeUninit<T::Wire>],
        /// Outer thunk's private count, published only when the inner call completes.
        count: &'a mut u64,
    },
}

/// Local completion state: an unwritten or invalid result is never published.
enum Publication {
    /// No complete native acquisition exists; failure may have produced a private prefix.
    Unacquired,
    /// All local elements validated; Drop still clears them if another output fails.
    Acquired,
    /// Every output in the call has validated and may now remain visible.
    Published,
}

/// Prepared client output, with all fallible allocation before provider entry.
#[doc(hidden)]
pub struct PreparedOutput<'a, T: ComInlineValue> {
    /// Exclusive output argument and its retained capacity.
    output: NcOutput<'a, T>,
    /// Provider-reported prefix, checked before any raw slot is read.
    written: u64,
    /// Controls whether unwinding/rejection discards native acquisition.
    publication: Publication,
}

impl<'a, T: ComInlineValue> NcOutput<'a, T> {
    /// Borrows allocation-free slots for values with proven identity wire layout.
    /// Restricted booleans/enums require the allocation-backed output buffer.
    /// The supplied count publishes only after the whole generated call succeeds.
    pub fn from_slots(offset: u64, slots: &'a mut [MaybeUninit<T>], count: &'a mut usize) -> Result<Self, NcStatus> {
        match T::BORROW_WIRE {
            false => Err(crate::NC_E_INVALID_ARG),
            true => {
                *count = 0;
                let capacity = slots.len();
                Ok(Self {
                    target: OutputTarget::Direct { slots, count },
                    offset,
                    capacity,
                })
            }
        }
    }
    /// Discards old results, admits every possible output, and reserves storage.
    #[doc(hidden)]
    pub fn prepare(mut self) -> Result<PreparedOutput<'a, T>, NcStatus> {
        self.clear_results();
        crate::check_buffer_output::<T>(u64::try_from(self.capacity).map_err(|_| NC_E_OUT_OF_RANGE)?)?;
        match &mut self.target {
            #[cfg(feature = "alloc")]
            OutputTarget::Buffer(buffer) => {
                // Native length is zero; reserve keeps earlier reusable capacity.
                buffer.values.try_reserve_exact(self.capacity).map_err(|_| crate::NC_E_OUT_OF_MEMORY)?;
                if !T::BORROW_WIRE {
                    buffer.wires.try_reserve_exact(self.capacity).map_err(|_| crate::NC_E_OUT_OF_MEMORY)?;
                }
            }
            OutputTarget::Direct { .. } | OutputTarget::Forward { .. } => {}
        }
        Ok(PreparedOutput {
            output: self,
            written: 0,
            publication: Publication::Unacquired,
        })
    }
    /// Makes failed/private results unavailable while retaining their storage.
    fn clear_results(&mut self) {
        match &mut self.target {
            #[cfg(feature = "alloc")]
            OutputTarget::Buffer(buffer) => buffer.values.clear(),
            OutputTarget::Direct { count, .. } => **count = 0,
            OutputTarget::Forward { count, .. } => **count = 0,
        }
    }
}

impl<T: ComInlineValue> PreparedOutput<'_, T> {
    /// Preserves the getter's logical source offset.
    pub fn offset(&self) -> u64 {
        self.output.offset
    }
    /// Returns the requested ABI capacity, never the allocator's rounded capacity.
    pub fn capacity(&self) -> u64 {
        self.output.capacity as u64
    }
    /// Borrows raw output storage without creating initialized native elements.
    pub fn as_mut_ptr(&mut self) -> *mut T::Wire {
        match &mut self.output.target {
            #[cfg(feature = "alloc")]
            OutputTarget::Buffer(buffer) => match T::BORROW_WIRE {
                true => buffer.values.as_mut_ptr().cast(),
                false => buffer.wires.as_mut_ptr().cast(),
            },
            OutputTarget::Direct { slots, .. } => slots.as_mut_ptr().cast(),
            OutputTarget::Forward { slots, .. } => slots.as_mut_ptr().cast(),
        }
    }
    /// Supplies the independent prefix-count slot to the synchronous raw call.
    pub fn written_ptr(&mut self) -> *mut u64 {
        &mut self.written
    }
    /// Acquires initialized results privately; no allocation or publication occurs.
    ///
    /// # Safety
    /// A successful raw provider initialized every active field in its reported
    /// prefix and kept this caller's storage valid. Restricted contents may be
    /// malformed: the codec validates them before constructing any native value.
    pub unsafe fn convert_output(&mut self) -> Result<(), NcStatus> {
        match self.publication {
            Publication::Unacquired => self.output.clear_results(),
            Publication::Acquired | Publication::Published => return Err(crate::NC_E_INVALID_ARG),
        }
        let count = usize::try_from(self.written).map_err(|_| NC_E_OUT_OF_RANGE)?;
        if count > self.output.capacity {
            return Err(NC_E_OUT_OF_RANGE);
        }
        match &mut self.output.target {
            #[cfg(feature = "alloc")]
            OutputTarget::Buffer(buffer) => match T::BORROW_WIRE {
                true => {
                    // SAFETY: the provider initialized the checked prefix, and
                    // the codec proves unrestricted bits and identical layout.
                    unsafe {
                        buffer.values.set_len(count);
                    }
                }
                false => {
                    for index in 0..count {
                        // SAFETY: read only initialized active fields of the
                        // reported prefix, after whole-capacity admission.
                        let wire = unsafe { buffer.wires.as_ptr().add(index).read().assume_init() };
                        let value = unsafe { T::decode_prechecked(wire) }?;
                        buffer.values.push(value);
                    }
                }
            },
            // Identity-only slots need no conversion. Count publication still
            // waits until every output in the generated call has validated.
            OutputTarget::Direct { .. } => {}
            OutputTarget::Forward { slots, .. } => {
                if !T::BORROW_WIRE {
                    for slot in &slots[..count] {
                        // SAFETY: successful nested provider initialized active
                        // fields in the checked prefix. No native storage escapes.
                        let wire = unsafe { slot.assume_init_read() };
                        unsafe { T::decode_prechecked(wire) }?;
                    }
                }
            }
        }
        self.publication = Publication::Acquired;
        Ok(())
    }
    /// Publishes this acquisition only after the generated client validates all outputs.
    ///
    /// # Panics
    /// Publication requires a complete successful acquisition, never a partial prefix.
    pub fn publish(mut self) {
        match self.publication {
            Publication::Acquired => {
                match &mut self.output.target {
                    OutputTarget::Direct { count, .. } => **count = self.written as usize,
                    OutputTarget::Forward { count, .. } => **count = self.written,
                    #[cfg(feature = "alloc")]
                    OutputTarget::Buffer(_) => {}
                }
                self.publication = Publication::Published;
            }
            Publication::Unacquired | Publication::Published => panic!("output was not completely acquired"),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Stack output proves the exact protocol also works with alloc disabled.
    use super::*;

    #[test]
    fn stack_numeric_output_publishes_only_after_acquisition() {
        let mut slots = [MaybeUninit::<u32>::uninit(); 2];
        let mut count = 0;
        let output = NcOutput::from_slots(3, &mut slots, &mut count).unwrap();
        let mut prepared = output.prepare().unwrap();
        let mut written = 0;
        // SAFETY: prepared owns the two stack slots, separate from the local count.
        let mut writer = unsafe { NcOutputWriter::<u32>::from_raw(3, prepared.as_mut_ptr(), 2, &mut written) }.unwrap();
        writer.extend_from_slice(&[10, 20]).unwrap();
        drop(writer);
        unsafe {
            *prepared.written_ptr() = written;
            prepared.convert_output().unwrap();
        }
        prepared.publish();
        assert_eq!(count, 2);
        // SAFETY: complete successful acquisition published exactly these slots.
        assert_eq!(unsafe { slots.map(|slot| slot.assume_init()) }, [10, 20]);
    }
}

impl<T: ComInlineValue> Drop for PreparedOutput<'_, T> {
    /// Every rejected, malformed, or unwinding call makes its complete output unavailable.
    fn drop(&mut self) {
        match self.publication {
            Publication::Unacquired | Publication::Acquired => self.output.clear_results(),
            Publication::Published => {}
        }
    }
}
