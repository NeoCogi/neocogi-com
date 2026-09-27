//! The same IID cannot select incompatible vtables through two ancestry paths.
use neocogi_com::{com_interface, com_object};

#[com_interface(assigned_guid = "eb682f03-f4b7-4403-97a5-eb66ec6e4810", name = "test.conflict.Left", assigned_signature = b"00000000000000000000000000000001")]
/// First revision of a shared nominal interface.
trait ILeft {}
#[com_interface(assigned_guid = "eb682f03-f4b7-4403-97a5-eb66ec6e4810", name = "test.conflict.Right", assigned_signature = b"00000000000000000000000000000002")]
/// An incompatible revision cannot be hidden by deduplication.
trait IRight {}
#[com_interface(auto_guid = "test.conflict.Top:v1", assigned_signature = b"00000000000000000000000000000003")]
/// Canonical ancestry evaluation must reject the conflicting parent revisions.
trait ITop: ILeft + IRight {}

#[com_object(ITop)]
/// Never reaches runtime construction because its layout contract is inconsistent.
struct Object;
impl ILeft for Object {}
impl IRight for Object {}
impl ITop for Object {}

/// Forces the same construction path used by real providers.
fn main() { let _ = Object::from_value(Object); }
