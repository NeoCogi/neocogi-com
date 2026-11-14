//! Isolated processes exercise provider selection through the real host C tables.

use std::sync::atomic::{AtomicUsize, Ordering};
use neocogi_com::{ComBox, ComPtr, ComRef, INcBytesAbi, IUnknown, NcBytes, NcCopyFromBufferOffset, NcGuid, NcStatus};
use neocogi_com::persistence::*;
use neocogi_com::persistence::Limits;
use neocogi_com::serialization::*;

/// Explicit semantic recipe, independent of the serialization plugin identity.
const KIND: NcGuid = neocogi_com::guid_from_qualified_name("nc.tests.host.codec-delegation");
/// Successful and failed named selections, observed independently of the codec.
static SELECTIONS: AtomicUsize = AtomicUsize::new(0);
/// Default selection must remain unused even when it supplies valid JSON.
static DEFAULTS: AtomicUsize = AtomicUsize::new(0);
/// Live roots and format factories must survive all operation callbacks.
static ROOTS: AtomicUsize = AtomicUsize::new(0);
/// Independently retained selected format factories, released on every exit.
static FORMATS: AtomicUsize = AtomicUsize::new(0);
/// Zero succeeds; the other cases inject distinct format-selection failures.
static FAILURE: AtomicUsize = AtomicUsize::new(0);
/// Recipe callbacks must not run before format selection and complete decoding.
static VALIDATIONS: AtomicUsize = AtomicUsize::new(0);

/// Runs each catalog-sensitive assertion with a fresh host, independent of peers.
fn isolated(name: &str) -> bool {
    const CHILD: &str = "NC_HOST_PERSISTENCE_CASE";
    if std::env::var(CHILD).ok().as_deref() == Some(name) {
        return false;
    }
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &format!("persistence::tests::{name}"), "--nocapture"])
        .env(CHILD, name)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    true
}

/// A registration from plugin code requires the catalog write lock. Completion
/// proves bootstrap/selection/session/recipe callbacks run outside catalog locks.
fn reenter_catalog(runtime: neocogi_com::Runtime) {
    // SAFETY: The immutable descriptor and its callbacks are linked into this process.
    unsafe { runtime.register_plugin_descriptor(marker::plugin_descriptor()) }.unwrap();
}

/// An unrelated catalog entry published reentrantly by the serialization provider.
mod marker {
    use super::*;
    /// Its factory is never invoked; only registration exercises the catalog lock.
    #[neocogi_com::plugin(name = "nc.tests.host.reentrant-serialization")]
    fn acquire(_: neocogi_com::Runtime) -> Result<ComPtr<ISerializationPluginAbi>, NcStatus> {
        Err(neocogi_com::NC_E_BUSY)
    }
}

/// A JSON-default provider with an independently observable named NCB capability.
#[neocogi_com::com_object(ISerializationPlugin)]
struct Selection;
impl Drop for Selection {
    fn drop(&mut self) {
        ROOTS.fetch_sub(1, Ordering::Relaxed);
    }
}
impl ISerializationPlugin for Selection {
    fn default_format(&self) -> Result<ComPtr<ISerializationFormatAbi>, NcStatus> {
        DEFAULTS.fetch_add(1, Ordering::Relaxed);
        Ok(neocogi_com_serialization::json::factory())
    }
    fn format(&self, name: NcCopyFromBufferOffset<'_, u8>) -> Result<ComPtr<ISerializationFormatAbi>, NcStatus> {
        assert_eq!((name.offset(), name.as_slice()), (0, b"ncb".as_slice()));
        SELECTIONS.fetch_add(1, Ordering::Relaxed);
        reenter_catalog(crate::initialize_runtime());
        match FAILURE.load(Ordering::Relaxed) {
            0 => {
                FORMATS.fetch_add(1, Ordering::Relaxed);
                Selected::from_value(Selected).query()
            }
            1 => Err(neocogi_com::NC_E_NOINTERFACE),
            2 => Err(neocogi_com::NC_E_BUSY),
            _ => Err(neocogi_com::NC_E_OUT_OF_RANGE),
        }
    }
}

/// Selects no other runtime: bootstrap uses the host access supplied to the plugin.
#[neocogi_com::plugin(name = "nc.serialization")]
fn acquire(runtime: neocogi_com::Runtime) -> Result<ComPtr<ISerializationPluginAbi>, NcStatus> {
    reenter_catalog(runtime);
    ROOTS.fetch_add(1, Ordering::Relaxed);
    Selection::from_value(Selection).query()
}

