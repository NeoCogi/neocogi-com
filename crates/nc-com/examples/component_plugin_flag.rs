use nc_com::{NcGuid, NcStatus, com_class, com_interface};

const FLAG_CLASS_ID: NcGuid = nc_com::guid_from_qualified_name("nc.examples.multi.Flag");

const COMPONENT_PLUGIN_FLAG_ID: NcGuid = nc_com::guid_from_qualified_name("nc.examples.multi.flag");

const COUNTER_PLUGIN_ID: NcGuid = nc_com::guid_from_qualified_name("nc.examples.multi.counter");

#[com_interface(auto_guid = "nc.examples.multi.Flag:v1")]
pub trait IMultiFlag {
    fn flag(&self) -> Result<u32, NcStatus>;
}

#[com_class(
    class_id(FLAG_CLASS_ID),
    name = "nc.examples.multi.Flag",
    version = "1.0.0",
    constructor(args = none),
    interfaces(IMultiFlag)
)]
pub struct MultiFlag;

impl IMultiFlag for MultiFlag {
    fn flag(&self) -> Result<u32, NcStatus> {
        Ok(17)
    }
}

nc_com::plugin! {
    plugin_id(COMPONENT_PLUGIN_FLAG_ID),
    name = "nc.examples.multi.flag",
    version = "1.0.0",
    dependencies = [COUNTER_PLUGIN_ID],
    classes = [MultiFlag],
}
