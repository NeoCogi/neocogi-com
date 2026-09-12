use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.CopyFromBufferMissingLifetime:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait ICopyFromBufferMissingLifetime {
    fn write(
        &self,
        input: neocogi_com::NcCopyFromBufferOffset<u8>,
    ) -> Result<(), neocogi_com::NcStatus>;
}

fn main() {}
