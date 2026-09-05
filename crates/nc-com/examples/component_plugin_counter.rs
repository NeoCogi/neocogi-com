use nc_com::{NcGuid, NcStatus, com_class, com_interface};

const COUNTER_CLASS_ID: NcGuid = nc_com::guid_from_qualified_name("nc.examples.multi.Counter");

const COUNTER_PLUGIN_ID: NcGuid = nc_com::guid_from_qualified_name("nc.examples.multi.counter");

#[com_interface(auto_guid = "nc.examples.multi.Counter:v1")]
pub trait IMultiCounter {
    fn value(&self) -> Result<u32, NcStatus>;
}

#[com_class(
    class_id(COUNTER_CLASS_ID),
    name = "nc.examples.multi.Counter",
    version = "1.0.0",
    constructor(args = none),
    interfaces(IMultiCounter)
)]
pub struct MultiCounter;

impl IMultiCounter for MultiCounter {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(7)
    }
}

nc_com::plugin! {
    plugin_id(COUNTER_PLUGIN_ID),
    name = "nc.examples.multi.counter",
    version = "1.0.0",
    classes = [MultiCounter],
}

// This crate is the final shared library, so it owns the sole dynamic entrypoint.
nc_com::export_plugin!();
