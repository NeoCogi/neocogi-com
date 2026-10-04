use neocogi_com::{com_interface, com_object, NcStatus, WeakLocalCom};

#[com_interface(auto_guid = "nc.tests.weak_local.Local:v1", assigned_signature = b"00000000000000000000000000000063")]
trait ILocal { fn value(&self) -> Result<u32, NcStatus>; }
#[com_object(ILocal)]
struct Local;
impl ILocal for Local { fn value(&self) -> Result<u32, NcStatus> { Ok(1) } }

fn send<T: Send>() {}
fn sync<T: Sync>() {}
fn main() {
    send::<WeakLocalCom<Local>>();
    sync::<WeakLocalCom<Local>>();
}
