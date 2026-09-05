use nc_com::{NcStatus, com_class, com_interface};

#[com_interface(auto_guid = "nc.tests.ui.NonUnitDescriptorRequiresDefault:v1")]
pub trait INonUnitDescriptorRequiresDefault {
    fn value(&self) -> Result<u32, NcStatus>;
}

#[com_class(
    class_id(auto_guid = "nc.tests.ui.NonUnitDescriptorRequiresDefault:class:v1"),
    name = "nc.tests.ui.NonUnitDescriptorRequiresDefault:class:v1",
    version = "1.0.0",
    constructor(args = none),
    interfaces(INonUnitDescriptorRequiresDefault)
)]
pub struct NonUnitDescriptorRequiresDefault {
    value: u32,
}

impl INonUnitDescriptorRequiresDefault for NonUnitDescriptorRequiresDefault {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.value)
    }
}

fn main() {}
