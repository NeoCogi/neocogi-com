pub mod facade {
    pub use renamed_nc::*;
}

use renamed_nc::{ComPtr, IUnknown, NC_E_INVALID_TAG, NcStatus};

#[renamed_nc::com_interface(auto_guid = "nc.tests.renamed.Auto:v1")]
pub trait IAutoResolved {
    fn add(&self, a: i32, b: i32) -> Result<i32, renamed_nc::NcStatus>;
}

#[renamed_nc::com_object(interfaces(IAutoResolved))]
pub struct AutoResolved;

impl IAutoResolved for AutoResolved {
    fn add(&self, a: i32, b: i32) -> Result<i32, renamed_nc::NcStatus> {
        Ok(a + b)
    }
}

#[renamed_nc::com_enum(tag = u32)]
#[derive(Debug, PartialEq)]
pub enum AutoEvent {
    Ready(u16),
}

#[renamed_nc::com_struct]
#[derive(Debug, PartialEq)]
pub struct AutoPoint {
    pub x: u16,
    pub y: u16,
}

#[renamed_nc::com_interface(nc_com = crate::facade, auto_guid = "nc.tests.renamed.Override:v1")]
pub trait IOverrideResolved {
    fn value(&self) -> Result<u16, crate::facade::NcStatus>;
}

#[renamed_nc::com_object(nc_com = crate::facade, thread_safe, interfaces(IOverrideResolved))]
pub struct OverrideResolved;

impl IOverrideResolved for OverrideResolved {
    fn value(&self) -> Result<u16, crate::facade::NcStatus> {
        Ok(42)
    }
}

#[renamed_nc::com_enum(nc_com = crate::facade, tag = u16)]
#[derive(Debug, PartialEq)]
pub enum OverrideEvent {
    Ready(u32),
}

#[renamed_nc::com_struct(nc_com = crate::facade)]
#[derive(Debug, PartialEq)]
pub struct OverridePoint {
    pub x: u16,
    pub y: u16,
}

pub fn exercise_auto_resolved_path() -> Result<i32, NcStatus> {
    let unknown: ComPtr<IUnknown> = AutoResolved::create();
    let resolved = unknown.query::<IAutoResolvedAbi>()?;

    resolved.add(20, 22)
}

pub fn exercise_auto_enum_path() -> Result<(), NcStatus> {
    let abi = AutoEventAbi::from(AutoEvent::Ready(9));
    let value = AutoEvent::try_from(abi)?;

    if value == AutoEvent::Ready(9) {
        Ok(())
    } else {
        Err(NC_E_INVALID_TAG)
    }
}

pub fn exercise_auto_struct_path() -> Result<u16, NcStatus> {
    let value = AutoPoint { x: 3, y: 4 };
    Ok(value.x + value.y)
}

pub fn exercise_explicit_override_path() -> Result<u16, NcStatus> {
    let unknown: ComPtr<IUnknown, renamed_nc::ThreadSafe> = OverrideResolved::create();
    let resolved = unknown.query::<IOverrideResolvedAbi>()?;

    resolved.value()
}

pub fn exercise_enum_override_path() -> Result<(), NcStatus> {
    let abi = OverrideEventAbi::from(OverrideEvent::Ready(7));
    let value = OverrideEvent::try_from(abi)?;

    if value == OverrideEvent::Ready(7) {
        Ok(())
    } else {
        Err(NC_E_INVALID_TAG)
    }
}

pub fn exercise_struct_override_path() -> Result<u16, NcStatus> {
    let value = OverridePoint { x: 5, y: 6 };
    Ok(value.x + value.y)
}
