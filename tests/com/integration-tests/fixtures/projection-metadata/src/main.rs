//! Linked-binary proof that using COM does not implicitly retain inspection schemas.
use neocogi_com::{com_object, ComInterface, NcStatus};
use std::hint::black_box;

#[allow(dead_code, unused_imports)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/contracts.rs"));
}
use bindings::*;
use bindings::{
    MetadataRetentionSentinelBase as IBase, MetadataRetentionSentinelBaseAbi as IBaseAbi, MetadataRetentionSentinelBaseVtblFor as IBaseVtblFor,
    MetadataRetentionSentinelLeft as ILeft, MetadataRetentionSentinelLeftVtblFor as ILeftVtblFor, MetadataRetentionSentinelRight as IRight,
    MetadataRetentionSentinelRightVtblFor as IRightVtblFor, MetadataRetentionSentinelTop as ITop, MetadataRetentionSentinelTopAbi as ITopAbi,
    MetadataRetentionSentinelTopVtblFor as ITopVtblFor,
};

#[com_object(interfaces(ITop))]
/// Concrete behavior reached through generated typed vtables.
struct Probe;
impl IBase for Probe {
    /// Returns an observable value so the executable exercises real COM dispatch.
    fn metadata_retention_sentinel_method(&self) -> Result<u32, NcStatus> {
        Ok(black_box(42))
    }
}
impl ILeft for Probe {}
impl IRight for Probe {}
impl ITop for Probe {}

/// Exercises construction, identity negotiation, method dispatch and final release.
fn main() {
    let object = Probe::from_value(Probe);
    let top = black_box(object).query::<ITopAbi>().unwrap();
    let base = top.query::<IBaseAbi>().unwrap();
    println!("value={}", base.metadata_retention_sentinel_method().unwrap());
    // Checking only compact identity information must not retain schema strings.
    black_box(IBaseAbi::INTERFACE_ID);
    #[cfg(feature = "inspect")]
    print_inspection();
}

/// Explicit inspection must retain every unique contract and its method descriptions.
#[cfg(feature = "inspect")]
fn print_inspection() {
    // Hiding the table from constant propagation exercises actual runtime reads.
    for descriptor in black_box(&ITopAbi::INTERFACE_CHAIN) {
        // SAFETY: descriptors and their method/text views are generated static data.
        unsafe {
            println!("{}", descriptor.qualified_name.as_str().unwrap());
            for index in 0..descriptor.method_count as usize {
                let method = &*descriptor.methods.add(index);
                println!("{}", method.name.as_str().unwrap());
            }
        }
    }
}
