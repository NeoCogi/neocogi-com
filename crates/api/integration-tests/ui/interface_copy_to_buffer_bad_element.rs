use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.CopyToBufferBadElement:v1")]
pub trait ICopyToBufferBadElement {
    fn read(&self, output: neocogi_com::NcCopyToBufferOffset<'_, String>)
        -> Result<(), neocogi_com::NcStatus>;
}

fn main() {}
