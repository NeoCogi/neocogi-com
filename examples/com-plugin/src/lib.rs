//! One plugin implementation built as either a shared library or a linked Rust library.

use neocogi_com::{
    binary, ComDeserialize, ComSerialize, ComPtr, NcValueRef, IUnknown, NcBytes, INcBytesAbi, NcGuid, NcStatus, NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE, com_object,
};
use neocogi_idl_example_contracts::{ExampleFactory, ExampleObject, ExampleObjectAbi, Extent, ExtentReport, IMeasured};

/// Private mutable object constructed only through `ExampleFactory`.
#[com_object(interfaces(neocogi_idl_example_contracts::ExampleObject, neocogi_com::persistence::IPersistable))]
struct DynamicExampleObject {
    /// Dimensions owned entirely by this COM object allocation.
    extent: Extent,
    /// Optional concrete destination, excluded from the persistent state.
    changed: neocogi_com::events::EventPort,
}

impl IMeasured for DynamicExampleObject {
    /// Returns the current dimensions by ABI-safe value copy.
    fn extent(&self) -> Result<Extent, NcStatus> {
        Ok(self.extent)
    }
    /// Writes only fresh copied records into caller storage, without provider scratch.
    fn reports(&self, mut output: neocogi_com::NcOutputWriter<'_, ExtentReport>) -> Result<u32, NcStatus> {
        if output.capacity() >= 2 {
            output.extend_from_slice(&[
                ExtentReport::Current { extent: self.extent, usable: true },
                ExtentReport::Empty])?;
        }
        Ok(2)
    }
}

impl ExampleObject for DynamicExampleObject {
    /// Replaces dimensions through the exclusive generated call gate.
    fn resize(&mut self, extent: Extent) -> Result<(), NcStatus> {
        self.extent = extent;
        let event = neocogi_idl_example_contracts::events::extent_changed(extent)?;
        self.changed.post(event)
    }
}

/// Stateless root factory exported by this plugin assembly.
#[com_object(neocogi_idl_example_contracts::ExampleFactory)]
struct DynamicExampleFactory;

impl ExampleFactory for DynamicExampleFactory {
    /// Constructs one typed object without consulting a runtime class table.
    fn create(&self, extent: Extent, events: NcValueRef<'_, neocogi_com::events::EventBinding>) -> Result<ComPtr<ExampleObjectAbi>, NcStatus> {
        let queue = events.to_owned()?.into_queue();
        let local = DynamicExampleObject::into_local(DynamicExampleObject {
            extent,
            changed: neocogi_com::events::EventPort::new(),
        });
        // Obtain identity from initialized storage and connect delivery before
        // returning the first application-facing object capability.
        let origin = local.query::<ExampleObjectAbi>()?.downgrade()?;
        local.try_borrow_mut()?.changed = neocogi_com::events::EventPort::with_queue(queue, origin);
        local.query()
    }
}

/// Acquires one owned root; object construction remains on its typed methods.
#[neocogi_com::plugin(name = "nc.examples.factory_plugin_component")]
fn acquire_root() -> Result<neocogi_com::ComPtr<neocogi_idl_example_contracts::ExampleFactoryAbi>, neocogi_com::NcStatus> {
    DynamicExampleFactory::from_value(DynamicExampleFactory).query()}

// Only the final DSO exports this symbol; monoliths call the Rust getter.

/// Recipe identity used by the explicitly registered persistence adapter.
pub const PERSISTENT_KIND: NcGuid = neocogi_com::guid_from_qualified_name("nc.examples.extent-state");

/// Version-one persistent state; neither Rust layout nor event receivers are saved.
#[derive(ComSerialize, ComDeserialize)]
pub struct ExtentState {
    /// Authored/current width.
    pub width: u32,
    /// Authored/current height.
    pub height: u32,
}

impl neocogi_com::persistence::IPersistable for DynamicExampleObject {
    /// Returns a recipe identity independent from ABI revisions.
    fn type_id(&self) -> Result<NcGuid, NcStatus> {
        Ok(PERSISTENT_KIND)
    }
    /// The persistent record has one explicitly supported revision.
    fn schema_version(&self) -> Result<u32, NcStatus> {
        Ok(1)
    }
    /// Snapshot captures current dimensions through the shared typed codec.
    fn save_state(&self) -> Result<ComPtr<INcBytesAbi>, NcStatus> {
        let state = ExtentState {
            width: self.extent.width,
            height: self.extent.height,
        };
        Ok(NcBytes::from_vec(binary::to_vec(&state).map_err(|_| NC_E_INVALID_ARG)?))
    }
    /// This ordinary object is a leaf in the persistent tree.
    fn child_count(&self) -> Result<u32, NcStatus> {
        Ok(0)
    }
    /// A leaf has no valid child indices.
    fn child(&self, _: u32) -> Result<ComPtr<IUnknown>, NcStatus> {
        Err(NC_E_OUT_OF_RANGE)
    }
}
