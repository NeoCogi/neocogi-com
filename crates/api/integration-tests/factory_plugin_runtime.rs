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

/// Verifies registered factory metadata inspection and direct activation on registered
/// static plugins, without reopening files or requiring caller-supplied root interface IDs.
#[test]
fn registered_factory_info_and_direct_activation_work_without_filesystem_path() {
    use neocogi_com::{ComInterface, NC_E_NOT_FOUND, NcGuid};

    let runtime = Runtime::default();
    let plugin_id = guid_from_qualified_name("nc.tests.generated_factory_plugin");

    // SAFETY: The static description outlives the process.
    unsafe { let _ = runtime.register_factory_plugin_description(factory_plugin_description()); }

    // Inspect registered factory info without any filesystem path.
    let factory_info = runtime
        .inspection()
        .plugin_factory_info(plugin_id)
        .expect("plugin factory inspection failed")
        .expect("registered factory info missing");

    assert_eq!(factory_info.plugin_id, plugin_id);
    assert_eq!(factory_info.root_factory, ExampleFactoryAbi::INTERFACE_ID);
    assert_eq!(factory_info.root_factory_threading, neocogi_com::NC_ABI_THREADING_THREAD_LOCAL);

    // Direct root activation without knowing the interface type in advance.
    let root = runtime
        .activate_registered_plugin_root(plugin_id)
        .expect("direct root activation failed");

    // Query canonical IUnknown and specific factory capability.
    let factory = root
        .query::<ExampleFactoryAbi>()
        .expect("root must query ExampleFactoryAbi");
    let object = factory
        .create(
            Extent { width: 10, height: 10 },
            neocogi_com::NcValueRef::new(&neocogi_idl_example_contracts::ExampleObjectEventBinding::Unobserved).unwrap(),
        )
        .expect("factory object creation failed");
    assert_eq!(object.extent().expect("extent query failed"), Extent { width: 10, height: 10 });

    // Non-existent plugin returns None / NOT_FOUND
    let unknown_id = NcGuid::from_fields(0xdeadbeef, 0, 0, [0; 8]);
    assert_eq!(
        runtime.inspection().plugin_factory_info(unknown_id).expect("inspection check"),
        None
    );
    assert_eq!(
        runtime.activate_registered_plugin_root(unknown_id).map(|_| ()),
        Err(NC_E_NOT_FOUND)
    );
}
