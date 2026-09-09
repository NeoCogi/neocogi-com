use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.CopyFromBufferStaticLifetime:v1")]
pub trait ICopyFromBufferStaticLifetime {
    fn write(
        &self,
        input: neocogi_com::NcCopyFromBufferOffset<'static, u8>,
    ) -> Result<(), neocogi_com::NcStatus>;
}

fn main() {}
