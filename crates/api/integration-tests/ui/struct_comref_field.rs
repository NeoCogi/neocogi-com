use neocogi_com::com_struct;

#[com_struct(assigned_signature = b"88888888888888888888888888888888")]
pub struct BadComRefField {
    value: neocogi_com::ComRef<'static, neocogi_com::IUnknown>,
}

fn main() {}
