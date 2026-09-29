use neocogi_com::{com_interface, com_object, NcStatus, NcValueRef};

/// A real named value whose encoding needs a temporary array of string wires.
#[neocogi_com::com_value(assigned_signature = b"00000000000000000000000000000091")]
pub struct Message {
    /// The producer retains text while its guard owns the converted sequence.
    pub entries: Vec<String>,
}

/// The supported declaration lends the value only for the current call.
#[com_interface(auto_guid = "nc.tests.ui.ValueRef.Escape:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IValueReceiver {
    /// Retention requires an owned value acquired during this method.
    fn retain(&mut self, value: NcValueRef<'_, Message>) -> Result<(), NcStatus>;
}

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
