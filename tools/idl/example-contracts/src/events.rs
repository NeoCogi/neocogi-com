//! Immutable COM event construction. Every result is the sole owner of its captured snapshot.
use crate::*;
use neocogi_com::{ComBox, NcStatus};
use neocogi_com::events::{IEvent, IEventAbi};

#[neocogi_com::com_object(interfaces(crate::IExtentChangedEvent), ownership = exclusive)]
/// Immutable captured fact; this allocation retains no source widget or service.
struct ExtentChanged {
    /// Captured extent, independent of subsequent producer edits.
    extent: Extent,
}
impl IEvent for ExtentChanged {}
impl IExtentChangedEvent for ExtentChanged {
    /// Reads the immutable extent snapshot through the public event ABI.
    fn extent(&self) -> Result<Extent, NcStatus> {
        Ok(self.extent)
    }
}
/// Captures extent changed as one exclusive event, without retaining its source.
pub fn extent_changed(extent: Extent) -> Result<ComBox<IEventAbi>, NcStatus> {
    ExtentChanged::into_box(ExtentChanged { extent })
        .query()
        .map_err(|(status, _unpublished)| status)
}
