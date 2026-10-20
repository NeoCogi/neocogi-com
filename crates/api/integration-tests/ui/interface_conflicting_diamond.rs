//! The same IID cannot select incompatible vtables through two ancestry paths.
use neocogi_com::{com_object};

extern crate self as fixture_ui_interface_conflicting_diamond;
neocogi_com_test_contracts::ui_interface_conflicting_diamond!();
extern crate self as fixture_ui_conflicting_right;
neocogi_com_test_contracts::ui_conflicting_right!();

#[com_object(interfaces(ILeft, IRight))]
/// Never reaches runtime construction because its layout contract is inconsistent.
struct Object;
impl ILeft for Object {}
impl IRight for Object {
    fn changed(&self) -> Result<(), neocogi_com::NcStatus> {
        Ok(())
    }
}

/// Forces the same construction path used by real providers.
fn main() {
    let _ = Object::from_value(Object);
}
