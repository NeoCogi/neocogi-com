//! Fallible, allocation-free construction in an array's actual element layout.
use core::{
    mem::{self, MaybeUninit},
    ptr, slice,
};
use crate::NcStatus;

/// Constructs each element once, handing complete ownership to the returned array.
/// Only the initialized prefix is destroyed on error or ordinary panic unwinding.
pub(crate) fn try_from_fn<T, const N: usize>(make: impl FnMut(usize) -> Result<T, NcStatus>) -> Result<[T; N], NcStatus> {
    let mut output = MaybeUninit::<[T; N]>::uninit();
    // SAFETY: an array is contiguous T storage; MaybeUninit preserves size and
    // alignment without requiring any initialized T. The typed destination also
    // supplies a non-null aligned pointer for empty arrays and aligned ZSTs.
    let slots = unsafe { slice::from_raw_parts_mut(output.as_mut_ptr().cast::<MaybeUninit<T>>(), N) };
    fill(slots, make)?;
    // SAFETY: fill succeeded only after all N elements were written and its
    // prefix owner was disarmed. There is no fallible work in this handoff.
    Ok(unsafe { output.assume_init() })
}

/// Borrows construction storage so the loop/guard need not own another array.
fn fill<T>(slots: &mut [MaybeUninit<T>], mut make: impl FnMut(usize) -> Result<T, NcStatus>) -> Result<(), NcStatus> {
    let mut prefix = Prefix { slots, initialized: 0 };
    while prefix.initialized < prefix.slots.len() {
        let value = make(prefix.initialized)?;
        prefix.slots[prefix.initialized].write(value);
        // The preceding bounds check proves this increment cannot overflow.
        // No user or fallible operation separates writing from owning the slot.
        prefix.initialized += 1;
    }
    // All elements now belong to the complete destination, not this guard.
    mem::forget(prefix);
    Ok(())
}

