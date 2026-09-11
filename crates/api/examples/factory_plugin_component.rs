//! Factory-only shared-library example backed by a generated contract crate.

use neocogi_com::{ComPtr, NcStatus, com_factory, com_object};
use neocogi_idl_example_contracts::{ExampleFactory, ExampleObject, ExampleObjectAbi, Extent, IMeasured, MODULE};

/// Private mutable object constructed only through `ExampleFactory`.
#[com_object(neocogi_idl_example_contracts::ExampleObject)]
struct DynamicExampleObject {
    /// Dimensions owned entirely by this COM object allocation.
    extent: Extent,
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
        Ok(())
    }
}

/// Stateless root factory exported by this plugin assembly.
#[com_factory(neocogi_idl_example_contracts::ExampleFactory)]
struct DynamicExampleFactory;

impl ExampleFactory for DynamicExampleFactory {
    /// Constructs one typed object without consulting a runtime class table.
    fn create(&self, extent: Extent) -> Result<ComPtr<ExampleObjectAbi>, NcStatus> {
        DynamicExampleObject::from_value(DynamicExampleObject { extent }).query()
    }
}

neocogi_com::factory_plugin! {
    name = "nc.examples.factory_plugin_component",
    version = "1.0.0",
    contracts = [MODULE],
    factory = DynamicExampleFactory,
}

// This crate is the final DSO and therefore owns the sole factory description export.
neocogi_com::export_factory_plugin!();
