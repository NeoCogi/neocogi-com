use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.StringArg:v1")]
pub trait IStringArg {
    fn value(&self, value: String) -> Result<u32, neocogi_com::NcStatus>;
}

fn main() {}
