use nc_com::{com_object, com_interface, NcStatus};

#[com_interface(auto_guid = "test.GenericDescriptor:v1")]
trait IRead {
    fn value(&self) -> Result<u32, NcStatus>;
}

#[com_object(
    name = "test.GenericDescriptor:class:v1",
    class_id(auto_guid = "test.GenericDescriptor:class:v1"),
    version = "1.0.0",
    constructor(args = none, default),
    interfaces(IRead)
)]
struct Reader<T> {
    value: T,
}

fn main() {}
