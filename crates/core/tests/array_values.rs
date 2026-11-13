//! Split wire/backing construction preserves prefix and child ownership.
use std::cell::Cell;
use neocogi_com_core::{ComValue, NcStatus, ValueBudget, ValueCost, ValueArray, NC_E_INVALID_ARG};

std::thread_local! {
    /// Independent test threads never share lifecycle observations.
    static OWNERS: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
}
/// Backing is zero-sized but highly aligned, unlike its one-byte wire.
#[repr(align(64))]
struct Backing;
impl Backing {
    /// Tracks ownership before the child can return or fail.
    fn new() -> Self {
        OWNERS.with(|count| {
            let (made, dropped) = count.get();
            count.set((made + 1, dropped));
        });
        Self
    }
}
impl Drop for Backing {
    fn drop(&mut self) {
        OWNERS.with(|count| {
            let (made, dropped) = count.get();
            count.set((made, dropped + 1));
        });
    }
}
/// Only Value is in this codec's accepted domain; other cases inject failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Unit {
    /// The normal checked unit representation.
    Value,
    /// Reject after acquiring child-local backing, before returning a pair.
    Reject,
    /// Unwind after acquiring backing; only constructed in unwind-enabled tests.
    #[cfg(panic = "unwind")]
    Unwind,
}
// SAFETY: raw zero is checked as unit, no pointees exist, and backing is a
// concrete move-independent owner. Fixed conversion allocates no backing memory.
unsafe impl ComValue for Unit {
    const FIXED_COST: Option<ValueCost> = Some(ValueCost::SCALAR);
    type Wire = u8;
    type Storage = Backing;
    fn encode_value(&self, budget: &mut ValueBudget) -> Result<(u8, Backing), NcStatus> {
        budget.value(|_| unsafe { self.encode_prechecked() })
    }
    unsafe fn decode_value(wire: &u8, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
        // SAFETY: the enclosing caller supplies the same initialized raw byte.
        unsafe { <() as ComValue>::decode_value(wire, budget) }.map(|_| Self::Value)
    }
    unsafe fn encode_prechecked(&self) -> Result<(u8, Backing), NcStatus> {
        let backing = Backing::new();
        match self {
            Self::Value => Ok((0, backing)),
            Self::Reject => Err(NC_E_INVALID_ARG),
            #[cfg(panic = "unwind")]
            Self::Unwind => panic!("injected child conversion panic"),
        }
    }
    unsafe fn decode_prechecked(wire: &u8) -> Result<Self, NcStatus> {
        // SAFETY: caller prepaid unit's scalar cost; byte validity is still checked.
        unsafe { <() as ComValue>::decode_prechecked(wire) }.map(|_| Self::Value)
    }
}

#[test]
fn split_encoding_drops_child_local_and_completed_prefix_backing_once() {
    for fail in 0..=4 {
        OWNERS.with(|count| count.set((0, 0)));
        let mut values = [Unit::Value; 4];
        if fail < 4 {
            values[fail] = Unit::Reject;
        }
        let result = values.encode_value(&mut ValueBudget::default());
        if fail == 4 {
            let (wire, backing) = result.unwrap();
            assert_eq!(wire.elements, [0; 4]);
            assert_eq!(core::mem::size_of_val(&backing), 0);
            assert_eq!(OWNERS.with(Cell::get), (4, 0));
            drop(backing);
            assert_eq!(OWNERS.with(Cell::get), (4, 4));
        } else {
            assert!(matches!(result, Err(NC_E_INVALID_ARG)));
            assert_eq!(OWNERS.with(Cell::get), (fail + 1, fail + 1));
        }
    }
}

#[cfg(panic = "unwind")]
#[test]
fn split_encoding_unwinds_child_and_prefix_backing() {
    for fail in 0..4 {
        OWNERS.with(|count| count.set((0, 0)));
        let mut values = [Unit::Value; 4];
        values[fail] = Unit::Unwind;
        let result = std::panic::catch_unwind(|| values.encode_value(&mut ValueBudget::default()));
        assert!(result.is_err());
        assert_eq!(OWNERS.with(Cell::get), (fail + 1, fail + 1));
    }
}

#[test]
fn borrowed_array_conversion_checks_all_raw_units_after_exact_preflight() {
    let valid = ValueArray { elements: [0u8; 4] };
    // Array plus four scalar units costs five values and depth two.
    assert_eq!(
        unsafe { <[Unit; 4]>::decode_value(&valid, &mut ValueBudget::new(0, 5, 2)) },
        Ok([Unit::Value; 4])
    );
    for (values, depth) in [(4, 2), (5, 1)] {
        assert!(unsafe { <[Unit; 4]>::decode_value(&valid, &mut ValueBudget::new(0, values, depth)) }.is_err());
    }
    for index in 0..4 {
        let mut invalid = valid;
        invalid.elements[index] = 1;
        // SAFETY: initialized byte storage deliberately violates unit's content rule.
        assert_eq!(
            unsafe { <[Unit; 4]>::decode_value(&invalid, &mut ValueBudget::new(0, 5, 2)) },
            Err(NC_E_INVALID_ARG)
        );
    }
}
