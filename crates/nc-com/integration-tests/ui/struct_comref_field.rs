use nc_com::com_struct;

#[com_struct]
pub struct BadComRefField {
    value: nc_com::ComRef<'static, nc_com::IUnknown>,
}

fn main() {}
