include!(concat!(env!("OUT_DIR"), "/proof_archive.include.rs"));
use neocogi_com::{com_object, NcStatus};
#[com_object(Counter)]
/// Concrete provider for a generated counter call in an archive-only assembly.
struct Value;
impl Counter for Value {
    fn read(&self) -> Result<u32, NcStatus> { Ok(42) }
}
#[neocogi_com::plugin(name = "proof.archive.provider")]
/// Acquire the consumer's provider through the generated runtime contract.
fn acquire(_runtime: neocogi_com::RuntimeHandle) -> Result<neocogi_com::ComPtr<CounterAbi>, NcStatus> {
    Value::from_value(Value).query()
}
/// Select exactly one linked or native host from extracted SDK dependencies.
fn host_runtime() -> neocogi_com::RuntimeHandle {
    #[cfg(feature = "static-host")]
    { neocogi_com_host::initialize_runtime() }
    #[cfg(not(feature = "static-host"))]
    {
        let path = std::env::args_os().nth(1).expect("explicit packaged host image");
        // SAFETY: the driver built this exact SDK shim from extracted siblings;
        // the SDK pins its complete immutable C graph for process lifetime.
        unsafe { neocogi_com::RuntimeHandle::load_host_library(path) }.unwrap()
    }
}

/// Register and execute real COM calls under either selected host composition.
fn main() {
    let runtime = host_runtime();
    assert_eq!(runtime.plugins().unwrap().len(), 0);
    // SAFETY: this extracted-only consumer links its complete provider graph.
    unsafe { runtime.register_plugin_descriptor(plugin_descriptor()) }.unwrap();
    let counter = runtime.acquire_plugin_root::<CounterAbi>(plugin_descriptor().plugin_id).unwrap();
    assert_eq!(counter.read(), Ok(42));
    assert_eq!(neocogi_com::NC_COM_RUNTIME_ABI_VERSION, 3);
    assert_eq!(neocogi_com::NC_PLUGIN_DESCRIPTOR_ABI_VERSION, 3);
    assert_eq!(host_runtime().plugins().unwrap().len(), 1);
    assert_eq!(counter.read(), Ok(42));
    println!("archive host: value=42; plugins=1");
}