/// Forwards sessions to the existing implementation without duplicating a codec.
#[neocogi_com::com_object(ISerializationFormat)]
struct Selected;
impl Drop for Selected {
    fn drop(&mut self) {
        assert_eq!(ROOTS.load(Ordering::Relaxed), 1, "root outlives its selected factory");
        FORMATS.fetch_sub(1, Ordering::Relaxed);
    }
}
impl ISerializationFormat for Selected {
    fn create_serializer(&self, bytes: u64, elements: u32, depth: u32, values: u64) -> Result<ComBox<ISerializerAbi>, NcStatus> {
        retained();
        neocogi_com_serialization::ncb::factory().create_serializer(bytes, elements, depth, values)
    }
    fn create_deserializer(
        &self,
        input: NcCopyFromBufferOffset<'_, u8>,
        bytes: u64,
        elements: u32,
        depth: u32,
        values: u64,
    ) -> Result<ComBox<IDeserializerAbi>, NcStatus> {
        retained();
        neocogi_com_serialization::ncb::factory().create_deserializer(input, bytes, elements, depth, values)
    }
}

/// Checks both selected owners while reentering host services from a callback.
fn retained() {
    assert_eq!(ROOTS.load(Ordering::Relaxed), 1);
    assert_eq!(FORMATS.load(Ordering::Relaxed), 1);
    reenter_catalog(crate::initialize_runtime());
}

/// Fixed semantic leaf used by real capture and reconstruction calls.
#[neocogi_com::com_object(IPersistable)]
struct Value;
impl IPersistable for Value {
    fn type_id(&self) -> Result<NcGuid, NcStatus> {
        Ok(KIND)
    }
    fn schema_version(&self) -> Result<u32, NcStatus> {
        Ok(1)
    }
    fn save_state(&self) -> Result<ComPtr<INcBytesAbi>, NcStatus> {
        Ok(NcBytes::from_vec(vec![42]))
    }
    fn child_count(&self) -> Result<u32, NcStatus> {
        Ok(0)
    }
    fn child(&self, _: u32) -> Result<ComPtr<IUnknown>, NcStatus> {
        Err(neocogi_com::NC_E_OUT_OF_RANGE)
    }
}

/// Validates copied initialization while observing retained selection capabilities.
#[neocogi_com::com_object(IPersistenceFactory)]
struct Factory;
impl IPersistenceFactory for Factory {
    fn validate(&self, version: u32, state: NcCopyFromBufferOffset<'_, u8>, count: u32) -> Result<(), NcStatus> {
        VALIDATIONS.fetch_add(1, Ordering::Relaxed);
        retained();
        if version == 1 && state.offset() == 0 && state.as_slice() == [42] && count == 0 {
            Ok(())
        } else {
            Err(neocogi_com::NC_E_INVALID_ARG)
        }
    }
    fn restore(&self, version: u32, state: NcCopyFromBufferOffset<'_, u8>, children: ComRef<'_, IObjectChildrenAbi>) -> Result<ComPtr<IUnknown>, NcStatus> {
        self.validate(version, state, children.count()?)?;
        Value::from_value(Value).query()
    }
}

/// A valid borrowed exclusive object cannot supply shared capture ownership.
#[neocogi_com::com_object(IObjectChildren, ownership = exclusive)]
struct Exclusive;
impl IObjectChildren for Exclusive {
    fn count(&self) -> Result<u32, NcStatus> {
        Ok(0)
    }
    fn get(&self, _: u32) -> Result<ComPtr<IUnknown>, NcStatus> {
        Err(neocogi_com::NC_E_OUT_OF_RANGE)
    }
}

/// Exact NCB-v1 oracle from the existing implementation, used only as a dev fixture.
fn bytes() -> Vec<u8> {
    neocogi_com_serialization::ncb::to_vec(&ObjectRecord {
        type_id: KIND.to_uuid_bytes(),
        schema_version: 1,
        state: vec![42],
        children: vec![],
    })
    .unwrap()
}

/// Creates the caller-owned recipe context without selecting a serialization provider.
fn registry(runtime: &neocogi_com::Runtime) -> FactoryRegistry {
    let mut registry = FactoryRegistry::new(runtime).unwrap();
    registry.register(KIND, Factory::from_value(Factory).query().unwrap()).unwrap();
    registry
}

