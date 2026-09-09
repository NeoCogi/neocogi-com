use neocogi_com::{ComInterface, IUNKNOWN_INTERFACE_ID, IUnknown, IUnknownVTable};

#[repr(C)]
struct ManualInterface {
    vtable: *const IUnknownVTable,
    object: *mut core::ffi::c_void,
}

impl ComInterface for ManualInterface {
    const INTERFACE_ID: neocogi_com::NcInterfaceId = IUNKNOWN_INTERFACE_ID;

    type VTable = IUnknownVTable;

    fn iunknown(this: *mut Self) -> *mut IUnknown {
        this.cast::<IUnknown>()
    }
}

fn main() {}
