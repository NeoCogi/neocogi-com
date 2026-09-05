use nc_com::com_interface;

pub trait Base {}

#[com_interface(auto_guid = "nc.tests.ui.Supertrait:v1")]
pub trait ISupertrait: Base {
    fn value(&self) -> Result<u32, nc_com::NcStatus>;
}

fn main() {}
