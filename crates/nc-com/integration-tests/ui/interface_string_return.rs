use nc_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.StringReturn:v1")]
pub trait IStringReturn {
    fn value(&self) -> Result<String, nc_com::NcStatus>;
}

fn main() {}
