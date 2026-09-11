use neocogi_com::{NcGuid, NcStatus, com_class, com_object};

const COMPONENT_PLUGIN_FLAG_ID: NcGuid = neocogi_com::guid_from_qualified_name("nc.examples.multi.flag");

const COUNTER_PLUGIN_ID: NcGuid = neocogi_com::guid_from_qualified_name("nc.examples.multi.counter");

/// Callable flag contract shared with the independently compiled host.
#[com_class(auto_guid = "nc.examples.multi.Flag:v1")]
pub trait MultiFlag {
    /// Creates the stateless example flag.
    #[constructor]
    fn new() -> Result<Self, NcStatus>;

    /// Returns the component's fixed demonstration flag.
    fn flag(&self) -> Result<u32, NcStatus>;
}

/// Private storage implementing the public callable flag contract.
#[com_object(constructible(MultiFlag))]
pub struct MultiFlagObject;

impl MultiFlag for MultiFlagObject {
    /// Creates the unit implementation without auxiliary constructor state.
    fn new() -> Result<Self, NcStatus> {
        Ok(Self)
    }

    /// Returns the fixed value used to identify this component.
    fn flag(&self) -> Result<u32, NcStatus> {
        Ok(17)
    }
}

neocogi_com::plugin! {
    plugin_id(COMPONENT_PLUGIN_FLAG_ID),
    name = "nc.examples.multi.flag",
    version = "1.0.0",
    dependencies = [COUNTER_PLUGIN_ID],
    classes = [MultiFlagObject],
}

// This crate is the final shared library, so it owns the sole dynamic entrypoint.
neocogi_com::export_plugin!();
