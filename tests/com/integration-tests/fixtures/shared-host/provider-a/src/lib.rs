//! Nested provider routes acquisition through its supplied host tables.
extern crate self as shared_host_a;
/// Independently generated shared contract bindings.
pub mod contracts {
    include!(env!("SHARED_HOST_BINDINGS"));
}
use neocogi_com::{ComPtr, NcStatus, RuntimeHandle, com_object, guid_from_qualified_name};
/// Hard provider identity; B must already be registered before A is loaded.
const B_ID: neocogi_com::NcGuid = guid_from_qualified_name("nc.tests.shared_host.b");
#[com_object(contracts::ICounter)]
/// Service retains access to the one host without owning a catalog.
struct Nested {
    /// All later nested acquisitions use the same host as this bootstrap.
    runtime: RuntimeHandle,
}
impl contracts::ICounter for Nested {
    /// Acquire and call B while A's own generated ABI call remains active.
    fn value(&self) -> Result<u32, NcStatus> {
        let leaf = self.runtime.acquire_plugin_root::<contracts::ICounterAbi>(B_ID)?;
        Ok(leaf.value()? + 1)
    }
}
/// Acquiring A immediately enters B's bootstrap through the selected host.
#[neocogi_com::plugin(name = "nc.tests.shared_host.a", version = "1.0.0", dependencies = [B_ID])]
fn acquire(runtime: RuntimeHandle) -> Result<ComPtr<contracts::ICounterAbi>, NcStatus> {
    let leaf = runtime.acquire_plugin_root::<contracts::ICounterAbi>(B_ID)?;
    assert_eq!(leaf.value()?, 41);
    Nested::from_value(Nested { runtime }).query()
}
