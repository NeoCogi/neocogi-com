use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.ComRefReturn.Object:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IObject {
    fn id(&self) -> Result<u32, neocogi_com::NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.ui.ComRefReturn:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IComRefReturn {
    fn value(&self) -> Result<neocogi_com::ComRef<'_, IObjectAbi>, neocogi_com::NcStatus>;
}

fn main() {}
