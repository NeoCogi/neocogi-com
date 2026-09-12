use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.MutReceiver:v1", assigned_signature = b"00000000000000000000000000000047")]
pub trait IMutReceiver {
    fn value(&mut self) -> Result<u32, neocogi_com::NcStatus>;
}

fn main() {}
