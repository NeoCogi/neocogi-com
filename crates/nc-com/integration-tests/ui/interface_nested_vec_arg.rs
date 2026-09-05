use nc_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.NestedVecArg:v1")]
pub trait INestedVecArg {
    fn value(&self, value: Option<Vec<u8>>) -> Result<u32, nc_com::NcStatus>;
}

fn main() {}
