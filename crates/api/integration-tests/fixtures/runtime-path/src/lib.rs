pub mod facade {
    pub use renamed_nc::*;
}

use renamed_nc::{ComPtr, IUnknown, NC_E_INVALID_TAG, NcStatus};

#[renamed_nc::com_interface(
    neocogi_com = crate::facade,
    auto_guid = "nc.tests.renamed.Override:v1",
    assigned_signature = b"00000000000000000000000000000037"
)]
pub trait IOverrideResolved {
    fn value(&self) -> Result<u16, crate::facade::NcStatus>;
}

#[renamed_nc::com_object(neocogi_com = crate::facade, thread_safe, interfaces(IOverrideResolved))]
pub struct OverrideResolved;

impl IOverrideResolved for OverrideResolved {
    fn value(&self) -> Result<u16, crate::facade::NcStatus> {
        Ok(42)
    }
}

#[renamed_nc::com_enum(
    neocogi_com = crate::facade,
    tag = u16,
    assigned_signature = b"00000000000000000000000000000038"
)]
#[derive(Debug, PartialEq)]
pub enum OverrideEvent {
    Ready(u32),
}

#[renamed_nc::com_struct(neocogi_com = crate::facade, assigned_signature = b"00000000000000000000000000000039")]
#[derive(Debug, PartialEq)]
pub struct OverridePoint {
    pub x: u16,
    pub y: u16,
}

pub fn exercise_explicit_override_path() -> Result<u16, NcStatus> {
    let unknown: ComPtr<IUnknown, renamed_nc::ThreadSafe> = OverrideResolved::from_value(OverrideResolved);
    let resolved = unknown.query::<IOverrideResolvedAbi>()?;

    resolved.value()
}

pub fn exercise_enum_override_path() -> Result<(), NcStatus> {
    let wire = renamed_nc::encode_inline(&OverrideEvent::Ready(7))?;
    // SAFETY: valid inline storage emitted by the matching codec.
    let value: OverrideEvent = unsafe { renamed_nc::decode_inline(wire) }?;

    if value == OverrideEvent::Ready(7) { Ok(()) } else { Err(NC_E_INVALID_TAG) }
}

pub fn exercise_struct_override_path() -> Result<u16, NcStatus> {
    let value = OverridePoint { x: 5, y: 6 };
    Ok(value.x + value.y)
}
