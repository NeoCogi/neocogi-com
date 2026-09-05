#[cfg(miri)]
// Miri cannot execute native shared-library loading. This stub keeps broad
// target discovery compile-compatible; the safety script uses pure in-process
// registry and COM-object equivalents instead.
fn main() {}

#[cfg(all(unix, not(miri)))]
fn main() {
    multi_host::run();
}

#[cfg(all(not(unix), not(miri)))]
fn main() {
    eprintln!("multi_plugin_host currently demonstrates Unix shared-library loading only");
}

#[cfg(all(unix, not(miri)))]
mod multi_host {
    use nc_com::{NcGuid, NcStatus, Runtime, com_interface};
    use std::env;
    use std::path::PathBuf;

    const COUNTER_CLASS_ID: NcGuid = nc_com::guid_from_qualified_name("nc.examples.multi.Counter");

    const FLAG_CLASS_ID: NcGuid = nc_com::guid_from_qualified_name("nc.examples.multi.Flag");

    const METRIC_CLASS_ID: NcGuid = nc_com::guid_from_qualified_name("nc.examples.multi.Metric");

    const STATE_CLASS_ID: NcGuid = nc_com::guid_from_qualified_name("nc.examples.multi.State");

    const EXTRA_CLASS_ID: NcGuid = nc_com::guid_from_qualified_name("nc.examples.multi.Extra");

    #[allow(dead_code)]
    #[com_interface(auto_guid = "nc.examples.multi.Counter:v1")]
    pub trait IMultiCounter {
        fn value(&self) -> Result<u32, NcStatus>;
    }

    #[allow(dead_code)]
    #[com_interface(auto_guid = "nc.examples.multi.Flag:v1")]
    pub trait IMultiFlag {
        fn flag(&self) -> Result<u32, NcStatus>;
    }

    #[allow(dead_code)]
    #[com_interface(auto_guid = "nc.examples.multi.Metric:v1")]
    pub trait IMultiMetric {
        fn metric(&self) -> Result<u32, NcStatus>;
    }

    #[allow(dead_code)]
    #[com_interface(auto_guid = "nc.examples.multi.State:v1")]
    pub trait IMultiState {
        fn state(&self) -> Result<u32, NcStatus>;
    }

    #[allow(dead_code)]
    #[com_interface(auto_guid = "nc.examples.multi.Extra:v1")]
    pub trait IMultiExtra {
        fn extra(&self) -> Result<u32, NcStatus>;
    }

    pub fn run() {
        let paths = env::args_os()
            .skip(1)
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        if paths.len() != 5 {
            eprintln!(
                "usage: cargo run --example multi_plugin_host -- <plugin-a> <plugin-b> <plugin-c> <plugin-d> <plugin-e>"
            );
            std::process::exit(2);
        }

        let runtime = Runtime::global();
        let loaded = unsafe { runtime.load_plugin_libraries(&paths) };
        assert_eq!(loaded.len(), paths.len());

        runtime
            .require_interface_type::<IMultiCounterAbi>(COUNTER_CLASS_ID)
            .expect("counter class registered");
        runtime
            .require_interface_type::<IMultiFlagAbi>(FLAG_CLASS_ID)
            .expect("flag class registered");
        runtime
            .require_interface_type::<IMultiMetricAbi>(METRIC_CLASS_ID)
            .expect("metric class registered");
        runtime
            .require_interface_type::<IMultiStateAbi>(STATE_CLASS_ID)
            .expect("state class registered");
        runtime
            .require_interface_type::<IMultiExtraAbi>(EXTRA_CLASS_ID)
            .expect("extra class registered");

        for plugin in loaded {
            println!(
                "{} {} {}",
                plugin.module_id(),
                plugin.qualified_name(),
                plugin.plugin_version()
            );
        }
    }
}
