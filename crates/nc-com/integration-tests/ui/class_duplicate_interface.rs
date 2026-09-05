use nc_com::{NcStatus, com_class, com_interface};

#[com_interface(auto_guid = "nc.tests.ui.DuplicateClassInterface:v1")]
pub trait IDuplicateClassInterface {
    fn value(&self) -> Result<u32, NcStatus>;
}

#[com_class(interfaces(
    IDuplicateClassInterface,
    IDuplicateClassInterface
))]
pub struct DuplicateInterface;

fn main() {}