/// Missing codec errors follow argument, ownership and capture-budget admission.
#[test]
fn missing_provider_preserves_error_precedence() {
    if isolated("missing_provider_preserves_error_precedence") {
        return;
    }
    let runtime = crate::initialize_runtime();
    let value = Value::from_value(Value);
    let registry = registry(&runtime);
    assert_eq!(snapshot(&runtime, value.as_com_ref()), Err(neocogi_com::NC_E_NOT_FOUND));
    assert_eq!(registry.restore(&bytes()).unwrap_err(), neocogi_com::NC_E_NOT_FOUND);
    assert_eq!(registry.restore(b"malformed").unwrap_err(), neocogi_com::NC_E_NOT_FOUND);
    assert_eq!(
        snapshot_with_limits(&runtime, value.as_com_ref(), Limits { objects: 0, ..Limits::default() }),
        Err(neocogi_com::NC_E_OUT_OF_RANGE)
    );
    assert_eq!(
        snapshot_with_limits(
            &runtime,
            value.as_com_ref(),
            Limits {
                binary: Limits::default().binary,
                depth: 0,
                ..Limits::default()
            }
        ),
        Err(neocogi_com::NC_E_OUT_OF_RANGE)
    );
    let mut limits = Limits::default();
    limits.binary.bytes = 0;
    assert_eq!(snapshot_with_limits(&runtime, value.as_com_ref(), limits), Err(neocogi_com::NC_E_OUT_OF_RANGE));
    assert_eq!(registry.restore_with_limits(&bytes(), limits).unwrap_err(), neocogi_com::NC_E_INVALID_ARG);
    let exclusive = Exclusive::into_box(Exclusive);
    let borrowed = exclusive.query_weak::<IUnknown>().unwrap();
    assert_eq!(snapshot(&runtime, borrowed.as_com_ref()), Err(neocogi_com::NC_E_INVALID_ARG));
    assert_eq!(
        snapshot(&runtime, Factory::from_value(Factory).as_com_ref()),
        Err(neocogi_com::NC_E_NOINTERFACE)
    );
    assert_eq!(VALIDATIONS.load(Ordering::Relaxed), 0);
    assert!(runtime.plugins().unwrap().is_empty(), "persistence must never seed the catalog");
}

/// JSON defaults never change persistence bytes; both directions select named NCB.
#[test]
fn named_ncb_retains_capabilities_and_ignores_json_default() {
    if isolated("named_ncb_retains_capabilities_and_ignores_json_default") {
        return;
    }
    let runtime = crate::initialize_runtime();
    // SAFETY: All descriptor metadata and callback code are linked process statics.
    unsafe { runtime.register_plugin_descriptor(plugin_descriptor()) }.unwrap();
    let value = Value::from_value(Value);
    let registry = registry(&runtime);
    let saved = snapshot(&runtime, value.as_com_ref()).unwrap();
    assert_eq!(saved, bytes(), "preserve the complete NCB-v1 envelope");
    let restored = registry.restore(&saved).unwrap();
    assert_eq!(restored.query::<IPersistableAbi>().unwrap().schema_version().unwrap(), 1);
    assert_eq!(SELECTIONS.load(Ordering::Relaxed), 2);
    assert_eq!(DEFAULTS.load(Ordering::Relaxed), 0);
    assert_eq!(VALIDATIONS.load(Ordering::Relaxed), 2);
    assert_eq!((ROOTS.load(Ordering::Relaxed), FORMATS.load(Ordering::Relaxed)), (0, 0));

    // The persistence ABI's usize policy remains permissive on 64-bit hosts,
    // even though format factories accept only u32 container/depth ceilings.
    let mut permissive = Limits::default();
    permissive.binary.elements = usize::MAX;
    permissive.binary.depth = usize::MAX;
    assert_eq!(snapshot_with_limits(&runtime, value.as_com_ref(), permissive).unwrap(), saved);
    drop(registry.restore_with_limits(&saved, permissive).unwrap());
    let validations = VALIDATIONS.load(Ordering::Relaxed);

    // A malformed document releases the selected capabilities before returning
    // and never invokes recipes. Every truncation and trailing data must fail.
    for end in 0..saved.len() {
        assert_eq!(registry.restore(&saved[..end]).unwrap_err(), neocogi_com::NC_E_INVALID_ARG);
    }
    let mut trailing = saved;
    trailing.push(0);
    assert_eq!(registry.restore(&trailing).unwrap_err(), neocogi_com::NC_E_INVALID_ARG);
    assert_eq!(VALIDATIONS.load(Ordering::Relaxed), validations);
    assert_eq!((ROOTS.load(Ordering::Relaxed), FORMATS.load(Ordering::Relaxed)), (0, 0));
}

/// Selection statuses propagate before codec error mapping or recipe callbacks.
#[test]
fn format_selection_errors_have_no_fallback() {
    if isolated("format_selection_errors_have_no_fallback") {
        return;
    }
    let runtime = crate::initialize_runtime();
    // SAFETY: All descriptor metadata and callback code are linked process statics.
    unsafe { runtime.register_plugin_descriptor(plugin_descriptor()) }.unwrap();
    let registry = registry(&runtime);
    let value = Value::from_value(Value);
    for (mode, status) in [
        (1, neocogi_com::NC_E_NOINTERFACE),
        (2, neocogi_com::NC_E_BUSY),
        (3, neocogi_com::NC_E_OUT_OF_RANGE),
    ] {
        FAILURE.store(mode, Ordering::Relaxed);
        assert_eq!(snapshot(&runtime, value.as_com_ref()), Err(status));
        assert_eq!(registry.restore(&bytes()).unwrap_err(), status);
        assert_eq!((ROOTS.load(Ordering::Relaxed), FORMATS.load(Ordering::Relaxed)), (0, 0));
    }
    assert_eq!(SELECTIONS.load(Ordering::Relaxed), 6);
    assert_eq!(DEFAULTS.load(Ordering::Relaxed), 0);
    assert_eq!(VALIDATIONS.load(Ordering::Relaxed), 0);
}
