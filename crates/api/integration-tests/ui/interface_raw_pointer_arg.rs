use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.RawPointerArg:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IRawPointerArg {
    fn value(&self, ptr: *const u8) -> Result<u32, neocogi_com::NcStatus>;
}

fn main() {}
