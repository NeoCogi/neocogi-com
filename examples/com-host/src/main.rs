//! Exercises identical typed construction in dynamic and monolithic hosts.

#[path = "../support/notification_channel.rs"]
mod notification_channel;
use neocogi_com::{binary, Runtime, ComPtr, ComRef, IUnknown, NcCopyFromBufferOffset, NcStatus, NC_E_ABI_VERSION, NC_E_INVALID_ARG, guid_from_qualified_name};
use neocogi_com::persistence::{FactoryRegistry, snapshot};
use neocogi_com::NcValueRef;

#[path = "../support/extent_state.rs"]
mod extent_state;
use extent_state::{ExtentState, PERSISTENT_KIND};
use neocogi_example_com_host::contracts::{ExampleFactoryAbi, ExampleObjectAbi, Extent, ExtentReport};

/// Registers the selected plugin, acquires its checked factory, and mutates one object.
#[neocogi_com::com_main]
fn main(runtime: Runtime) {
    register_plugin(&runtime);
    qualify_peer(&runtime);
    // The runtime verifies the complete transitive contract before bootstrap.
    let factory = runtime
        .acquire_plugin_root::<ExampleFactoryAbi>(guid_from_qualified_name("nc.examples.factory_plugin_component"))
        .expect("compatible example factory");
    let (queue, messages) = notification_channel::channel();
    let binding = neocogi_example_com_host::contracts::ExtentBinding::from_observer(Some(queue.as_com_ref())).expect("shared application receiver");
    let mut object = factory
        .create(Extent { width: 20, height: 10 }, NcValueRef::new(&binding).unwrap())
        .expect("typed object construction");
    object.resize(Extent { width: 44, height: 31 }).expect("exclusive object mutation");
    let extent = object.extent().expect("inherited interface call");
    assert_eq!(extent, Extent { width: 44, height: 31 });
    // This assertion runs with independently selected compilers, private-layout
    // randomization, LTO and panic profiles. Native enum layout never crosses COM.
    let mut reports = neocogi_com::NcOutputBuffer::new();
    assert_eq!(object.reports(reports.output(0, 3)).unwrap(), 2);
    assert_eq!(reports.as_slice(), [ExtentReport::Current { extent, usable: true }, ExtentReport::Empty]);
    assert_eq!(object.reports(reports.output(0, 0)).unwrap(), 2);
    assert!(reports.is_empty(), "a size query must not publish an uninitialized tail");
    let message = messages.try_recv().unwrap();
    let origin = message.source;
    let changed = message.extent;
    assert!(origin.same_object(&object.downgrade().unwrap()));
    assert_eq!((changed.width, changed.height), (44, 31));
    // A promotion crosses from the host's weak protocol into plugin-owned storage.
    let weak = object.downgrade().unwrap();
    let promoted = weak.upgrade().unwrap().expect("live plugin object");
    assert_eq!(promoted.extent().unwrap(), extent);
    drop(promoted);
    let saved = snapshot(&runtime, object.query::<IUnknown>().unwrap().as_com_ref()).unwrap();
    let mut registry = FactoryRegistry::new(&runtime).unwrap();
    registry
        .register(
            PERSISTENT_KIND,
            PersistenceAdapter::from_value(PersistenceAdapter { factory, queue }).query().unwrap(),
        )
        .unwrap();
    let restored = registry.restore(&saved).unwrap();
    assert_eq!(snapshot(&runtime, restored.as_com_ref()).unwrap(), saved);
    restored.query::<ExampleObjectAbi>().unwrap().resize(Extent { width: 45, height: 32 }).unwrap();
    let message = messages.try_recv().unwrap();
    let origin = message.source;
    let changed = message.extent;
    assert!(origin.same_object(&restored.downgrade().unwrap()));
    assert_eq!((changed.width, changed.height), (45, 32));
    // No queued origin or persistent recipe is allowed to keep either object alive.
    // These assertions execute in release/abort builds too, unlike debug_assert.
    drop(object);
    assert!(weak.upgrade().unwrap().is_none(), "final release must destroy the plugin value");
    drop(restored);
    assert!(origin.upgrade().unwrap().is_none(), "queued weak origin must expire");
    println!("factory object: {} x {}", extent.width, extent.height);
}

