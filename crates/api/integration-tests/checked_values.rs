//! Restricted nested fields cross real generated COM slots only as checked wire values.
use neocogi_com::*;

#[com_struct(assigned_signature = b"00000000000000000000000000000081")]
#[derive(Debug, PartialEq)]
/// A plain record containing a restricted scalar.
pub struct Flags {
    /// A byte on the wire and a bool only after validation.
    pub enabled: bool,
}
#[com_struct(assigned_signature = b"00000000000000000000000000000082")]
#[derive(Debug, PartialEq)]
/// Nested ordinary records must recursively lower their fields.
pub struct Settings {
    /// Validation cannot stop at this named type boundary.
    pub flags: Flags,
}
#[com_enum(tag = u8, assigned_signature = b"00000000000000000000000000000083")]
#[derive(Clone, Copy, Debug, PartialEq)]
/// Enum payload uses the same nested record converter.
pub enum Action {
    #[com_tag(4)]
    /// Selects a nested checked payload.
    Configure(Settings),
    #[com_tag(8)]
    /// Requires no initialized payload.
    Idle,
}
#[com_interface(auto_guid = "nc.tests.values.Check:v1", assigned_signature = b"00000000000000000000000000000084")]
/// Real copied, enum, and buffer slots for adversarial raw callers.
pub trait ICheck {
    /// Returns the input after the thunk validates it.
    fn settings(&self, value: Settings) -> Result<Settings, NcStatus>;
    /// The enum's application wrapper never appears in a raw slot.
    fn action(&self, value: ActionAbi) -> Result<ActionAbi, NcStatus>;
    /// Nested restricted fields are checked before reconstructing a typed slice.
    fn input(&self, value: NcCopyFromBufferOffset<'_, Settings>) -> Result<u32, NcStatus>;
    /// Writable buffer elements use the same bidirectional conversion.
    fn output(&self, value: NcCopyToBufferOffset<'_, Settings>) -> Result<(), NcStatus>;
}
#[com_object(ICheck)]
/// Stateless implementation; errors must originate before method dispatch.
struct Check;
impl ICheck for Check {
    fn settings(&self, value: Settings) -> Result<Settings, NcStatus> {
        Ok(value)
    }
    fn action(&self, value: ActionAbi) -> Result<ActionAbi, NcStatus> {
        Ok(value)
    }
    fn input(&self, value: NcCopyFromBufferOffset<'_, Settings>) -> Result<u32, NcStatus> {
        Ok(value.len() as u32)
    }
    fn output(&self, mut value: NcCopyToBufferOffset<'_, Settings>) -> Result<(), NcStatus> {
        for value in value.as_mut_slice() {
            value.flags.enabled = true;
        }
        Ok(())
    }
}

#[test]
fn invalid_nested_bool_fails_copied_and_buffer_thunks() {
    let check = Check::from_value(Check).query::<ICheckAbi>().unwrap();
    let ptr = check.as_mut_ptr();
    let wire = __SettingsValueWire { flags: __FlagsValueWire { enabled: 2 } };
    let mut out = core::mem::MaybeUninit::uninit();
    // SAFETY: all raw storage is initialized, aligned and live; invalid bytes
    // deliberately test conversion before a restricted Rust value is created.
    unsafe {
        assert_eq!(((*(*ptr).vtable).settings)(ptr, wire, out.as_mut_ptr()), NC_E_INVALID_ARG);
        let mut count = 0;
        assert_eq!(((*(*ptr).vtable).input)(ptr, 0, 1, &wire, &mut count), NC_E_INVALID_ARG);
        let mut destination = wire;
        assert_eq!(((*(*ptr).vtable).output)(ptr, 0, 1, &mut destination), NC_E_INVALID_ARG);
    }
}

#[test]
fn enum_tag_is_checked_before_payload_and_nested_field_is_checked_afterward() {
    let check = Check::from_value(Check).query::<ICheckAbi>().unwrap();
    let ptr = check.as_mut_ptr();
    let bad_field = __ActionValueWire {
        tag: 4,
        payload: __ActionValuePayload {
            case_0: __ActionValueCase0 {
                field_0: __SettingsValueWire { flags: __FlagsValueWire { enabled: 3 } },
            },
        },
    };
    let bad_tag = __ActionValueWire {
        tag: 99,
        payload: __ActionValuePayload { case_1: __ActionValueCase1 { empty: 0 } },
    };
    let mut out = core::mem::MaybeUninit::uninit();
    unsafe {
        assert_eq!(((*(*ptr).vtable).action)(ptr, bad_field, out.as_mut_ptr()), NC_E_INVALID_ARG);
        assert_eq!(((*(*ptr).vtable).action)(ptr, bad_tag, out.as_mut_ptr()), NC_E_INVALID_TAG);
    }
}

#[test]
fn valid_calls_acquire_output_buffers_and_nested_return_values() {
    let check = Check::from_value(Check).query::<ICheckAbi>().unwrap();
    let settings = Settings { flags: Flags { enabled: false } };
    assert_eq!(check.settings(settings).unwrap(), settings);
    let mut values = [settings];
    check.output(NcCopyToBufferOffset::new(0, &mut values)).unwrap();
    assert!(values[0].flags.enabled);
    assert_eq!(check.input(NcCopyFromBufferOffset::new(0, &values)).unwrap(), 1);
}
