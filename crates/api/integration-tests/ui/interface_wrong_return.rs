use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.WrongReturn:v1")]
pub trait IWrongReturn {
    fn value(&self) -> u32;
}

fn main() {}
