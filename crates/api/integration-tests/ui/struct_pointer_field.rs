use neocogi_com::com_struct;

#[com_struct]
pub struct BadPointerField {
    value: *const u8,
}

fn main() {}
