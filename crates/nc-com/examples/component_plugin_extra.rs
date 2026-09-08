use nc_com::{NcGuid, NcStatus, com_object, com_interface};

const EXTRA_CLASS_ID: NcGuid = nc_com::guid_from_qualified_name("nc.examples.multi.Extra");

const COMPONENT_PLUGIN_EXTRA_ID: NcGuid =
    nc_com::guid_from_qualified_name("nc.examples.multi.extra");

const STATE_PLUGIN_ID: NcGuid = nc_com::guid_from_qualified_name("nc.examples.multi.state");

#[com_interface(auto_guid = "nc.examples.multi.Extra:v1")]
pub trait IMultiExtra {
    fn extra(&self) -> Result<u32, NcStatus>;
}

#[com_object(
    class_id(EXTRA_CLASS_ID),
    name = "nc.examples.multi.Extra",
    version = "1.0.0",
    constructor(args = none),
    interfaces(IMultiExtra)
)]
pub struct MultiExtra;

impl IMultiExtra for MultiExtra {
    fn extra(&self) -> Result<u32, NcStatus> {
        Ok(19)
    }
}

nc_com::plugin! {
    plugin_id(COMPONENT_PLUGIN_EXTRA_ID),
    name = "nc.examples.multi.extra",
    version = "1.0.0",
    dependencies = [STATE_PLUGIN_ID],
    classes = [MultiExtra],
}

// This crate is the final shared library, so it owns the sole dynamic entrypoint.
nc_com::export_plugin!();
