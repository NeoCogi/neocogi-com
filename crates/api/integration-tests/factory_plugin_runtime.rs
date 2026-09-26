//! End-to-end static registration and activation through a generated IDL DSO.

use std::path::PathBuf;
use std::process::Command;

use neocogi_com::{ComPtr, NcStatus, Runtime, com_factory, com_class, factory_plugin, guid_from_qualified_name};
use neocogi_idl_example_contracts::{ExampleFactory, ExampleFactoryAbi, ExampleObject, ExampleObjectAbi, Extent, IMeasured};

/// Private mutable implementation of the constructor-free IDL class.
#[com_class(neocogi_idl_example_contracts::ExampleObject, event = events)]
struct ExampleObjectImplementation {
    /// Current dimensions returned by both callable projections.
    extent: Extent,
    /// The sole stream shared by the concrete class and inherited event capability.
    events: neocogi_idl_example_contracts::ExampleObjectEventPort,
}

impl neocogi_idl_example_contracts::IExampleEvents for ExampleObjectImplementation {}

impl IMeasured for ExampleObjectImplementation {
    /// Returns a copy of the object's current dimensions.
    fn extent(&self) -> Result<Extent, NcStatus> {
        Ok(self.extent)
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
    fn create(&self, extent: Extent) -> Result<ComPtr<ExampleObjectAbi>, NcStatus> {
        ExampleObjectImplementation::from_value(ExampleObjectImplementation { extent, events: Default::default() }).query()
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
    let mut object = factory.create(Extent { width: 20, height: 10 }).expect("factory object creation failed");
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

/// Verifies the linked runtime loads a real factory DSO, publishes its exact
/// contract, and exposes its root through the same host-visible registry.
#[test]
fn dynamic_factory_plugin_uses_compiled_contracts() {
    let library = build_dynamic_fixture();
    let runtime = Runtime::default();
    // SAFETY: The built example is trusted test code and the runtime retains
    // its library plus linked canonical contract module for process lifetime.
    let loaded = unsafe { runtime.load_plugin_library(&library) };
    assert_eq!(loaded.path(), library);

    let plugin_id = guid_from_qualified_name("nc.examples.factory_plugin_component");
    assert_eq!(loaded.plugin_id(), plugin_id);
    let factory = runtime
        .activate_plugin_factory::<ExampleFactoryAbi>(plugin_id)
        .expect("dynamic root factory activation failed");
    let object = factory.create(Extent { width: 7, height: 9 }).expect("dynamic factory method failed");
    assert_eq!(object.extent().expect("dynamic object method failed"), Extent { width: 7, height: 9 });
}

/// Builds the factory-only cdylib example in an isolated target directory.
fn build_dynamic_fixture() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../examples/com-plugin/Cargo.toml");
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../target/factory-plugin-component-fixture");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .arg("rustc")
        .arg("--offline")
        .arg("--locked")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--target-dir")
        .arg(&target)
        .args(["--lib", "--crate-type", "cdylib", "--features", "dynamic-plugin"])
        .output()
        .expect("failed to build dynamic factory plugin fixture");
    assert!(
        output.status.success(),
        "factory plugin fixture failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    target
        .join("debug")
        .join(format!("{}nc_example_com{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX,))
}
