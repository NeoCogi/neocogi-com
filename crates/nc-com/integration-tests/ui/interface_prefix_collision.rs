use nc_com::{com_interface, NcStatus};
#[com_interface(auto_guid = "nc.tests.ui.PrefixBase:v1")]
pub trait IBase {}
#[com_interface(auto_guid = "nc.tests.ui.PrefixDerived:v1")]
pub trait IDerived: IBase {
    fn base(&self) -> Result<(), NcStatus>;
}
#[com_interface(auto_guid = "nc.tests.ui.PrefixRoot:v1")]
pub trait IRoot {
    fn unknown(&self) -> Result<(), NcStatus>;
}
fn main() {}
