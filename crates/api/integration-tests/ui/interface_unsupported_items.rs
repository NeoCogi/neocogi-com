use neocogi_com::com_interface;

#[allow(unused_macros)]
macro_rules! add_method {
    () => {
        fn generated(&self) -> Result<u32, neocogi_com::NcStatus>;
    };
}

#[com_interface(auto_guid = "nc.tests.ui.UnsupportedItems:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IUnsupportedItems {
    type Value;
    const VERSION: u32;
    add_method!();

    fn value(&self) -> Result<u32, neocogi_com::NcStatus>;
}

fn main() {}
