use neocogi_com::{com_interface, com_object};

#[com_interface(auto_guid = "nc.tests.ui.Object:v1", assigned_signature = b"00000000000000000000000000000001")]
/// An ordinary callable contract has no public default-construction policy.
trait IObject {}

#[com_object(IObject)]
/// Even a unit implementation must be passed explicitly to `from_value`.
struct Object;

impl IObject for Object {}

fn main() {
    let _ = Object::create();
}
