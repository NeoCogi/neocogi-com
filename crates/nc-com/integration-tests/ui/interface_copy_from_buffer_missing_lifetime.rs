use nc_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.CopyFromBufferMissingLifetime:v1")]
pub trait ICopyFromBufferMissingLifetime {
    fn write(
        &self,
        input: nc_com::NcCopyFromBufferOffset<u8>,
    ) -> Result<(), nc_com::NcStatus>;
}

fn main() {}
