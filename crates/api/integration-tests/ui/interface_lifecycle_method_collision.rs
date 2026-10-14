use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.LifecycleMethodCollision:v1", assigned_signature = b"99999999999999999999999999999999")]
trait ILifecycleMethodCollision {
    fn query_interface(&self) -> Result<(), neocogi_com::NcStatus>;
    fn add_ref(&self) -> Result<(), neocogi_com::NcStatus>;
    fn release(&self) -> Result<(), neocogi_com::NcStatus>;
    fn r#downgrade(&self) -> Result<neocogi_com::WeakComPtr<ILifecycleMethodCollisionAbi>, neocogi_com::NcStatus>;
    fn clone(&self) -> Result<u32, neocogi_com::NcStatus>;
    fn as_raw(&self) -> Result<u32, neocogi_com::NcStatus>;
    fn query(&self) -> Result<u32, neocogi_com::NcStatus>;
}

fn main() {}
