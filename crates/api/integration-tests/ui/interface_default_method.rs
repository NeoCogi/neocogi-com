use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.DefaultMethod:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IDefaultMethod {
    fn value(&self) -> Result<u32, neocogi_com::NcStatus> {
        Ok(1)
    }
}

fn main() {}
