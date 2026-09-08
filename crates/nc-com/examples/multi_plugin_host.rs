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
    use nc_com::{NcStatus, Runtime, com_class};
    use std::env;
    use std::path::PathBuf;

    /// Callable counter contract mirrored from its independently built plugin.
    #[allow(dead_code)]
    #[com_class(auto_guid = "nc.examples.multi.Counter:v1")]
    pub trait MultiCounter {
        /// Creates the plugin's stateless counter.
        #[constructor]
        fn new() -> Result<Self, NcStatus>;

        /// Returns the counter value supplied by the component.
        fn value(&self) -> Result<u32, NcStatus>;
    }

    /// Callable flag contract mirrored from its independently built plugin.
    #[allow(dead_code)]
    #[com_class(auto_guid = "nc.examples.multi.Flag:v1")]
    pub trait MultiFlag {
        /// Creates the plugin's stateless flag provider.
        #[constructor]
        fn new() -> Result<Self, NcStatus>;

        /// Returns the flag supplied by the component.
        fn flag(&self) -> Result<u32, NcStatus>;
    }

    /// Callable metric contract mirrored from its independently built plugin.
    #[allow(dead_code)]
    #[com_class(auto_guid = "nc.examples.multi.Metric:v1")]
    pub trait MultiMetric {
        /// Creates the plugin's stateless metric provider.
        #[constructor]
        fn new() -> Result<Self, NcStatus>;

        /// Returns the metric supplied by the component.
        fn metric(&self) -> Result<u32, NcStatus>;
    }

    /// Callable state contract mirrored from its independently built plugin.
    #[allow(dead_code)]
    #[com_class(auto_guid = "nc.examples.multi.State:v1")]
    pub trait MultiState {
        /// Creates the plugin's stateless state provider.
        #[constructor]
        fn new() -> Result<Self, NcStatus>;

        /// Returns the state supplied by the component.
        fn state(&self) -> Result<u32, NcStatus>;
    }

    /// Callable extra-value contract mirrored from its independently built plugin.
    #[allow(dead_code)]
    #[com_class(auto_guid = "nc.examples.multi.Extra:v1")]
    pub trait MultiExtra {
        /// Creates the plugin's stateless extra-value provider.
        #[constructor]
        fn new() -> Result<Self, NcStatus>;

        /// Returns the extra value supplied by the component.
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
            .require_interface_type::<MultiCounterAbi>(MultiCounterAbi::CLASS_ID)
            .expect("counter class registered");
        runtime
            .require_interface_type::<MultiFlagAbi>(MultiFlagAbi::CLASS_ID)
            .expect("flag class registered");
        runtime
            .require_interface_type::<MultiMetricAbi>(MultiMetricAbi::CLASS_ID)
            .expect("metric class registered");
        runtime
            .require_interface_type::<MultiStateAbi>(MultiStateAbi::CLASS_ID)
            .expect("state class registered");
        runtime
            .require_interface_type::<MultiExtraAbi>(MultiExtraAbi::CLASS_ID)
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
