use nc_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.CopyFromBufferStaticLifetime:v1")]
pub trait ICopyFromBufferStaticLifetime {
    fn write(
        &self,
        input: nc_com::NcCopyFromBufferOffset<'static, u8>,
    ) -> Result<(), nc_com::NcStatus>;
}

fn main() {}
