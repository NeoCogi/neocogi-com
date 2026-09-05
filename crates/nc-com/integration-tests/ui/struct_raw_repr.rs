use nc_com::com_struct;

#[com_struct]
#[repr(C, packed)]
pub struct BadRepr {
    value: u32,
}

fn main() {}
