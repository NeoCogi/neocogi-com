use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.WrongError:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IWrongError {
    fn value(&self) -> Result<u32, u32>;
}

fn main() {}
