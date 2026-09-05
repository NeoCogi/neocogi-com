use nc_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.ReferenceArg:v1")]
pub trait IReferenceArg {
    fn value(&self, value: &u32) -> Result<u32, nc_com::NcStatus>;
}

fn main() {}
