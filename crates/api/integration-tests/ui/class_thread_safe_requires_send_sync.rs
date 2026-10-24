#[allow(unused_imports, unused_macros)]
mod test_contracts { include!(concat!(env!("OUT_DIR"), "/test_contracts.rs")); }
use neocogi_com::{NcStatus, com_object};
use std::rc::Rc;

extern crate self as fixture_ui_class_thread_safe_requires_send_sync;
test_contracts::ui_class_thread_safe_requires_send_sync!();

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
