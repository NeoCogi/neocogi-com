use neocogi_com::{com_object, com_interface, NcStatus};

#[com_interface(auto_guid = "test.BorrowedClass:v1", assigned_signature = b"99999999999999999999999999999999")]
trait IRead {
    fn value(&self) -> Result<u32, NcStatus>;
}

#[com_object(interfaces(IRead))]
struct Reader<'a> {
    value: &'a u32,
}

fn main() {}
