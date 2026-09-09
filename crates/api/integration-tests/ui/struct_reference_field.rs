use neocogi_com::com_struct;

#[com_struct]
pub struct BadReferenceField {
    value: &'static u8,
}

fn main() {}
