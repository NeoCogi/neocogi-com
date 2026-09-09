use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.CopyFromBufferBadElement:v1")]
pub trait ICopyFromBufferBadElement {
    fn write(&self, input: neocogi_com::NcCopyFromBufferOffset<'_, String>)
        -> Result<(), neocogi_com::NcStatus>;
}

fn main() {}
