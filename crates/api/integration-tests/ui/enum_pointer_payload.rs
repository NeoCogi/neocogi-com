use neocogi_com::{com_enum, com_interface};

#[com_interface(auto_guid = "nc.tests.ui.EnumPointerPayload.Interface:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IObject {
    fn value(&self) -> Result<u32, neocogi_com::NcStatus>;
}

#[com_enum(tag = u32, assigned_signature = b"77777777777777777777777777777777")]
pub enum BadPayload {
    Object(*mut IObjectAbi),
}

fn main() {}
