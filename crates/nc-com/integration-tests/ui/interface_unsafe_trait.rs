use nc_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.UnsafeTrait:v1")]
pub unsafe trait IUnsafeTrait {
    fn value(&self) -> Result<u32, nc_com::NcStatus>;
}

fn main() {}
