use neocogi_com::{ComInterface, IUNKNOWN_INTERFACE_ID, IUnknownVTable, ThreadLocal};

// A marker cannot assert the raw COM contract without an unsafe implementation.
enum ManualInterface {}

impl ComInterface for ManualInterface {
    const INTERFACE_ID: neocogi_com::NcInterfaceId = IUNKNOWN_INTERFACE_ID;
    type VTable = IUnknownVTable;
    type Threading = ThreadLocal;
}

fn main() {}
