use neocogi_com::com_struct;

#[com_struct]
pub struct BadVecField {
    value: Vec<u8>,
}

fn main() {}
