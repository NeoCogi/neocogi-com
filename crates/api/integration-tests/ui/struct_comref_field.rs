use neocogi_com::com_struct;

#[com_struct]
pub struct BadComRefField {
    value: neocogi_com::ComRef<'static, neocogi_com::IUnknown>,
}

fn main() {}
