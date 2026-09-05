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
    use nc_com::{
        ComInterface, ComPtr, IUnknown, NC_COM_CORE_ABI_VERSION, NC_COM_RUNTIME_ABI_VERSION,
        NcGuid, NcStatus, Runtime, ThreadSafe, com_class, com_interface,
    };
    use std::env;
    use std::path::Path;

    const GREETER_CLASS_ID: NcGuid =
        nc_com::guid_from_qualified_name("nc.examples.component.Greeter");

    #[allow(dead_code)]
    #[com_interface(auto_guid = "nc.examples.component.Greeter:v1")]
    pub trait IGreeter {
        fn greeting_code(&self) -> Result<u32, NcStatus>;
    }

    #[com_interface(auto_guid = "nc.examples.component.GreeterArgs:v1")]
    pub trait IGreeterArgs {
        fn seed(&self) -> Result<u32, NcStatus>;
    }

    #[com_class(interfaces(IGreeterArgs))]
    pub struct GreeterArgs {
        seed: u32,
    }

    impl IGreeterArgs for GreeterArgs {
        fn seed(&self) -> Result<u32, NcStatus> {
            Ok(self.seed)
        }
    }

    pub fn run() {
        let Some(plugin_path) = env::args_os().nth(1) else {
            eprintln!(
                "usage: LD_LIBRARY_PATH=<runtime-dir> cargo run --example runtime_host --features runtime -- <component shared library>"
            );
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
        println!(
            "plugin: {} {}",
            loaded.qualified_name(),
            loaded.plugin_version()
        );
        println!("path: {}", loaded.path().display());

        let classes = runtime.inspection().class_ids_in_module(module_id).unwrap();
        assert!(
            classes.contains(&GREETER_CLASS_ID),
            "runtime inspection did not report the greeter class"
        );

        let class_module_id = runtime
            .inspection()
            .module_id_for_class(GREETER_CLASS_ID)
            .unwrap();
        assert_eq!(class_module_id, Some(module_id));

        let class_info = runtime
            .inspection()
            .class_info(GREETER_CLASS_ID)
            .unwrap()
            .expect("greeter class info");
        assert_eq!(class_info.threading, nc_com::NcAbiThreadingMode::ThreadSafe);

        let interfaces = runtime
            .inspection()
            .interface_ids_in_module(module_id)
            .unwrap();
        assert!(
            interfaces.contains(&IGreeterAbi::INTERFACE_ID),
            "runtime inspection did not report the greeter interface"
        );
        runtime
            .require_interface(GREETER_CLASS_ID, IGreeterAbi::INTERFACE_ID)
            .unwrap();

        let args: ComPtr<IUnknown> = GreeterArgs::from_value(GreeterArgs { seed: 23 });
        let greeter = unsafe {
            runtime.construct_class_as_with_model::<IGreeterAbi, ThreadSafe>(
                GREETER_CLASS_ID,
                args.as_mut_ptr(),
            )
        }
        .expect("construct thread-safe greeter");
        let greeting_code = std::thread::spawn(move || greeter.greeting_code().unwrap())
            .join()
            .expect("greeter worker");
        println!("greeting code: {greeting_code}");
    }
}
