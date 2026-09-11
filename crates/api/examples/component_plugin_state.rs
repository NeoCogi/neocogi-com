use neocogi_com::{NcGuid, NcStatus, com_class, com_object};

const COMPONENT_PLUGIN_STATE_ID: NcGuid = neocogi_com::guid_from_qualified_name("nc.examples.multi.state");

const METRIC_PLUGIN_ID: NcGuid = neocogi_com::guid_from_qualified_name("nc.examples.multi.metric");

/// Callable state contract shared with the independently compiled host.
#[com_class(auto_guid = "nc.examples.multi.State:v1")]
pub trait MultiState {
    /// Creates the stateless example state provider.
    #[constructor]
    fn new() -> Result<Self, NcStatus>;

    /// Returns the component's fixed demonstration state.
    fn state(&self) -> Result<u32, NcStatus>;
}

/// Private storage implementing the public callable state contract.
#[com_object(constructible(MultiState))]
pub struct MultiStateObject;

impl MultiState for MultiStateObject {
    /// Creates the unit implementation without auxiliary constructor state.
    fn new() -> Result<Self, NcStatus> {
        Ok(Self)
    }

    /// Returns the fixed value used to identify this component.
    fn state(&self) -> Result<u32, NcStatus> {
        Ok(13)
    }
}

neocogi_com::plugin! {
    plugin_id(COMPONENT_PLUGIN_STATE_ID),
    name = "nc.examples.multi.state",
    version = "1.0.0",
    dependencies = [METRIC_PLUGIN_ID],
    classes = [MultiStateObject],
}

// This crate is the final shared library, so it owns the sole dynamic entrypoint.
neocogi_com::export_plugin!();
