//! Generated factory registration and activation in shared-runtime or static mode.

use neocogi_com::{ComPtr, NcStatus, Runtime, com_factory, com_class, factory_plugin, guid_from_qualified_name};
use neocogi_idl_example_contracts::{ExampleFactory, ExampleFactoryAbi, ExampleObject, ExampleObjectAbi, Extent, IMeasured};

/// Private mutable implementation of the constructor-free IDL class.
#[com_class(neocogi_idl_example_contracts::ExampleObject)]
struct ExampleObjectImplementation {
    /// Current dimensions returned by both callable projections.
    extent: Extent,
    /// The constructor supplies the sole receiver, retained as private producer state.
    events: neocogi_idl_example_contracts::ExampleObjectEventPort,
}

impl IMeasured for ExampleObjectImplementation {
    /// Returns a copy of the object's current dimensions.
    fn extent(&self) -> Result<Extent, NcStatus> {
        Ok(self.extent)
    }
    /// Emits a fresh checked prefix using the same contract as the independent plugin.
    fn reports(&self, mut output: neocogi_com::NcOutputWriter<'_, neocogi_idl_example_contracts::ExtentReport>) -> Result<u32, NcStatus> {
        use neocogi_idl_example_contracts::ExtentReport;
        if output.capacity() >= 2 {
            output.extend_from_slice(&[ExtentReport::Current { extent: self.extent, usable: true }, ExtentReport::Empty])?;
        }
        Ok(2)
    }
}

impl ExampleObject for ExampleObjectImplementation {
    /// Replaces dimensions through the generated exclusive `&mut self` call.
    fn resize(&mut self, extent: Extent) -> Result<(), NcStatus> {
        self.extent = extent;
        self.events.post(&neocogi_idl_example_contracts::ExtentChanged {
            width: extent.width,
            height: extent.height,
        })
    }
}

/// Stateless plugin root that owns object construction policy.
#[com_factory(neocogi_idl_example_contracts::ExampleFactory)]
struct ExampleFactoryImplementation;

impl ExampleFactory for ExampleFactoryImplementation {
    /// Creates one object directly without a runtime class registry lookup.
    fn create(
        &self,
        extent: Extent,
        events: neocogi_com::NcValueRef<'_, neocogi_idl_example_contracts::ExampleObjectEventBinding>,
    ) -> Result<ComPtr<ExampleObjectAbi>, NcStatus> {
        let queue = events.to_owned()?.into_queue();
        ExampleObjectImplementation::from_value_cyclic::<ExampleObjectAbi>(move |origin| ExampleObjectImplementation {
            extent,
            events: neocogi_com::events::EventPort::with_queue(queue, origin),
        })?
        .query()
    }
}

factory_plugin! {
    name = "nc.tests.generated_factory_plugin",
    version = "1.0.0",
    factory = ExampleFactoryImplementation,
}

/// Verifies one facade observes registration and typed activation through the
/// linked registry, in both shared-runtime and intentional static modes.
#[test]
fn generated_contract_factory_activates_without_a_class_table() {
    let runtime = Runtime::default();
    // SAFETY: Macro-generated plugin and linked contract metadata are immutable
    // statics retained by the test executable for the process lifetime.
    unsafe { runtime.register_factory_plugin_description(factory_plugin_description()) }.expect("factory plugin registration failed");

    let plugin_id = guid_from_qualified_name("nc.tests.generated_factory_plugin");
    let factory = runtime
        .activate_plugin_factory::<ExampleFactoryAbi>(plugin_id)
        .expect("typed root factory activation failed");
    let mut object = factory
        .create(
            Extent { width: 20, height: 10 },
            neocogi_com::NcValueRef::new(&neocogi_idl_example_contracts::ExampleObjectEventBinding::Unobserved).unwrap(),
        )
        .expect("factory object creation failed");
    assert_eq!(object.extent().expect("extent query failed"), Extent { width: 20, height: 10 });
    object.resize(Extent { width: 44, height: 31 }).expect("exclusive resize failed");
    assert_eq!(object.extent().expect("updated extent query failed"), Extent { width: 44, height: 31 });

    let plugins = runtime.plugin_registry().plugin_ids().expect("plugin enumeration failed");
    assert!(plugins.contains(&plugin_id));
    let info = runtime
        .inspection()
        .plugin_info(plugin_id)
        .expect("plugin inspection failed")
        .expect("registered factory plugin missing from inspection");
    assert_eq!(info.qualified_name, "nc.tests.generated_factory_plugin");
}
