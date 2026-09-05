use nc_com::com_interface;

#[com_interface(
    "00000000-0000-5000-8000-000000000000",
    name = "nc.tests.ui.ExplicitGuidMismatch:v1"
)]
trait IExplicitGuidMismatch {}

fn main() {}
