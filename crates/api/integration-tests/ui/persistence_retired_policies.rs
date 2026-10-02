//! NCB has one positional schema; retired adapter options must fail explicitly.
use neocogi_com::{ComDeserialize, ComSerialize};

/// Sequence selection no longer changes the record representation.
#[derive(ComDeserialize)]
#[com_deserialize(sequence)]
struct Ordered {
    /// Ordinary positional scalar.
    value: u32,
}

/// Variant names never occur in NCB bytes.
#[derive(ComSerialize)]
#[com_deserialize(rename_all = "snake_case")]
enum Named {
    /// The binary codec would encode ordinal zero regardless of this name.
    First,
}

/// This attribute frontend must reject the same retired option as the derives.
#[neocogi_com::com_struct(deserialize, sequence, assigned_signature = b"00000000000000000000000000000001")]
struct Attributed {
    /// Ordinary positional scalar.
    value: u32,
}

/// Compile-fail entrypoint; no rejected declaration is constructed.
fn main() {}
