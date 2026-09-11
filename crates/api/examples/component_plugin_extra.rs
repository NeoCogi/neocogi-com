use neocogi_com::{NcGuid, NcStatus, com_class, com_object};

const COMPONENT_PLUGIN_EXTRA_ID: NcGuid = neocogi_com::guid_from_qualified_name("nc.examples.multi.extra");

const STATE_PLUGIN_ID: NcGuid = neocogi_com::guid_from_qualified_name("nc.examples.multi.state");

/// Callable extra-value contract shared with the independently compiled host.
#[com_class(auto_guid = "nc.examples.multi.Extra:v1")]
pub trait MultiExtra {
    /// Creates the stateless example value provider.
    #[constructor]
    fn new() -> Result<Self, NcStatus>;

    /// Returns the component's fixed demonstration value.
    fn extra(&self) -> Result<u32, NcStatus>;
}

/// Private storage implementing the public callable extra-value contract.
#[com_object(constructible(MultiExtra))]
pub struct MultiExtraObject;

impl MultiExtra for MultiExtraObject {
    /// Creates the unit implementation without auxiliary constructor state.
    fn new() -> Result<Self, NcStatus> {
        Ok(Self)
    }

    /// Returns the fixed value used to identify this component.
    fn extra(&self) -> Result<u32, NcStatus> {
        Ok(19)
    }
}

neocogi_com::plugin! {
    plugin_id(COMPONENT_PLUGIN_EXTRA_ID),
    name = "nc.examples.multi.extra",
    version = "1.0.0",
    dependencies = [STATE_PLUGIN_ID],
    classes = [MultiExtraObject],
}

// This crate is the final shared library, so it owns the sole dynamic entrypoint.
neocogi_com::export_plugin!();
