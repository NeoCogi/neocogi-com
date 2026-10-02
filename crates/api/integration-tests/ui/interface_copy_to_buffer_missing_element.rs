use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.CopyToBufferMissingElement:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait ICopyToBufferMissingElement {
    fn read(
        &self,
        output: neocogi_com::NcOutputWriter<'_>,
    ) -> Result<(), neocogi_com::NcStatus>;
}

fn main() {}
