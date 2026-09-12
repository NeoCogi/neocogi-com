use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.NoncollidingGeneratedNames:v1", assigned_signature = b"00000000000000000000000000000046")]
trait INoncollidingGeneratedNames {
    fn write(
        &self,
        object: neocogi_com::NcCopyFromBufferOffset<'_, u8>,
        __nc_out: u64,
        result: u64,
        status: u64,
    ) -> Result<(), neocogi_com::NcStatus>;
}

fn main() {}
