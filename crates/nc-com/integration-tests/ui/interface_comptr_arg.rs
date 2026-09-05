use nc_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.ComPtrArg.Object:v1")]
pub trait IObject {
    fn id(&self) -> Result<u32, nc_com::NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.ui.ComPtrArg:v1")]
pub trait IComPtrArg {
    fn value(&self, object: nc_com::ComPtr<IObjectAbi>) -> Result<u32, nc_com::NcStatus>;
}

fn main() {}
