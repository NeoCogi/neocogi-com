use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.GenericMethod:v1")]
pub trait IGenericMethod {
    fn value<T>(&self, value: T) -> Result<T, neocogi_com::NcStatus>;
}

fn main() {}
