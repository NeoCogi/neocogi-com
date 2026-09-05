use nc_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.CopyToBufferBadElement:v1")]
pub trait ICopyToBufferBadElement {
    fn read(&self, output: nc_com::NcCopyToBufferOffset<'_, String>)
        -> Result<(), nc_com::NcStatus>;
}

fn main() {}
