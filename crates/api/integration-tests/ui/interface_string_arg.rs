use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.StringArg:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IStringArg {
    fn value(&self, value: String) -> Result<u32, neocogi_com::NcStatus>;
}

fn main() {}
