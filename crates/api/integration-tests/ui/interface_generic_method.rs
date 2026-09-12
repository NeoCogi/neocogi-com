use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.GenericMethod:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IGenericMethod {
    fn value<T>(&self, value: T) -> Result<T, neocogi_com::NcStatus>;
}

fn main() {}
