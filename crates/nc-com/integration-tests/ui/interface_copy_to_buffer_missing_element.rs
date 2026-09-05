use nc_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.CopyToBufferMissingElement:v1")]
pub trait ICopyToBufferMissingElement {
    fn read(
        &self,
        output: nc_com::NcCopyToBufferOffset<'_>,
    ) -> Result<(), nc_com::NcStatus>;
}

fn main() {}
