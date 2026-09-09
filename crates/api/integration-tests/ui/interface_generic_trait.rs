use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.GenericTrait:v1")]
pub trait IGenericTrait<T> {
    fn value(&self, value: T) -> Result<T, neocogi_com::NcStatus>;
}

fn main() {}
