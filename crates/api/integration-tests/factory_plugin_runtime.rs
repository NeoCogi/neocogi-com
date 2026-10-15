//! Generated factory registration and acquisition in shared-runtime or static mode.

use neocogi_com::{ComPtr, NcStatus, Runtime, com_object, guid_from_qualified_name};
use neocogi_idl_example_contracts::{ExampleFactory, ExampleFactoryAbi, ExampleObject, ExampleObjectAbi, Extent, IMeasured};

/// Private mutable implementation of the constructor-free IDL class.
#[com_object(neocogi_idl_example_contracts::ExampleObject)]
struct ExampleObjectImplementation {
    /// Current dimensions returned by both callable projections.
    extent: Extent,
    /// The constructor supplies the sole receiver, retained as private producer state.
    events: neocogi_com::events::EventPort,
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
        self.events.post(neocogi_idl_example_contracts::events::extent_changed(extent)?)
    }
}

/// Stateless plugin root that owns object construction policy.
#[com_object(neocogi_idl_example_contracts::ExampleFactory)]
struct ExampleFactoryImplementation;

impl ExampleFactory for ExampleFactoryImplementation {
    /// Creates one object directly without a runtime class registry lookup.
    fn create(&self, extent: Extent, events: neocogi_com::NcValueRef<'_, neocogi_com::events::EventBinding>) -> Result<ComPtr<ExampleObjectAbi>, NcStatus> {
        let queue = events.to_owned()?.into_queue();
        ExampleObjectImplementation::from_value_cyclic::<ExampleObjectAbi>(move |origin| ExampleObjectImplementation {
            extent,
            events: neocogi_com::events::EventPort::with_queue(queue, origin),
        })?
        .query()
    }
}

/// Acquires one owned root; object construction remains on its typed methods.
#[neocogi_com::plugin(name = "nc.tests.generated_factory_plugin",
    version = "1.0.0")]
fn acquire_root() -> Result<neocogi_com::ComPtr<neocogi_idl_example_contracts::ExampleFactoryAbi>,
    neocogi_com::NcStatus> {
    ExampleFactoryImplementation::from_value(ExampleFactoryImplementation).query()
}

/// Verifies one facade observes registration and typed acquisition through the
/// linked registry, in both shared-runtime and intentional static modes.
#[test]
fn generated_contract_factory_acquires_without_a_class_table() {
    let runtime = Runtime::default();
    // SAFETY: Macro-generated plugin and linked contract metadata are immutable
    // statics retained by the test executable for the process lifetime.
    unsafe { runtime.register_plugin_descriptor(plugin_descriptor()) }.expect("interface plugin registration failed");

    let plugin_id = guid_from_qualified_name("nc.tests.generated_factory_plugin");
    let factory = runtime
        .acquire_plugin_root::<ExampleFactoryAbi>(plugin_id)
        .expect("typed root interface acquisition failed");
    let mut object = factory
        .create(
            Extent { width: 20, height: 10 },
            neocogi_com::NcValueRef::new(&neocogi_com::events::EventBinding::Unobserved).unwrap(),
        )
        .expect("interface object creation failed");
    assert_eq!(object.extent().expect("extent query failed"), Extent { width: 20, height: 10 });
    object.resize(Extent { width: 44, height: 31 }).expect("exclusive resize failed");
    assert_eq!(object.extent().expect("updated extent query failed"), Extent { width: 44, height: 31 });

    let plugins = runtime.plugins().expect("plugin enumeration failed");
    assert!(plugins.iter().any(|plugin| plugin.plugin_id == plugin_id));
    let info = runtime
        .plugin_info(plugin_id)
        .expect("plugin inspection failed")
        .expect("registered interface plugin missing from inspection");
    assert_eq!(info.qualified_name, "nc.tests.generated_factory_plugin");
    assert_eq!(info.origin, neocogi_com::PluginOrigin::Static);
}

/// Verifies registered factory metadata inspection and direct acquisition on registered
/// static plugins, without reopening files or requiring caller-supplied root interface IDs.
#[test]
fn complete_plugin_info_and_direct_acquisition_work_without_filesystem_path() {
    use neocogi_com::{ComInterface, NC_E_NOT_FOUND, NcGuid};

    let runtime = Runtime::default();
    let plugin_id = guid_from_qualified_name("nc.tests.generated_factory_plugin");

    // SAFETY: The static description outlives the process.
    unsafe {
        let _ = runtime.register_plugin_descriptor(plugin_descriptor());
    }

    // Inspect registered factory info without any filesystem path.
    let factory_info = runtime
        .plugin_info(plugin_id)
        .expect("plugin interface inspection failed")
        .expect("registered interface info missing");

    assert_eq!(factory_info.plugin_id, plugin_id);
    assert_eq!(factory_info.factory.interface, ExampleFactoryAbi::INTERFACE_ID);
    assert_eq!(factory_info.factory.threading, neocogi_com::NcAbiThreadingMode::ThreadLocal);

    // Direct root acquisition without knowing the interface type in advance.
    let root = runtime.acquire_registered_plugin_root(plugin_id).expect("direct root acquisition failed");

    // Query canonical IUnknown and specific factory capability.
    let factory = root.query::<ExampleFactoryAbi>().expect("root must query ExampleFactoryAbi");
    let object = factory
        .create(
            Extent { width: 10, height: 10 },
            neocogi_com::NcValueRef::new(&neocogi_com::events::EventBinding::Unobserved).unwrap(),
        )
        .expect("interface object creation failed");
    assert_eq!(object.extent().expect("extent query failed"), Extent { width: 10, height: 10 });

    // Non-existent plugin returns None / NOT_FOUND
    let unknown_id = NcGuid::from_fields(0xdeadbeef, 0, 0, [0; 8]);
    assert_eq!(runtime.plugin_info(unknown_id).expect("inspection check"), None);
    assert_eq!(runtime.acquire_registered_plugin_root(unknown_id).map(|_| ()), Err(NC_E_NOT_FOUND));
}

