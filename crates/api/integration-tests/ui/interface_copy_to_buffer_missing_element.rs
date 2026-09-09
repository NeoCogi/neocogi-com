use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.CopyToBufferMissingElement:v1")]
pub trait ICopyToBufferMissingElement {
    fn read(
        &self,
        output: neocogi_com::NcCopyToBufferOffset<'_>,
    ) -> Result<(), neocogi_com::NcStatus>;
}

fn main() {}
