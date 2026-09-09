use neocogi_com::{com_enum, com_interface};

#[com_interface(auto_guid = "nc.tests.ui.EnumComPtrPayload.Interface:v1")]
pub trait IObject {
    fn value(&self) -> Result<u32, neocogi_com::NcStatus>;
}

#[com_enum(tag = u32)]
pub enum BadPayload {
    Object(neocogi_com::ComPtr<IObjectAbi>),
}

fn main() {}
