use nc_com::{com_object, com_interface};

#[com_interface(auto_guid = "nc.tests.ui.MissingImpl:v1")]
pub trait IMissingImpl {
    fn value(&self) -> Result<u32, nc_com::NcStatus>;
}

#[com_object(interfaces(IMissingImpl))]
pub struct MissingImpl;

fn main() {
    let _object = MissingImpl::create();
}
