use neocogi_com::{NC_OK, NcStatus, plugin};

unsafe extern "C" fn initialize() -> NcStatus {
    NC_OK
}

plugin! {
    plugin_id(auto_guid = "nc.tests.ui.PluginInitializerRemoved:v1"),
    name = "nc.tests.ui.PluginInitializerRemoved:v1",
    version = "1.0.0",
    init = initialize,
    classes = [],
}

fn main() {}
