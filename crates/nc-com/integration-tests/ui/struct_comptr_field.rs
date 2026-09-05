use nc_com::com_struct;

#[com_struct]
pub struct BadComPtrField {
    value: nc_com::ComPtr<nc_com::IUnknown>,
}

fn main() {}
