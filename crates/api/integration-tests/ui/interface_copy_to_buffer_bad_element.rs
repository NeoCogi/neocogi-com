use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.CopyToBufferBadElement:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait ICopyToBufferBadElement {
    fn read(&self, output: neocogi_com::NcOutputWriter<'_, String>)
        -> Result<(), neocogi_com::NcStatus>;
}

fn main() {}
