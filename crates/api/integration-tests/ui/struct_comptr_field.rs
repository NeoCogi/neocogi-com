use neocogi_com::com_struct;

#[com_struct(assigned_signature = b"88888888888888888888888888888888")]
pub struct BadComPtrField {
    value: neocogi_com::ComPtr<neocogi_com::IUnknown>,
}

fn main() {}
