use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.ComRefStaticLifetime.Object:v1")]
pub trait IObject {
    fn id(&self) -> Result<u32, neocogi_com::NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.ui.ComRefStaticLifetime:v1")]
pub trait IComRefStaticLifetime {
    fn retain(
        &self,
        object: neocogi_com::ComRef<'static, IObjectAbi>,
    ) -> Result<(), neocogi_com::NcStatus>;
}

fn main() {}
