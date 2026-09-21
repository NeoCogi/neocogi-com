//! Exercises identical typed construction in dynamic and monolithic hosts.

use neocogi_com::{binary, Runtime, ComPtr, ComRef, IUnknown, NcCopyFromBufferOffset, NcStatus, NC_E_ABI_VERSION, NC_E_INVALID_ARG, guid_from_qualified_name};
use neocogi_com::persistence::{FactoryRegistry, snapshot};
use neocogi_com::events::{INamedOutputsAbi, bind_output};
use nc_example_com::{ExtentState, PERSISTENT_KIND};
use neocogi_idl_example_contracts::{ExampleFactoryAbi, ExampleObjectAbi, Extent, ExtentChanged};

/// Registers the selected plugin, acquires its checked factory, and mutates one object.
fn main() {
    let runtime = Runtime::global();
    register_plugin(runtime);
    // The runtime verifies the complete transitive contract before bootstrap.
    let factory = runtime
        .activate_plugin_factory::<ExampleFactoryAbi>(guid_from_qualified_name("nc.examples.factory_plugin_component"))
        .expect("compatible example factory");
    let mut object = factory.create(Extent { width: 20, height: 10 }).expect("typed object construction");
    let outputs = object.query::<INamedOutputsAbi>().expect("opaque event capability");
    let (sender, messages) = std::sync::mpsc::channel();
    bind_output(outputs.as_com_ref(), "resized", sender.clone(), |event: ExtentChanged| {
        (event.width, event.height)
    })
    .unwrap();
    object.resize(Extent { width: 44, height: 31 }).expect("exclusive object mutation");
    let extent = object.extent().expect("inherited interface call");
    assert_eq!(extent, Extent { width: 44, height: 31 });
    assert_eq!(messages.try_recv().unwrap(), (44, 31));
    let saved = snapshot(object.query::<IUnknown>().unwrap().as_com_ref()).unwrap();
    let mut registry = FactoryRegistry::new().unwrap();
    registry
        .register(PERSISTENT_KIND, PersistenceAdapter::from_value(PersistenceAdapter { factory }).query().unwrap())
        .unwrap();
    let restored = registry.restore(&saved).unwrap();
    assert_eq!(snapshot(restored.as_com_ref()).unwrap(), saved);
    let restored_outputs = restored.query::<INamedOutputsAbi>().unwrap();
    bind_output(restored_outputs.as_com_ref(), "resized", sender, |event: ExtentChanged| {
        (event.width, event.height)
    })
    .unwrap();
    restored.query::<ExampleObjectAbi>().unwrap().resize(Extent { width: 45, height: 32 }).unwrap();
    assert_eq!(messages.try_recv().unwrap(), (45, 32));
    println!("factory object: {} x {}", extent.width, extent.height);
}

/// Borrows immutable plugin metadata from the rlib retained by this executable.
#[cfg(feature = "monolith")]
fn register_plugin(runtime: &Runtime) {
    // SAFETY: Both the descriptor and its code are linked into this process.
    unsafe { runtime.register_factory_plugin_description(nc_example_com::factory_plugin_description()) }.expect("static plugin registration");
}

/// Loads the trusted sample plugin supplied as the executable's first argument.
#[cfg(not(feature = "monolith"))]
fn register_plugin(runtime: &Runtime) {
    let path = std::env::args_os().nth(1).expect("usage: neocogi-example-com PATH_TO_PLUGIN");
    // SAFETY: This demonstration accepts its own trusted build artifact. The
    // runtime retains the mapping while any factory or object can reference it.
    unsafe { runtime.load_plugin_library(std::path::Path::new(&path)) };
}

/// Adapts an existing typed factory without changing its construction ABI.
#[neocogi_com::com_object(interfaces(neocogi_com::persistence::IPersistenceFactory))]
struct PersistenceAdapter {
    /// In shared mode this is a checked factory in the separately loaded plugin.
    factory: ComPtr<ExampleFactoryAbi>,
}
/// Validates the explicitly supported schema before invoking plugin construction.
fn decode(version: u32, state: NcCopyFromBufferOffset<'_, u8>, children: u32) -> Result<ExtentState, NcStatus> {
    if version != 1 {
        return Err(NC_E_ABI_VERSION);
    }
    if state.offset() != 0 || children != 0 {
        return Err(NC_E_INVALID_ARG);
    }
    binary::from_slice(state.as_slice()).map_err(|_| NC_E_INVALID_ARG)
}
impl neocogi_com::persistence::IPersistenceFactory for PersistenceAdapter {
    /// Preflight has no construction side effects.
    fn validate(&self, version: u32, state: NcCopyFromBufferOffset<'_, u8>, children: u32) -> Result<(), NcStatus> {
        decode(version, state, children).map(|_| ())
    }
    /// Binary and ordinary construction share the same provider constructor.
    fn restore(
        &self,
        version: u32,
        state: NcCopyFromBufferOffset<'_, u8>,
        children: ComRef<'_, neocogi_com::persistence::IObjectChildrenAbi>,
    ) -> Result<ComPtr<IUnknown>, NcStatus> {
        let state = decode(version, state, children.count()?)?;
        self.factory.create(Extent { width: state.width, height: state.height })?.query()
    }
}
