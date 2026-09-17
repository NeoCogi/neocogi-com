use neocogi_com::{ComPtr, IUnknown, NcStatus, com_object, com_interface};

#[com_interface(auto_guid = "nc.tests.ui.ThreadLocalSend:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IThreadLocalSend {
    fn value(&self) -> Result<u32, NcStatus>;
}

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
