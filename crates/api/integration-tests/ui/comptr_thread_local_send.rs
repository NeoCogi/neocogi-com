use neocogi_com::{ComPtr, IUnknown, NcStatus, com_object};

extern crate self as fixture_ui_comptr_thread_local_send;
neocogi_com_test_contracts::ui_comptr_thread_local_send!();

#[com_object(interfaces(IThreadLocalSend))]
pub struct ThreadLocalObject;

impl IThreadLocalSend for ThreadLocalObject {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(1)
    }
}

fn main() {
    let unknown: ComPtr<IUnknown> = ThreadLocalObject::from_value(ThreadLocalObject);
    std::thread::spawn(move || drop(unknown));
}
