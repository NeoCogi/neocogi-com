//! Packed application storage needs an explicit aligned-field persistence codec.
#[derive(neocogi_com_sdk::ComSerialize)]
#[repr(C, packed)]
struct Packed {
    value: u32,
}

#[derive(neocogi_com_sdk::ComSerialize)]
#[repr(C, packed(2))]
struct PackedTwo {
    value: u32,
}

fn main() {}
