use neocogi_com::com_struct;

#[com_struct(assigned_signature = b"88888888888888888888888888888888")]
pub struct BadStringField {
    value: String,
}

fn main() {}
