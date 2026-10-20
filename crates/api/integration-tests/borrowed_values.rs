//! Real generated COM calls may retain acquired values, never borrowed wire guards.

use neocogi_com::{com_object, ComPtr, NcStatus, NcValueRef};
use std::sync::LazyLock;

extern crate self as fixture_borrowed_values;
neocogi_com_test_contracts::borrowed_values!();

/// Mutable provider state stores the application value without shared mutation.
#[com_object(IValueReceiver)]
struct Receiver {
    /// None before the first successful acquisition; subsequent calls replace it.
    saved: Option<Message>,
}

impl IValueReceiver for Receiver {
    /// Decode before publishing so the only retained state owns its backing data.
    fn retain(&mut self, value: NcValueRef<'_, Message>) -> Result<(), NcStatus> {
        self.saved = Some(value.to_owned()?);
        Ok(())
    }

    /// Compare owned values without handing internal storage back across the ABI.
    fn matches(&self, expected: NcValueRef<'_, Message>) -> Result<bool, NcStatus> {
        let expected = expected.to_owned()?;
        Ok(self.saved.as_ref().is_some_and(|saved| saved.entries == expected.entries))
    }
}

/// Construct a real COM object so calls cross generated clients and entry thunks.
fn receiver() -> ComPtr<IValueReceiverAbi> {
    Receiver::from_value(Receiver { saved: None }).query::<IValueReceiverAbi>().unwrap()
}

/// Acquired nested strings survive both guard destruction and source mutation/drop.
#[test]
fn owned_message_outlives_producer_guard_and_source() {
    let mut receiver = receiver();
    let expected = Message {
        entries: vec!["first".into(), "second".into()],
    };
    assert!(!receiver.matches(NcValueRef::new(&expected).unwrap()).unwrap());
    {
        let mut source = expected.clone();
        receiver.retain(NcValueRef::new(&source).unwrap()).unwrap();
        // The call has consumed its guard. Mutating and dropping the source must
        // not change either the receiver's sequence or its nested string storage.
        source.entries[0].replace_range(.., "changed");
        source.entries.clear();
    }
    assert!(receiver.matches(NcValueRef::new(&expected).unwrap()).unwrap());
}

/// Static application data still needs temporary producer wire storage per call.
#[test]
fn static_source_is_allowed_without_granting_a_static_receiver_borrow() {
    /// The application data lives forever; each NcValueRef still owns a fresh wire array.
    static SOURCE: LazyLock<Message> = LazyLock::new(|| Message { entries: vec!["permanent text".into()] });
    let mut receiver = receiver();
    // The restriction belongs to COM parameter declarations. A producer guard
    // may carry a static source lifetime while it owns temporary wire storage.
    let guard: NcValueRef<'static, Message> = NcValueRef::new(&*SOURCE).unwrap();
    receiver.retain(guard).unwrap();
    assert!(receiver.matches(NcValueRef::new(&*SOURCE).unwrap()).unwrap());
}

/// Empty and repeated calls each acquire a complete replacement independent of prior guards.
#[test]
fn empty_and_repeated_messages_replace_owned_state() {
    let mut receiver = receiver();
    for entries in [vec![], vec![String::new()], vec!["one".into(), "two".into()], vec![]] {
        let expected = Message { entries };
        {
            let source = expected.clone();
            receiver.retain(NcValueRef::new(&source).unwrap()).unwrap();
        }
        assert!(receiver.matches(NcValueRef::new(&expected).unwrap()).unwrap());
    }
}
