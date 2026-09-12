use neocogi_com::com_struct;

#[com_struct(assigned_signature = b"88888888888888888888888888888888")]
#[repr(C, packed)]
pub struct BadRepr {
    value: u32,
}

fn main() {}