/// A plugin may publish a finished service without a construction-role marker.
mod service_root {
    use neocogi_com::{ComInterface, ComPtr, NcStatus, Runtime, ThreadSafe, com_interface, com_object};
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Tracks initialization separately from metadata inspection.
    static ACQUIRED: AtomicU32 = AtomicU32::new(0);
    /// Counts owners released after real calls through the acquired ABI.
    static DROPPED: AtomicU32 = AtomicU32::new(0);

    #[com_interface(auto_guid = "nc.tests.plugin.Service:v1", threading = shared, assigned_signature = b"00000000000000000000000000000091")]
    /// Ordinary service whose state is initialized in Rust.
    trait IService {
        /// Reads the sequence assigned to this acquisition.
        fn sequence(&self) -> Result<u32, NcStatus>;
    }

    #[com_object(IService)]
    /// Provider-private state; no factory implementation association is needed.
    struct Service {
        /// The acquisition sequence owned by this object.
        sequence: u32,
    }

    impl IService for Service {
        fn sequence(&self) -> Result<u32, NcStatus> {
            Ok(self.sequence)
        }
    }

    impl Drop for Service {
        fn drop(&mut self) {
            DROPPED.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[neocogi_com::plugin(name = "nc.tests.plugin.service")]
    /// Creates one initialized service with the interface's declared threading.
    fn acquire() -> Result<ComPtr<IServiceAbi, ThreadSafe>, NcStatus> {
        Service::from_value(Service { sequence: ACQUIRED.fetch_add(1, Ordering::SeqCst) }).query()
    }

    /// Inspection does not construct; two acquisitions own two independent services.
    #[test]
    fn ordinary_service_root_is_typed_owned_and_shared() {
        let descriptor = plugin_descriptor();
        assert_eq!(descriptor.root_factory, IServiceAbi::INTERFACE_ID);
        assert_eq!(unsafe { descriptor.plugin_version.as_str() }.unwrap(), env!("CARGO_PKG_VERSION"));
        assert_eq!(ACQUIRED.load(Ordering::SeqCst), 0);
        let runtime = Runtime::global();
        // SAFETY: the descriptor and provider code are retained by this executable.
        unsafe { runtime.register_plugin_descriptor(descriptor) }.unwrap();
        assert_eq!(ACQUIRED.load(Ordering::SeqCst), 0);
        let first = runtime.acquire_plugin_root::<IServiceAbi>(descriptor.plugin_id).unwrap();
        let second = runtime.acquire_plugin_root::<IServiceAbi>(descriptor.plugin_id).unwrap();
        assert_eq!(first.sequence().unwrap(), 0);
        assert_eq!(second.sequence().unwrap(), 1);
        std::thread::spawn(move || assert_eq!(second.sequence().unwrap(), 1)).join().unwrap();
        drop(first);
        assert_eq!(DROPPED.load(Ordering::SeqCst), 2);
    }
}

/// Fallible provider initialization obeys the same fixed output contract.
mod failed_root {
    use neocogi_com::{ComPtr, ComRaw, IUnknown, NcStatus, NC_E_POINTER};
    use neocogi_idl_example_contracts::ExampleFactoryAbi;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Records provider execution so invalid output storage cannot enter it.
    static CALLED: AtomicU32 = AtomicU32::new(0);

    #[neocogi_com::plugin(name = "nc.tests.plugin.failure", version = "2.3.4")]
    /// Deliberately misuses a successful status as an error to test normalization.
    fn acquire() -> Result<ComPtr<ExampleFactoryAbi>, NcStatus> {
        CALLED.fetch_add(1, Ordering::SeqCst);
        Err(neocogi_com::NC_OK)
    }

    /// Null output is rejected before provider code; errors clear valid storage.
    #[test]
    fn acquisition_clears_output_before_failure_and_normalizes_status() {
        let acquire = plugin_descriptor().acquire.unwrap();
        // SAFETY: null is a supported defensive ABI input.
        assert_eq!(unsafe { acquire(core::ptr::null_mut()) }, NC_E_POINTER);
        assert_eq!(CALLED.load(Ordering::SeqCst), 0);
        let mut out = ComRaw::<IUnknown> { object: core::ptr::dangling_mut(), vtable: core::ptr::dangling() };
        // SAFETY: output storage is valid; the stale pair is never dereferenced.
        assert!(unsafe { acquire(&mut out) }.is_err());
        assert!(out.is_null());
        assert_eq!(CALLED.load(Ordering::SeqCst), 1);
    }
}
