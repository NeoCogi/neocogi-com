use nc_com::{NcStatus, com_object, com_interface};
use std::rc::Rc;

#[com_interface(auto_guid = "nc.tests.ui.ThreadSafeRequiresSendSync:v1")]
pub trait IThreadSafeRequiresSendSync {
    fn value(&self) -> Result<u32, NcStatus>;
}

#[com_object(thread_safe, interfaces(IThreadSafeRequiresSendSync))]
pub struct NotActuallyThreadSafe {
    value: Rc<u32>,
}

impl IThreadSafeRequiresSendSync for NotActuallyThreadSafe {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(*self.value)
    }
}

fn main() {}
