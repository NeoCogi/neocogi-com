//! Leaf provider receives the runtime originally selected by the application.
extern crate self as linked_host_b;
/// Independently generated shared contract bindings.
pub mod contracts {
    include!(env!("LINKED_HOST_BINDINGS"));
}
use neocogi_com::{ComPtr, NcStatus, RuntimeHandle, com_object, guid_from_qualified_name};
#[com_object(contracts::ICounter)]
/// Private leaf implementation remains in this provider image.
struct Leaf;
impl contracts::ICounter for Leaf {
    /// Real vtable dispatch returns the leaf's value.
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(41)
    }
}
/// Bootstrap must see A's registration through the original host's catalog.
#[neocogi_com::plugin(name = "nc.tests.linked_host.b", version = "1.0.0")]
fn acquire(runtime: RuntimeHandle) -> Result<ComPtr<contracts::ICounterAbi>, NcStatus> {
    if runtime.plugin_info(guid_from_qualified_name("nc.tests.linked_host.a"))?.is_none() {
        return Err(neocogi_com::NC_E_NOT_FOUND);
    }
    Leaf::from_value(Leaf).query()
}
