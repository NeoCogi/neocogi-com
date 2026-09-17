use neocogi_com::{com_factory, com_interface};

#[com_interface(auto_guid = "nc.tests.ui.NotFactory:v1", assigned_signature = b"00000000000000000000000000000002")]
/// Ordinary interfaces cannot accidentally become plugin roots.
trait IObject {}

#[com_factory(IObject)]
/// The implementation attribute must prove the contract's factory marker.
struct InvalidFactory;

impl IObject for InvalidFactory {}

fn main() {}
