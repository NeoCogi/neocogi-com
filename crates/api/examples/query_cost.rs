//! Controlled five-interface query/ownership probe; no rendering or application work.
//! Run with `cargo run --release -p neocogi-com --example query_cost`.
//! Each line reports integer nanoseconds per successful query plus balanced release;
//! discard the first warmup line and compare the remaining samples on the same host.
use neocogi_com::{com_interface, com_object, ComInterface, IUnknown, NcInterfaceId, NC_OK};
use std::{hint::black_box, time::Instant};

#[com_interface(auto_guid = "probe.Widget", assigned_signature = b"00000000000000000000000000000001")]
/// First independent capability, matching a normal leaf's ancestry shape.
trait IWidget {}
#[com_interface(auto_guid = "probe.Layout", assigned_signature = b"00000000000000000000000000000002")]
/// Second independent capability.
trait ILayout {}
#[com_interface(auto_guid = "probe.Events", assigned_signature = b"00000000000000000000000000000003")]
/// Third independent capability.
trait IEvents {}
#[com_interface(auto_guid = "probe.Leaf", assigned_signature = b"00000000000000000000000000000004")]
/// Concrete leaf contract with no repeated ancestors.
trait ILeaf: IWidget + ILayout + IEvents {}
#[com_object(interfaces(ILeaf))]
/// State is irrelevant to query dispatch and deliberately empty.
struct Leaf;
impl IWidget for Leaf {}
impl ILayout for Leaf {}
impl IEvents for Leaf {}
impl ILeaf for Leaf {}

/// Times rotating successful queries with exactly balanced ownership transfers.
fn main() {
    let object = Leaf::from_value(Leaf);
    let identities = [
        IUnknown::INTERFACE_ID,
        ILeafAbi::INTERFACE_ID,
        IWidgetAbi::INTERFACE_ID,
        ILayoutAbi::INTERFACE_ID,
        IEventsAbi::INTERFACE_ID,
    ];
    let source = black_box(object.as_raw());
    // Repeated runs expose scheduling variation; ignore the first warmup sample.
    for _ in 0..6 {
        let start = Instant::now();
        for _ in 0..200_000 {
            for identity in &identities {
                let mut out = neocogi_com::ComRaw::null();
                // SAFETY: the owning handle retains source; identities are static
                // generated contracts and every successful result is released once.
                unsafe {
                    assert_eq!(
                        ((*source.vtable).query_interface)(source.object, black_box(identity as *const NcInterfaceId), &mut out),
                        NC_OK
                    );
                    let view = out.cast::<IUnknown>();
                    ((*view.vtable).release)(view.object);
                }
            }
        }
        println!("{}", start.elapsed().as_nanos() / 1_000_000);
    }
}
