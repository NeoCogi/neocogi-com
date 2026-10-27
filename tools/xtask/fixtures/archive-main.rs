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
fn acquire(_runtime: neocogi_com::Runtime) -> Result<neocogi_com::ComPtr<CounterAbi>, NcStatus> {
    Value::from_value(Value).query()
}
#[neocogi_com::com_main]
/// Register and execute a real COM call with only extracted SDK dependencies.
fn main(runtime: neocogi_com::Runtime) {
    // SAFETY: this extracted-only consumer links its complete provider graph.
    unsafe { runtime.register_plugin_descriptor(plugin_descriptor()) }.unwrap();
    let counter = runtime.acquire_plugin_root::<CounterAbi>(plugin_descriptor().plugin_id).unwrap();
    assert_eq!(counter.read(), Ok(42));
    assert_eq!(neocogi_com::NC_COM_RUNTIME_ABI_VERSION, 1);
    assert_eq!(neocogi_com::NC_PLUGIN_DESCRIPTOR_ABI_VERSION, 1);
}
