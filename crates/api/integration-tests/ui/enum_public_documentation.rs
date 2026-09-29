//! Fully exported enum wire types work in a crate that requires complete public documentation.
#![deny(missing_docs)]
use neocogi_com::{com_enum, ComValue};

#[com_enum(tag = u8, assigned_signature = b"00000000000000000000000000000045")]
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
    let value = Documented::Named { weight: 2.0 };
    let (wire, _storage) = value.encode_value(&mut Default::default()).unwrap();
    // SAFETY: the checked encoder produced this fully initialized inline wire.
    let decoded = unsafe { Documented::decode_value(wire, &mut Default::default()) }.unwrap();
    assert!(matches!(decoded, Documented::Named { weight: 2.0 }));
}
