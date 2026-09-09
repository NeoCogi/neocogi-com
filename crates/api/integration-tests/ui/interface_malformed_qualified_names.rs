use neocogi_com::com_interface;

#[com_interface(auto_guid = ".nc.tests.ui.LeadingSeparator:v1")]
trait ILeadingSeparator {}

#[com_interface(auto_guid = "nc.tests..ui.ConsecutiveSeparators:v1")]
trait IConsecutiveSeparators {}

#[com_interface(auto_guid = "nc.tests.ui.TrailingSeparator:v1:")]
trait ITrailingSeparator {}

fn main() {}
