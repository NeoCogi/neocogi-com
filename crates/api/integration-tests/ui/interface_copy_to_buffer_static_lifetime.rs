use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.CopyToBufferStaticLifetime:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait ICopyToBufferStaticLifetime {
    fn read(
        &self,
        output: neocogi_com::NcOutputWriter<'static, u8>,
    ) -> Result<(), neocogi_com::NcStatus>;
}

fn main() {}
