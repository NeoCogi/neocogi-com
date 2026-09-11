use neocogi_com::{NcGuid, NcStatus, com_class, com_object};

const COMPONENT_PLUGIN_METRIC_ID: NcGuid = neocogi_com::guid_from_qualified_name("nc.examples.multi.metric");

const FLAG_PLUGIN_ID: NcGuid = neocogi_com::guid_from_qualified_name("nc.examples.multi.flag");

/// Callable metric contract shared with the independently compiled host.
#[com_class(auto_guid = "nc.examples.multi.Metric:v1")]
pub trait MultiMetric {
    /// Creates the stateless example metric.
    #[constructor]
    fn new() -> Result<Self, NcStatus>;

    /// Returns the component's fixed demonstration metric.
    fn metric(&self) -> Result<u32, NcStatus>;
}

/// Private storage implementing the public callable metric contract.
#[com_object(constructible(MultiMetric))]
pub struct MultiMetricObject;

impl MultiMetric for MultiMetricObject {
    /// Creates the unit implementation without auxiliary constructor state.
    fn new() -> Result<Self, NcStatus> {
        Ok(Self)
    }

    /// Returns the fixed value used to identify this component.
    fn metric(&self) -> Result<u32, NcStatus> {
        Ok(11)
    }
}

neocogi_com::plugin! {
    plugin_id(COMPONENT_PLUGIN_METRIC_ID),
    name = "nc.examples.multi.metric",
    version = "1.0.0",
    dependencies = [FLAG_PLUGIN_ID],
    classes = [MultiMetricObject],
}

// This crate is the final shared library, so it owns the sole dynamic entrypoint.
neocogi_com::export_plugin!();
