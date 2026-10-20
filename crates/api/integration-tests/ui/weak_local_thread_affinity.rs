use neocogi_com::{com_object, NcStatus, WeakLocalCom};

extern crate self as fixture_ui_weak_local_thread_affinity;
neocogi_com_test_contracts::ui_weak_local_thread_affinity!();
#[com_object(ILocal)]
struct Local;
impl ILocal for Local {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(1)
    }
}

fn send<T: Send>() {}
fn sync<T: Sync>() {}
fn main() {
    send::<WeakLocalCom<Local>>();
    sync::<WeakLocalCom<Local>>();
}
