use nc_com::{NcGuid, NcStatus, com_class, com_interface};

const STATE_CLASS_ID: NcGuid = nc_com::guid_from_qualified_name("nc.examples.multi.State");

const COMPONENT_PLUGIN_STATE_ID: NcGuid =
    nc_com::guid_from_qualified_name("nc.examples.multi.state");

const METRIC_PLUGIN_ID: NcGuid = nc_com::guid_from_qualified_name("nc.examples.multi.metric");

#[com_interface(auto_guid = "nc.examples.multi.State:v1")]
pub trait IMultiState {
    fn state(&self) -> Result<u32, NcStatus>;
}

#[com_class(
    class_id(STATE_CLASS_ID),
    name = "nc.examples.multi.State",
    version = "1.0.0",
    constructor(args = none),
    interfaces(IMultiState)
)]
pub struct MultiState;

impl IMultiState for MultiState {
    fn state(&self) -> Result<u32, NcStatus> {
        Ok(13)
    }
}

nc_com::plugin! {
    plugin_id(COMPONENT_PLUGIN_STATE_ID),
    name = "nc.examples.multi.state",
    version = "1.0.0",
    dependencies = [METRIC_PLUGIN_ID],
    classes = [MultiState],
}

// This crate is the final shared library, so it owns the sole dynamic entrypoint.
nc_com::export_plugin!();
