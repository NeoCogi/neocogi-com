#[cfg(unix)]
fn main() {
    runtime_host::run();
}

#[cfg(not(unix))]
fn main() {
    eprintln!("runtime_host currently demonstrates Unix dynamic linking only");
}

#[cfg(unix)]
mod runtime_host {
    use neocogi_com::{ComInterface, NC_COM_CORE_ABI_VERSION, NC_COM_RUNTIME_ABI_VERSION, NcStatus, Runtime, ThreadSafe, com_class};
    use std::env;
    use std::path::Path;

    /// Callable class contract shared with the dynamically loaded component.
    #[allow(dead_code)]
    #[com_class(auto_guid = "nc.examples.component.Greeter")]
    pub trait Greeter {
        /// Creates a greeter from one plain seed value.
        #[constructor]
        fn new(seed: u32) -> Result<Self, NcStatus>;

        /// Returns the code computed by the plugin implementation.
        fn greeting_code(&self) -> Result<u32, NcStatus>;
    }

    pub fn run() {
        let Some(plugin_path) = env::args_os().nth(1) else {
            eprintln!("usage: LD_LIBRARY_PATH=<runtime-dir> cargo run --example runtime_host --features runtime -- <component shared library>");
            return;
        };
        let plugin_path = Path::new(&plugin_path);
        let runtime = Runtime::global();

        let runtime_abi = runtime.runtime_abi_version();
        assert_eq!(runtime_abi, NC_COM_RUNTIME_ABI_VERSION);
        println!("runtime ABI version: {runtime_abi}");

        let core_abi = runtime.core_abi_version();
        assert_eq!(core_abi, NC_COM_CORE_ABI_VERSION);
        println!("core ABI version: {core_abi}");

        let loaded = unsafe { runtime.load_plugin_library(plugin_path) };
        let module_id = loaded.module_id();
        println!("loaded module: {}", module_id);
        println!("plugin: {} {}", loaded.qualified_name(), loaded.plugin_version());
        println!("path: {}", loaded.path().display());

        let classes = runtime.inspection().class_ids_in_module(module_id).unwrap();
        assert!(classes.contains(&GreeterAbi::CLASS_ID), "runtime inspection did not report the greeter class");

        let class_module_id = runtime.inspection().module_id_for_class(GreeterAbi::CLASS_ID).unwrap();
        assert_eq!(class_module_id, Some(module_id));

        let class_info = runtime.inspection().class_info(GreeterAbi::CLASS_ID).unwrap().expect("greeter class info");
        assert_eq!(class_info.threading, neocogi_com::NcAbiThreadingMode::ThreadSafe);

        let interfaces = runtime.inspection().interface_ids_in_module(module_id).unwrap();
        assert!(
            interfaces.contains(&GreeterAbi::INTERFACE_ID),
            "runtime inspection did not report the greeter class projection"
        );
        runtime.require_interface(GreeterAbi::CLASS_ID, GreeterAbi::INTERFACE_ID).unwrap();

        let arguments = GreeterAbi::arguments(23).expect("valid constructor arguments");
        let greeter = runtime
            .construct_with_model::<GreeterAbi, ThreadSafe>(arguments)
            .expect("construct thread-safe greeter");
        let greeting_code = std::thread::spawn(move || greeter.greeting_code().unwrap()).join().expect("greeter worker");
        println!("greeting code: {greeting_code}");
    }
}
