#[allow(unused_imports, unused_macros)]
mod test_contracts { include!(concat!(env!("OUT_DIR"), "/test_contracts.rs")); }
use neocogi_com::{com_object, NcStatus, NcValueRef};

extern crate self as fixture_ui_interface_value_ref_escape;
test_contracts::ui_interface_value_ref_escape!();

/// Deliberately attempts to keep the receiver's borrowed guard permanently.
#[com_object(IValueReceiver)]
struct Receiver {
    /// This field must never accept a guard reconstructed for a single call.
    saved: Option<NcValueRef<'static, Message>>,
}

impl IValueReceiver for Receiver {
    /// The macro accepts the contract, then Rust must reject this implementation.
    fn retain(&mut self, value: NcValueRef<'_, Message>) -> Result<(), NcStatus> {
        self.saved = Some(value);
        Ok(())
    }
}

/// The regression is compile-only; no expired wire storage is accessed.
fn main() {}
