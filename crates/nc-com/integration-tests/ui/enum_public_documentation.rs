//! Fully exported enum wire types work in a crate that requires complete public documentation.
#![deny(missing_docs)]
use nc_com::com_enum;

#[com_enum(tag = u8)]
/// Public semantic choice with all three supported payload shapes.
pub enum Documented {
    /// Variant without payload data.
    Empty,
    /// Tuple payload retains its positional field documentation in the generated struct.
    Tuple(
        /// Exact requested extent.
        i32,
    ),
    /// Named payload retains its field documentation in the generated struct.
    Named {
        /// Relative weight used by the consuming policy.
        weight: f32,
    },
}

fn main() {
    let value: DocumentedAbi = Documented::Named { weight: 2.0 }.into();
    assert!(matches!(Documented::try_from(value), Ok(Documented::Named { weight: 2.0 })));
}
