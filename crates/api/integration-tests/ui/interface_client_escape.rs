use neocogi_com::{com_interface, ComPtr, NcStatus};

#[com_interface(auto_guid = "nc.tests.ui.ClientBase:v1")]
pub trait IBase {
    fn value(&self) -> Result<u32, NcStatus>;
}
#[com_interface(auto_guid = "nc.tests.ui.ClientDerived:v1")]
pub trait IDerived: IBase {}

// A method view cannot survive the owner which lends it, including after
// dereferencing through an inherited base view.
fn escape(owner: ComPtr<IDerivedAbi>) -> &'static IBaseClient {
    &owner
}

fn main() {}
