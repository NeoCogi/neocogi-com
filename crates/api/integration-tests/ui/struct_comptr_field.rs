use neocogi_com::com_struct;

#[com_struct]
pub struct BadComPtrField {
    value: neocogi_com::ComPtr<neocogi_com::IUnknown>,
}

fn main() {}
