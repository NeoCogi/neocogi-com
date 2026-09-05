use nc_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.CopyToBufferStaticLifetime:v1")]
pub trait ICopyToBufferStaticLifetime {
    fn read(
        &self,
        output: nc_com::NcCopyToBufferOffset<'static, u8>,
    ) -> Result<(), nc_com::NcStatus>;
}

fn main() {}
