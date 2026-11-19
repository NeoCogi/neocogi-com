include!(concat!(env!("OUT_DIR"), "/proof_archive.include.rs"));
use neocogi_com_sdk::{com_object, NcStatus};
#[com_object(Counter)]
/// Concrete provider for a generated counter call in an archive-only assembly.
struct Value;
impl Counter for Value {
    fn read(&self) -> Result<u32, NcStatus> { Ok(42) }
}
#[neocogi_com_sdk::plugin(name = "proof.archive.provider")]
/// Acquire the consumer's provider through the generated runtime contract.
fn acquire(_runtime: neocogi_com_sdk::RuntimeHandle) -> Result<neocogi_com_sdk::ComPtr<CounterAbi>, NcStatus> {
    Value::from_value(Value).query()
}
/// Register and execute real COM calls through the linked host.
fn main() {
    let runtime = neocogi_com_host::initialize_runtime();
    assert_eq!(runtime.plugins().unwrap().len(), 0);
    // SAFETY: this extracted-only consumer links its complete provider graph.
    unsafe { runtime.register_plugin_descriptor(plugin_descriptor()) }.unwrap();
    let counter = runtime.acquire_plugin_root::<CounterAbi>(plugin_descriptor().plugin_id).unwrap();
    assert_eq!(counter.read(), Ok(42));
    assert_eq!(neocogi_com_sdk::NC_COM_RUNTIME_ABI_VERSION, 3);
    assert_eq!(neocogi_com_sdk::NC_PLUGIN_DESCRIPTOR_ABI_VERSION, 3);
    assert_eq!(neocogi_com_host::initialize_runtime().plugins().unwrap().len(), 1);
    assert_eq!(counter.read(), Ok(42));
    println!("archive host: value=42; plugins=1");
}
