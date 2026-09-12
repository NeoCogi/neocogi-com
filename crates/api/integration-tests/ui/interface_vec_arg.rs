use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.VecArg:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IVecArg {
    fn value(&self, value: Vec<u8>) -> Result<u32, neocogi_com::NcStatus>;
}

fn main() {}
