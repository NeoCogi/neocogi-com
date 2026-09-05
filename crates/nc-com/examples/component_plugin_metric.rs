use nc_com::{NcGuid, NcStatus, com_class, com_interface};

const METRIC_CLASS_ID: NcGuid = nc_com::guid_from_qualified_name("nc.examples.multi.Metric");

const COMPONENT_PLUGIN_METRIC_ID: NcGuid =
    nc_com::guid_from_qualified_name("nc.examples.multi.metric");

const FLAG_PLUGIN_ID: NcGuid = nc_com::guid_from_qualified_name("nc.examples.multi.flag");

#[com_interface(auto_guid = "nc.examples.multi.Metric:v1")]
pub trait IMultiMetric {
    fn metric(&self) -> Result<u32, NcStatus>;
}

#[com_class(
    class_id(METRIC_CLASS_ID),
    name = "nc.examples.multi.Metric",
    version = "1.0.0",
    constructor(args = none),
    interfaces(IMultiMetric)
)]
pub struct MultiMetric;

impl IMultiMetric for MultiMetric {
    fn metric(&self) -> Result<u32, NcStatus> {
        Ok(11)
    }
}

nc_com::plugin! {
    plugin_id(COMPONENT_PLUGIN_METRIC_ID),
    name = "nc.examples.multi.metric",
    version = "1.0.0",
    dependencies = [FLAG_PLUGIN_ID],
    classes = [MultiMetric],
}
