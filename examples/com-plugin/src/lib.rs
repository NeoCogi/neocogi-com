//! One plugin implementation built as either a shared library or a linked Rust library.

use neocogi_com::{
    binary, ComDeserialize, ComSerialize, ComPtr, NcValueRef, IUnknown, NcBytes, INcBytesAbi, NcGuid, NcStatus, NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE,
    com_factory, com_object,
};
use neocogi_idl_example_contracts::{ExtentChanged, ExampleObjectEventPort, ExampleObjectEventBinding};
use neocogi_idl_example_contracts::{ExampleFactory, ExampleObject, ExampleObjectAbi, Extent, IMeasured};

/// Private mutable object constructed only through `ExampleFactory`.
#[com_object(interfaces(neocogi_idl_example_contracts::ExampleObject, neocogi_com::persistence::IPersistable))]
struct DynamicExampleObject {
    /// Dimensions owned entirely by this COM object allocation.
    extent: Extent,
    /// Optional concrete destination, excluded from the persistent state.
    changed: ExampleObjectEventPort,
}

impl IMeasured for DynamicExampleObject {
    /// Returns the current dimensions by ABI-safe value copy.
    fn extent(&self) -> Result<Extent, NcStatus> {
        Ok(self.extent)
    }
}

impl ExampleObject for DynamicExampleObject {
    /// Replaces dimensions through the exclusive generated call gate.
    fn resize(&mut self, extent: Extent) -> Result<(), NcStatus> {
        self.extent = extent;
        self.changed.post(&ExtentChanged {
            width: extent.width,
            height: extent.height,
        })
    }
}

/// Stateless root factory exported by this plugin assembly.
#[com_factory(neocogi_idl_example_contracts::ExampleFactory)]
struct DynamicExampleFactory;

impl ExampleFactory for DynamicExampleFactory {
    /// Constructs one typed object without consulting a runtime class table.
    fn create(&self, extent: Extent, events: NcValueRef<'_, ExampleObjectEventBinding>) -> Result<ComPtr<ExampleObjectAbi>, NcStatus> {
        DynamicExampleObject::from_value(DynamicExampleObject {
            extent,
            changed: neocogi_com::events::EventPort::with_sink(events.to_owned()?.into_sink()),
        })
        .query()
    }
}

neocogi_com::factory_plugin! {
    name = "nc.examples.factory_plugin_component",
    version = "0.0.0",
    factory = DynamicExampleFactory,
}

// Only the final DSO exports this symbol; monoliths call the Rust getter.
#[cfg(feature = "dynamic-plugin")]
neocogi_com::export_factory_plugin!();

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
