use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.RawPointerArg:v1")]
pub trait IRawPointerArg {
    fn value(&self, ptr: *const u8) -> Result<u32, neocogi_com::NcStatus>;
}

fn main() {}
