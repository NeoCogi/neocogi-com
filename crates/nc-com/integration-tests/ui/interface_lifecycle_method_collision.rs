use nc_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.LifecycleMethodCollision:v1")]
trait ILifecycleMethodCollision {
    fn query_interface(&self) -> Result<(), nc_com::NcStatus>;
    fn add_ref(&self) -> Result<(), nc_com::NcStatus>;
    fn release(&self) -> Result<(), nc_com::NcStatus>;
}

fn main() {}
