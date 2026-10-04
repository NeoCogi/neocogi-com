//! Compile-checked reference contract generated entirely from NeoCogi IDL.

#![no_std]
#![deny(missing_docs)]

include!(concat!(env!("OUT_DIR"), "/neocogi_idl_example.include.rs"));

#[neocogi_com::com_object(interfaces(IExtentChangedEvent), ownership = exclusive)]
/// Owns one copied snapshot without retaining the example object.
struct ExtentSnapshot(ExtentChanged);
impl neocogi_com::events::IEvent for ExtentSnapshot {}
impl IExtentChangedEvent for ExtentSnapshot {
    fn value(&self) -> Result<ExtentChanged, neocogi_com::NcStatus> {
        Ok(self.0)
    }
}
impl neocogi_com::events::EventPayload for ExtentChanged {
    fn into_event(self) -> Result<neocogi_com::ComBox<neocogi_com::events::IEventAbi>, neocogi_com::NcStatus> {
        ExtentSnapshot::into_box(ExtentSnapshot(self)).query().map_err(|(status, _)| status)
    }
    fn from_event(event: &neocogi_com::ComBox<neocogi_com::events::IEventAbi>) -> Result<Option<Self>, neocogi_com::NcStatus> {
        neocogi_com::events::downcast_event::<IExtentChangedEventAbi>(event)?
            .map(|value| value.value())
            .transpose()
    }
}
