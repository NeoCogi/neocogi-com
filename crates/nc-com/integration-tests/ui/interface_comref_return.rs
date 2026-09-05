use nc_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.ComRefReturn.Object:v1")]
pub trait IObject {
    fn id(&self) -> Result<u32, nc_com::NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.ui.ComRefReturn:v1")]
pub trait IComRefReturn {
    fn value(&self) -> Result<nc_com::ComRef<'_, IObjectAbi>, nc_com::NcStatus>;
}

fn main() {}
