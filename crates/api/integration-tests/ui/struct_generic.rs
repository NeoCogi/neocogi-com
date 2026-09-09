use neocogi_com::com_struct;

#[com_struct]
pub struct BadGeneric<T> {
    value: T,
}

fn main() {}
