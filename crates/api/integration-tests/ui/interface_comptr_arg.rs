use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.ComPtrArg.Object:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IObject {
    fn id(&self) -> Result<u32, neocogi_com::NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.ui.ComPtrArg:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IComPtrArg {
    fn value(&self, object: neocogi_com::ComPtr<IObjectAbi>) -> Result<u32, neocogi_com::NcStatus>;
}

fn main() {}
