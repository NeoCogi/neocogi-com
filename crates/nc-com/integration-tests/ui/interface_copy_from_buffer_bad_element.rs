use nc_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.CopyFromBufferBadElement:v1")]
pub trait ICopyFromBufferBadElement {
    fn write(&self, input: nc_com::NcCopyFromBufferOffset<'_, String>)
        -> Result<(), nc_com::NcStatus>;
}

fn main() {}
