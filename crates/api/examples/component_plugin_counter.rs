use neocogi_com::{NcGuid, NcStatus, com_class, com_object};

const COUNTER_PLUGIN_ID: NcGuid = neocogi_com::guid_from_qualified_name("nc.examples.multi.counter");

/// Callable counter contract shared with the independently compiled host.
#[com_class(auto_guid = "nc.examples.multi.Counter:v1")]
pub trait MultiCounter {
    /// Creates the stateless example counter.
    #[constructor]
    fn new() -> Result<Self, NcStatus>;

    /// Returns the counter's fixed demonstration value.
    fn value(&self) -> Result<u32, NcStatus>;
}

/// Private storage implementing the public callable counter contract.
#[com_object(constructible(MultiCounter))]
pub struct MultiCounterObject;

impl MultiCounter for MultiCounterObject {
    /// Creates the unit implementation without auxiliary constructor state.
    fn new() -> Result<Self, NcStatus> {
        Ok(Self)
    }

    /// Returns the fixed value used to identify this component.
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(7)
    }
}

neocogi_com::plugin! {
    plugin_id(COUNTER_PLUGIN_ID),
    name = "nc.examples.multi.counter",
    version = "1.0.0",
    classes = [MultiCounterObject],
}

// This crate is the final shared library, so it owns the sole dynamic entrypoint.
neocogi_com::export_plugin!();
