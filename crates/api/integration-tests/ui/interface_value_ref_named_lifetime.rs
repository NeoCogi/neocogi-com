use neocogi_com::com_interface;

/// A real named value whose encoding needs a temporary array of string wires.
#[neocogi_com::com_value(assigned_signature = b"00000000000000000000000000000091")]
pub struct Message {
    /// The producer retains text while its guard owns the converted sequence.
    pub entries: Vec<String>,
}

/// A borrowed value cannot declare that producer storage survives the call.
#[com_interface(auto_guid = "nc.tests.ui.ValueRef.named_lifetime:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IValueReceiver {
    /// Deliberately invalid lifetime on a synchronous borrowed input.
    fn retain(&mut self, value: neocogi_com::NcValueRef<'borrowed, Message>) -> Result<(), neocogi_com::NcStatus>;
}

/// Compilation must fail before any invalid borrowed value could be used.
fn main() {}
