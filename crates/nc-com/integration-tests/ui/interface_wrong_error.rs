use nc_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.WrongError:v1")]
pub trait IWrongError {
    fn value(&self) -> Result<u32, u32>;
}

fn main() {}
