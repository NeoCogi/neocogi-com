use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.CopyToBufferStaticLifetime:v1")]
pub trait ICopyToBufferStaticLifetime {
    fn read(
        &self,
        output: neocogi_com::NcCopyToBufferOffset<'static, u8>,
    ) -> Result<(), neocogi_com::NcStatus>;
}

fn main() {}
