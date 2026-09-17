use neocogi_com::{com_object, com_interface};

#[com_interface(auto_guid = "nc.tests.ui.MissingImpl:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IMissingImpl {
    fn value(&self) -> Result<u32, neocogi_com::NcStatus>;
}

#[com_object(interfaces(IMissingImpl))]
pub struct MissingImpl;

fn main() {
    let _object = MissingImpl::from_value(MissingImpl);
}
