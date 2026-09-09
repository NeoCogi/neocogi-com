use neocogi_com::com_interface;

pub trait Base {}

#[com_interface(auto_guid = "nc.tests.ui.Supertrait:v1")]
pub trait ISupertrait: Base + Send {
    fn value(&self) -> Result<u32, neocogi_com::NcStatus>;
}

fn main() {}
