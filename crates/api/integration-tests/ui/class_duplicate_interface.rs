use neocogi_com::{NcStatus, com_object, com_interface};

#[com_interface(auto_guid = "nc.tests.ui.DuplicateClassInterface:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IDuplicateClassInterface {
    fn value(&self) -> Result<u32, NcStatus>;
}

#[com_object(interfaces(
    IDuplicateClassInterface,
    IDuplicateClassInterface
))]
pub struct DuplicateInterface;

fn main() {}
