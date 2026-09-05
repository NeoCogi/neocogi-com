use nc_com::{ComPtr, IUnknown, NcStatus, com_class, com_interface};

#[com_interface(auto_guid = "nc.tests.ui.ThreadLocalSend:v1")]
pub trait IThreadLocalSend {
    fn value(&self) -> Result<u32, NcStatus>;
}

#[com_class(interfaces(IThreadLocalSend))]
pub struct ThreadLocalObject;

impl IThreadLocalSend for ThreadLocalObject {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(1)
    }
}

fn main() {
    let unknown: ComPtr<IUnknown> = ThreadLocalObject::create();
    std::thread::spawn(move || drop(unknown));
}
