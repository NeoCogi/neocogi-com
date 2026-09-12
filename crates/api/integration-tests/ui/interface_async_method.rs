use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.AsyncMethod:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IAsyncMethod {
    async fn value(&self) -> Result<u32, neocogi_com::NcStatus>;
}

fn main() {}
