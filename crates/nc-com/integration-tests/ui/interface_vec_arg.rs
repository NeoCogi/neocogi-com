use nc_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.VecArg:v1")]
pub trait IVecArg {
    fn value(&self, value: Vec<u8>) -> Result<u32, nc_com::NcStatus>;
}

fn main() {}
