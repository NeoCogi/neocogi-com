use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.GeneratedNameCollision:v1", assigned_signature = b"99999999999999999999999999999999")]
trait IGeneratedNameCollision {
    fn fixed_locals(
        &self,
        this: u32,
        object: u32,
        __nc_value: u32,
        __nc_out: u32,
    ) -> Result<u32, neocogi_com::NcStatus>;

    fn buffer_slot(
        &self,
        data: neocogi_com::NcCopyFromBufferOffset<'_, u8>,
        __nc_data_len: u64,
    ) -> Result<(), neocogi_com::NcStatus>;
}

fn main() {}
