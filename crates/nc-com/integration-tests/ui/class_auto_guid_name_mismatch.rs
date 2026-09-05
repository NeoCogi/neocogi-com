use nc_com::{NcStatus, com_class, com_interface};

#[com_interface(auto_guid = "nc.tests.ui.ClassIdentityMismatch:v1")]
trait IClassIdentityMismatch {
    fn value(&self) -> Result<u32, NcStatus>;
}

#[com_class(
    class_id(auto_guid = "nc.tests.ui.ClassIdentityMismatch:class:v1"),
    name = "nc.tests.ui.ClassIdentityMismatch",
    version = "1.0.0",
    constructor(args = none),
    interfaces(IClassIdentityMismatch)
)]
struct ClassIdentityMismatch;

impl IClassIdentityMismatch for ClassIdentityMismatch {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(0)
    }
}

fn main() {}
