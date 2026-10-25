pub mod facade {
    pub use renamed_nc::*;
}

use renamed_nc::{ComPtr, IUnknown, NC_E_INVALID_TAG, NcStatus};

extern crate renamed_nc as neocogi_com;
#[allow(dead_code, unused_imports)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/contracts.rs"));
}
use bindings::*;

#[renamed_nc::com_object(neocogi_com = crate::facade, thread_safe, interfaces(IOverrideResolved))]
pub struct OverrideResolved;

impl IOverrideResolved for OverrideResolved {
    fn value(&self) -> Result<u16, crate::facade::NcStatus> {
        Ok(42)
    }
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
