use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.Café:v1")]
trait INonAsciiQualifiedName {}

fn main() {}