/// Borrows immutable plugin metadata from the rlib retained by this executable.
#[cfg(feature = "monolith")]
fn register_plugin(runtime: &Runtime) {
    // SAFETY: Both the descriptor and its code are linked into this process.
    unsafe { runtime.register_plugin_descriptor(nc_example_com::plugin_descriptor()) }.expect("static plugin registration");
}

/// Loads the trusted sample plugin supplied as the executable's first argument.
#[cfg(not(feature = "monolith"))]
fn register_plugin(runtime: &Runtime) {
    let path = std::env::args_os().nth(1).expect("usage: neocogi-example-com PATH_TO_PLUGIN");
    // SAFETY: This demonstration accepts its own trusted build artifact. The
    // runtime retains the mapping while any factory or object can reference it.
    unsafe { runtime.load_plugin_library(std::path::Path::new(&path)) };
}

/// An optional second native provider proves supplied access reaches one owner.
/// It is built independently by qualification, never linked into this host.
#[cfg(not(feature = "monolith"))]
fn qualify_peer(runtime: &Runtime) {
    let Some(path) = std::env::args_os().nth(2) else { return };
    let initial = runtime.plugins().unwrap().len();
    // SAFETY: qualification supplies its own trusted independently compiled DLL.
    let peer = unsafe { runtime.load_plugin_library(std::path::PathBuf::from(path)) };
    let before = runtime.plugins().unwrap();
    assert_eq!(before.len(), initial + 1);
    assert_eq!(
        runtime.plugin_info(peer).unwrap().unwrap().dependencies,
        [guid_from_qualified_name("nc.examples.factory_plugin_component")]
    );
    let marker = guid_from_qualified_name("nc.tests.registry-peer.marker");
    assert!(runtime.plugin_info(marker).unwrap().is_none(), "metadata discovery must not acquire B");
    let root = runtime.acquire_plugin_root::<ExampleFactoryAbi>(peer).unwrap();
    assert!(runtime.plugin_info(marker).unwrap().is_some(), "B's mutation must be visible to the host");
    assert_eq!(runtime.plugins().unwrap().len(), initial + 2);
    let state = runtime.plugins().unwrap();
    drop(runtime.acquire_plugin_root::<ExampleFactoryAbi>(peer).unwrap());
    assert_eq!(runtime.plugins().unwrap(), state, "repeated B acquisition reuses one marker");
    // SAFETY: this conflicting descriptor is process-lived in this executable.
    assert_eq!(
        unsafe { runtime.register_plugin_descriptor(conflict::plugin_descriptor()) },
        Err(NC_E_INVALID_ARG)
    );
    assert_eq!(runtime.plugins().unwrap(), state, "a conflict must preserve the shared catalog");
    drop(root);
}

/// Static execution has no separately built peer or native loader requirement.
#[cfg(feature = "monolith")]
fn qualify_peer(_runtime: &Runtime) {}

/// Same valid identity in different static storage must conflict with provider A.
#[cfg(not(feature = "monolith"))]
mod conflict {
    #[neocogi_com::plugin(name = "nc.examples.factory_plugin_component")]
    fn acquire(_runtime: neocogi_com::Runtime) -> Result<neocogi_com::ComPtr<neocogi_example_com_host::contracts::ExampleFactoryAbi>, neocogi_com::NcStatus> {
        panic!("conflicting metadata must never be acquired")
    }
}

/// Adapts an existing typed factory without changing its construction ABI.
#[neocogi_com::com_object(interfaces(neocogi_com::persistence::IPersistenceFactory))]
struct PersistenceAdapter {
    /// In shared mode this is a checked factory in the separately loaded plugin.
    factory: ComPtr<ExampleFactoryAbi>,
    /// Same application queue is shared by ordinary and restored objects.
    /// Live routing state is supplied separately from persisted dimensions.
    queue: ComPtr<neocogi_example_com_host::contracts::IExtentObserverAbi>,
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
        let binding = neocogi_example_com_host::contracts::ExtentBinding::from_observer(Some(self.queue.as_com_ref()))?;
        self.factory
            .create(Extent { width: state.width, height: state.height }, NcValueRef::new(&binding)?)?
            .query()
    }
}
