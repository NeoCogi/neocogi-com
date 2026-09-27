//! Linked-binary proof that using COM does not implicitly retain inspection schemas.
use neocogi_com::{com_interface, com_object, ComInterface, NcStatus};
use std::hint::black_box;

#[com_interface(
    auto_guid = "test.projection.MetadataRetentionSentinelBase:v1",
    assigned_signature = b"00000000000000000000000000000001"
)]
/// Shared base whose schema name must disappear unless inspection is requested.
trait IBase {
    /// The method name is another marker which dispatch itself does not need.
    fn metadata_retention_sentinel_method(&self) -> Result<u32, NcStatus>;
}
#[com_interface(
    auto_guid = "test.projection.MetadataRetentionSentinelLeft:v1",
    assigned_signature = b"00000000000000000000000000000002"
)]
/// First inheritance path to the common base.
trait ILeft: IBase {}
#[com_interface(
    auto_guid = "test.projection.MetadataRetentionSentinelRight:v1",
    assigned_signature = b"00000000000000000000000000000003"
)]
/// Second inheritance path must resolve to the same physical base header.
trait IRight: IBase {}
#[com_interface(
    auto_guid = "test.projection.MetadataRetentionSentinelTop:v1",
    assigned_signature = b"00000000000000000000000000000004"
)]
/// Top of a diamond with four unique authored contracts.
trait ITop: ILeft + IRight {}
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
