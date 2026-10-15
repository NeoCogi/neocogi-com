use neocogi_com::{com_object, com_interface};

#[com_interface(auto_guid = "nc.tests.ui.NotFactory:v1", assigned_signature = b"00000000000000000000000000000002")]
/// Ordinary interfaces need no construction role to have an implementation.
trait IObject {}

#[com_object(IObject)]
/// The same object macro supports services and construction providers.
struct Object;

impl IObject for Object {}

fn main() {}