/// Owns exactly slots[0..initialized]; unwritten slots never contain a borrowed T.
struct Prefix<'a, T> {
    /// Live storage at T's alignment, borrowed until handoff or prefix destruction.
    slots: &'a mut [MaybeUninit<T>],
    /// Logical owner count, including ZSTs whose addresses may all be equal.
    initialized: usize,
}
impl<T> Drop for Prefix<'_, T> {
    /// Slice drop glue also cleans remaining elements if one destructor panics.
    fn drop(&mut self) {
        // SAFETY: only this prefix contains owned initialized values. The view
        // has the original allocation/alignment even when its length/size is zero.
        unsafe {
            ptr::drop_in_place(ptr::slice_from_raw_parts_mut(self.slots.as_mut_ptr().cast::<T>(), self.initialized));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::try_from_fn;
    use crate::NC_E_INVALID_ARG;
    use std::{cell::Cell, vec::Vec};

    /// Non-Copy owner with independently observable per-element destruction.
    struct Tracked<'a> {
        /// Owner identity does not depend on its address or move count.
        index: usize,
        /// Outlives all values created in the current test.
        drops: &'a [Cell<usize>],
        /// Optional single destructor panic, after recording its own destruction.
        panic_at: Option<usize>,
    }
    impl Drop for Tracked<'_> {
        fn drop(&mut self) {
            self.drops[self.index].set(self.drops[self.index].get() + 1);
            assert_ne!(self.panic_at, Some(self.index), "injected destructor panic");
        }
    }

    #[test]
    fn errors_drop_each_returned_owner_once_and_success_transfers_ownership() {
        for fail in 0..=5 {
            let drops: [Cell<usize>; 5] = core::array::from_fn(|_| Cell::new(0));
            let mut calls = Vec::new();
            let result = try_from_fn::<_, 5>(|index| {
                calls.push(index);
                if index == fail {
                    return Err(NC_E_INVALID_ARG);
                }
                Ok(Tracked { index, drops: &drops, panic_at: None })
            });
            assert_eq!(result.is_ok(), fail == 5);
            assert_eq!(calls, (0..(fail + 1).min(5)).collect::<Vec<_>>());
            if result.is_ok() {
                assert!(drops.iter().all(|count| count.get() == 0));
            }
            drop(result);
            for (index, count) in drops.iter().enumerate() {
                assert_eq!(count.get(), usize::from(index < fail));
            }
        }
    }

    #[test]
    fn nested_failure_separates_incomplete_child_and_completed_parent_ownership() {
        let drops: [Cell<usize>; 6] = core::array::from_fn(|_| Cell::new(0));
        let result = try_from_fn::<_, 2>(|outer| {
            try_from_fn::<_, 3>(|inner| {
                let index = outer * 3 + inner;
                if index == 4 {
                    return Err(NC_E_INVALID_ARG);
                }
                Ok(Tracked { index, drops: &drops, panic_at: None })
            })
        });
        assert!(matches!(result, Err(NC_E_INVALID_ARG)));
        assert_eq!(drops.each_ref().map(Cell::get), [1, 1, 1, 1, 0, 0]);
    }

    std::thread_local! {
        /// Thread-local accounting keeps independent parallel test cases isolated.
        static ALIGNED_DROPS: Cell<usize> = const { Cell::new(0) };
    }
    /// Alignment must survive even though this owner occupies zero bytes.
    #[repr(align(64))]
    struct AlignedDrop;
    impl Drop for AlignedDrop {
        fn drop(&mut self) {
            ALIGNED_DROPS.with(|count| count.set(count.get() + 1));
        }
    }

    #[test]
    fn empty_and_aligned_zero_sized_elements_have_logical_owners() {
        ALIGNED_DROPS.with(|count| count.set(0));
        {
            let _empty = try_from_fn::<AlignedDrop, 0>(|_| panic!("empty generator called")).unwrap();
        }
        // Empty arrays need not fabricate an inhabitant of their element type.
        enum Uninhabited {}
        assert!(try_from_fn::<Uninhabited, 0>(|_| panic!("uninhabited generator called")).is_ok());
        assert_eq!(try_from_fn::<u8, 1>(|_| Ok(7)), Ok([7]));
        assert_eq!(try_from_fn::<u8, 1>(|_| Err(NC_E_INVALID_ARG)), Err(NC_E_INVALID_ARG));
        assert_eq!(ALIGNED_DROPS.with(Cell::get), 0);
        let values = try_from_fn::<_, 257>(|_| Ok(AlignedDrop)).unwrap();
        assert_eq!(core::mem::size_of_val(&values), 0);
        assert_eq!((&values as *const _ as usize) % 64, 0);
        drop(values);
        assert_eq!(ALIGNED_DROPS.with(Cell::get), 257);
        let result = try_from_fn::<_, 257>(|index| if index == 17 { Err(NC_E_INVALID_ARG) } else { Ok(AlignedDrop) });
        assert!(result.is_err());
        assert_eq!(ALIGNED_DROPS.with(Cell::get), 274);
    }

    /// Abort builds cannot observe unwind cleanup; stable SDK/Miri runs cover it.
    #[cfg(panic = "unwind")]
    #[test]
    fn child_panics_drop_each_completed_prefix() {
        for fail in 0..5 {
            let drops: [Cell<usize>; 5] = core::array::from_fn(|_| Cell::new(0));
            let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                try_from_fn::<_, 5>(|index| {
                    assert_ne!(index, fail, "injected child panic");
                    Ok(Tracked { index, drops: &drops, panic_at: None })
                })
            }));
            assert!(panic.is_err());
            for (index, count) in drops.iter().enumerate() {
                assert_eq!(count.get(), usize::from(index < fail));
            }
        }
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn one_panicking_destructor_still_drops_the_remaining_prefix() {
        for panic_at in 0..3 {
            let drops: [Cell<usize>; 4] = core::array::from_fn(|_| Cell::new(0));
            let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                try_from_fn::<_, 4>(|index| {
                    if index == 3 {
                        return Err(NC_E_INVALID_ARG);
                    }
                    Ok(Tracked {
                        index,
                        drops: &drops,
                        panic_at: Some(panic_at),
                    })
                })
            }));
            assert!(panic.is_err());
            assert_eq!(drops.each_ref().map(Cell::get), [1, 1, 1, 0]);
        }
    }
}
