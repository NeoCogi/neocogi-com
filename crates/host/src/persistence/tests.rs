//! Isolated processes exercise provider selection through the real host C tables.

use std::{
    cell::Cell,
    rc::Rc,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};
use neocogi_com_sdk::{ComBox, ComBytesExt, ComPtr, ComRef, INcBytesAbi, IUnknown, NcBytes, NcCopyFromBufferOffset, NcGuid, NcStatus};
use neocogi_com_sdk::persistence::*;
use neocogi_com_sdk::persistence::Limits;
use neocogi_com_sdk::serialization::*;

mod stream_writer;

/// Explicit semantic recipe, independent of the serialization plugin identity.
const KIND: NcGuid = neocogi_com_sdk::guid_from_qualified_name("nc.tests.host.codec-delegation");
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
/// Refuses creation of the selected provider's owning input session.
static REFUSE_SOURCE: AtomicBool = AtomicBool::new(false);
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
fn reenter_catalog(runtime: neocogi_com_sdk::RuntimeHandle) {
    // SAFETY: The immutable descriptor and its callbacks are linked into this process.
    unsafe { runtime.register_plugin_descriptor(marker::plugin_descriptor()) }.unwrap();
}

/// An unrelated catalog entry published reentrantly by the serialization provider.
mod marker {
    use super::*;
    /// Its factory is never invoked; only registration exercises the catalog lock.
    #[neocogi_com_sdk::plugin(name = "nc.tests.host.reentrant-serialization")]
    fn acquire(_: neocogi_com_sdk::RuntimeHandle) -> Result<ComPtr<ISerializationPluginAbi>, NcStatus> {
        Err(neocogi_com_sdk::NC_E_BUSY)
    }
}

/// A JSON-default provider with an independently observable named NCB capability.
#[neocogi_com_sdk::com_object(ISerializationPlugin)]
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
            1 => Err(neocogi_com_sdk::NC_E_NOINTERFACE),
            2 => Err(neocogi_com_sdk::NC_E_BUSY),
            _ => Err(neocogi_com_sdk::NC_E_OUT_OF_RANGE),
        }
    }
}

/// Selects no other runtime: bootstrap uses the host access supplied to the plugin.
#[neocogi_com_sdk::plugin(name = "nc.serialization")]
fn acquire(runtime: neocogi_com_sdk::RuntimeHandle) -> Result<ComPtr<ISerializationPluginAbi>, NcStatus> {
    reenter_catalog(runtime);
    ROOTS.fetch_add(1, Ordering::Relaxed);
    Selection::from_value(Selection).query()
}

/// Forwards sessions to the existing implementation without duplicating a codec.
#[neocogi_com_sdk::com_object(ISerializationFormat)]
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
        let mode = stream_writer::MODE.with(Cell::get);
        if mode != stream_writer::WriterMode::Production {
            let binary = neocogi_com_sdk::serialization::Limits {
                bytes: usize::try_from(bytes).map_err(|_| neocogi_com_sdk::NC_E_OUT_OF_RANGE)?,
                elements: elements as usize,
                depth: depth as usize,
                values: usize::try_from(values).map_err(|_| neocogi_com_sdk::NC_E_OUT_OF_RANGE)?,
            };
            return stream_writer::Writer::create(mode, binary);
        }
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
        if REFUSE_SOURCE.load(Ordering::Relaxed) {
            return Err(neocogi_com_sdk::NC_E_OUT_OF_MEMORY);
        }
        neocogi_com_serialization::ncb::factory().create_deserializer(input, bytes, elements, depth, values)
    }
}

/// Checks both selected owners while reentering host services from a callback.
fn retained() {
    assert_eq!(ROOTS.load(Ordering::Relaxed), 1);
    assert_eq!(FORMATS.load(Ordering::Relaxed), 1);
    reenter_catalog(crate::initialize_runtime());
}

/// Frozen semantic leaf used by real save and reconstruction calls.
#[neocogi_com_sdk::com_object(IPersistable)]
struct Value {
    /// Frozen recipe version also determines the state payload.
    revision: u32,
    /// Optional live model that advances when the saved view is read.
    live_revision: Option<Rc<Cell<u32>>>,
}
impl Value {
    /// Makes a standalone immutable saved view of the given revision.
    fn new(revision: u32) -> Self {
        Self { revision, live_revision: None }
    }
}
impl IPersistable for Value {
    fn type_id(&self) -> Result<NcGuid, NcStatus> {
        Ok(KIND)
    }
    fn schema_version(&self) -> Result<u32, NcStatus> {
        Ok(self.revision)
    }
    fn save_state(&self) -> Result<ComPtr<INcBytesAbi>, NcStatus> {
        if let Some(live) = &self.live_revision {
            live.set(2);
        }
        Ok(NcBytes::from_vec(vec![41 + self.revision as u8]))
    }
    fn child_count(&self) -> Result<u32, NcStatus> {
        Ok(0)
    }
    fn child(&self, _: u32) -> Result<ComPtr<IUnknown>, NcStatus> {
        Err(neocogi_com_sdk::NC_E_OUT_OF_RANGE)
    }
}

/// Validates copied initialization while observing retained selection capabilities.
#[neocogi_com_sdk::com_object(IPersistenceFactory)]
struct Factory;
impl IPersistenceFactory for Factory {
    fn validate(&self, version: u32, state: NcCopyFromBufferOffset<'_, u8>, count: u32) -> Result<(), NcStatus> {
        VALIDATIONS.fetch_add(1, Ordering::Relaxed);
        retained();
        if matches!((version, state.as_slice()), (1, [42]) | (2, [43])) && state.offset() == 0 && count == 0 {
            Ok(())
        } else {
            Err(neocogi_com_sdk::NC_E_INVALID_ARG)
        }
    }
    fn restore(&self, version: u32, state: NcCopyFromBufferOffset<'_, u8>, children: ComRef<'_, IObjectChildrenAbi>) -> Result<ComPtr<IUnknown>, NcStatus> {
        self.validate(version, state, children.count()?)?;
        Value::from_value(Value::new(version)).query()
    }
}

/// Exact NCB-v1 oracle from the existing implementation, used only as a dev fixture.
fn bytes() -> Vec<u8> {
    neocogi_com_serialization::ncb::to_vec(&SavedObject {
        type_id: KIND.to_uuid_bytes(),
        schema_version: 1,
        state: vec![42],
        children: vec![],
    })
    .unwrap()
}

/// Creates the caller-owned recipe context without selecting a serialization provider.
fn registry(runtime: &neocogi_com_sdk::RuntimeHandle) -> FactoryRegistry {
    let mut registry = FactoryRegistry::new(runtime).unwrap();
    registry.register(KIND, Factory::from_value(Factory).query().unwrap()).unwrap();
    registry
}

/// Missing codec errors follow root admission and precede live getters.
#[test]
fn missing_provider_preserves_error_precedence() {
    if isolated("missing_provider_preserves_error_precedence") {
        return;
    }
    let runtime = crate::initialize_runtime();
    let value = Value::from_value(Value::new(1));
    let registry = registry(&runtime);
    assert_eq!(save_object_tree(&runtime, &value), Err(neocogi_com_sdk::NC_E_NOT_FOUND));
    assert_eq!(registry.restore_object_tree(&bytes()).unwrap_err(), neocogi_com_sdk::NC_E_NOT_FOUND);
    assert_eq!(registry.restore_object_tree(b"malformed").unwrap_err(), neocogi_com_sdk::NC_E_NOT_FOUND);
    assert_eq!(
        registry
            .restore_object_tree_with_limits(&bytes(), Limits { objects: 0, ..Limits::default() })
            .unwrap_err(),
        neocogi_com_sdk::NC_E_NOT_FOUND
    );
    assert_eq!(
        save_object_tree_with_limits(&runtime, &value, Limits { objects: 0, ..Limits::default() }),
        Err(neocogi_com_sdk::NC_E_OUT_OF_RANGE)
    );
    assert_eq!(
        save_object_tree_with_limits(
            &runtime,
            &value,
            Limits {
                binary: Limits::default().binary,
                depth: 0,
                ..Limits::default()
            }
        ),
        Err(neocogi_com_sdk::NC_E_OUT_OF_RANGE)
    );
    let mut limits = Limits::default();
    limits.binary.bytes = 0;
    assert_eq!(save_object_tree_with_limits(&runtime, &value, limits), Err(neocogi_com_sdk::NC_E_NOT_FOUND));
    assert_eq!(
        registry.restore_object_tree_with_limits(&bytes(), limits).unwrap_err(),
        neocogi_com_sdk::NC_E_INVALID_ARG
    );
    assert_eq!(save_object_tree(&runtime, &Factory::from_value(Factory)), Err(neocogi_com_sdk::NC_E_NOT_FOUND));
    assert_eq!(VALIDATIONS.load(Ordering::Relaxed), 0);
    assert!(runtime.plugins().unwrap().is_empty(), "persistence must never seed the catalog");
}

/// Caller-owned observation of actual byte delivery across the persistence ABI.
#[derive(Default)]
struct SaveReceipt {
    /// Successful deliveries; the host must call exactly once for a complete document.
    calls: usize,
    /// Copied bytes, independent of the runtime's temporary output allocation.
    bytes: Vec<u8>,
}

/// Copies one complete document while the raw ABI lends its storage.
///
/// # Safety
/// `data` exclusively borrows a live SaveReceipt until this callback returns.
/// Nonempty `bytes` is readable for `len` bytes and does not overlap that receipt.
unsafe extern "C" fn receive_saved_document(data: *mut core::ffi::c_void, bytes: *const u8, len: usize) -> NcStatus {
    if data.is_null() || (len != 0 && bytes.is_null()) {
        return neocogi_com_sdk::NC_E_POINTER;
    }
    // SAFETY: The test lends this exact receipt exclusively for the synchronous call.
    let receipt = unsafe { &mut *data.cast::<SaveReceipt>() };
    if receipt.calls != 0 {
        return neocogi_com_sdk::NC_E_INVALID_ARG;
    }
    let bytes = match len {
        0 => &[],
        // SAFETY: The runtime keeps its complete document alive through delivery.
        _ => unsafe { core::slice::from_raw_parts(bytes, len) },
    };
    receipt.bytes.extend_from_slice(bytes);
    receipt.calls += 1;
    neocogi_com_sdk::NC_OK
}

/// ABI admission and recipe registration do not require a codec. Supplying one
/// after failed calls recovers the same context without replacing its recipes.
#[test]
fn provider_registration_recovers_existing_context_through_raw_api() {
    if isolated("provider_registration_recovers_existing_context_through_raw_api") {
        return;
    }
    let host = crate::initialize_process();
    // SAFETY: This host lends its complete immutable process-lived service graph.
    assert_eq!(unsafe { neocogi_com_sdk::validate_host_api(host) }, neocogi_com_sdk::NC_OK);
    // SAFETY: The admitted persistence table remains mapped for the entire test.
    let api = unsafe { &*host.persistence };
    let runtime = crate::initialize_runtime();
    let value: ComPtr<IUnknown> = Value::from_value(Value::new(1)).query().unwrap();
    let factory: ComPtr<IPersistenceFactoryAbi> = Factory::from_value(Factory).query().unwrap();
    let expected = bytes();
    let limits = Limits::default().into();
    let mut context = core::ptr::null_mut();
    // SAFETY: Valid output storage and a live borrowed factory are supplied on
    // this one thread; registration exclusively borrows the new context.
    unsafe {
        assert_eq!((api.create_context)(&mut context), neocogi_com_sdk::NC_OK);
        assert!(!context.is_null());
        assert_eq!((api.register_factory)(context, KIND, factory.as_raw()), neocogi_com_sdk::NC_OK);
    }
    drop(factory); // Only the context's retained factory can service the later retry.
    let mut receipt = SaveReceipt::default();
    // A live borrowed pair is a valid output sentinel; failure must clear both
    // words without consuming this owner's separate reference.
    let mut root = value.as_raw();
    // SAFETY: Owners, context, input bytes, output and receipt remain live for
    // each synchronous call; inputs do not overlap either writable output.
    unsafe {
        assert_eq!(
            (api.save_object_tree)(value.as_raw(), limits, Some(receive_saved_document), core::ptr::from_mut(&mut receipt).cast()),
            neocogi_com_sdk::NC_E_NOT_FOUND
        );
        assert_eq!(
            (api.restore_object_tree)(context, expected.as_ptr(), expected.len(), limits, &mut root),
            neocogi_com_sdk::NC_E_NOT_FOUND
        );
    }
    assert!(root.object.is_null() && root.vtable.is_null());
    assert_eq!(receipt.calls, 0);
    assert!(receipt.bytes.is_empty());
    assert_eq!(VALIDATIONS.load(Ordering::Relaxed), 0);
    assert!(runtime.plugins().unwrap().is_empty(), "failed persistence must not register a fallback");

    // SAFETY: The descriptor and all provider callbacks are linked process statics.
    unsafe { runtime.register_plugin_descriptor(plugin_descriptor()) }.unwrap();
    let mut short = Limits::default();
    short.binary.bytes = 3;
    // SAFETY: Both owners are live and the callback borrows the receipt only
    // for this synchronous call, with no overlapping writable storage.
    assert_eq!(
        unsafe {
            (api.save_object_tree)(
                value.as_raw(),
                short.into(),
                Some(receive_saved_document),
                core::ptr::from_mut(&mut receipt).cast(),
            )
        },
        neocogi_com_sdk::NC_E_OUT_OF_RANGE
    );
    assert_eq!(SELECTIONS.load(Ordering::Relaxed), 1);
    assert_eq!((ROOTS.load(Ordering::Relaxed), FORMATS.load(Ordering::Relaxed)), (0, 0));
    assert_eq!(receipt.calls, 0);
    assert!(receipt.bytes.is_empty());

    // SAFETY: The same live context still owns its recipe; value, receipt and
    // output storage satisfy the unchanged synchronous C calling contract.
    unsafe {
        assert_eq!(
            (api.save_object_tree)(value.as_raw(), limits, Some(receive_saved_document), core::ptr::from_mut(&mut receipt).cast()),
            neocogi_com_sdk::NC_OK
        );
        assert_eq!(
            (api.restore_object_tree)(context, receipt.bytes.as_ptr(), receipt.bytes.len(), limits, &mut root),
            neocogi_com_sdk::NC_OK
        );
        // No call still borrows the unique context, so its retained factory can go.
        (api.destroy_context)(context);
    }
    assert_eq!(receipt.calls, 1);
    assert_eq!(receipt.bytes, expected, "named NCB retains the complete v1 document");
    // SAFETY: Successful restore transferred exactly one owned common COM pair.
    let restored = unsafe { ComPtr::<IUnknown>::__nc_from_raw_owned_checked(root) }.unwrap();
    assert_eq!(restored.query::<IPersistableAbi>().unwrap().schema_version().unwrap(), 1);
    assert_eq!(SELECTIONS.load(Ordering::Relaxed), 3);
    assert_eq!(DEFAULTS.load(Ordering::Relaxed), 0);
    assert_eq!(VALIDATIONS.load(Ordering::Relaxed), 2);
    assert_eq!((ROOTS.load(Ordering::Relaxed), FORMATS.load(Ordering::Relaxed)), (0, 0));
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
    let value = Value::from_value(Value::new(1));
    let registry = registry(&runtime);
    let saved = save_object_tree(&runtime, &value).unwrap();
    assert_eq!(saved, bytes(), "preserve the complete NCB-v1 envelope");
    let restored = registry.restore_object_tree(&saved).unwrap();
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
    assert_eq!(save_object_tree_with_limits(&runtime, &value, permissive).unwrap(), saved);
    drop(registry.restore_object_tree_with_limits(&saved, permissive).unwrap());
    let validations = VALIDATIONS.load(Ordering::Relaxed);

    // Writer creation rejects the byte cap before entering any live getter.
    let selections = SELECTIONS.load(Ordering::Relaxed);
    let mut short = Limits::default();
    short.binary.bytes = 3;
    assert_eq!(save_object_tree_with_limits(&runtime, &value, short), Err(neocogi_com_sdk::NC_E_OUT_OF_RANGE));
    assert_eq!(SELECTIONS.load(Ordering::Relaxed), selections + 1);
    assert_eq!((ROOTS.load(Ordering::Relaxed), FORMATS.load(Ordering::Relaxed)), (0, 0));

    // A malformed document releases the selected capabilities before returning
    // and never invokes recipes. Every truncation and trailing data must fail.
    for end in 0..saved.len() {
        assert_eq!(registry.restore_object_tree(&saved[..end]).unwrap_err(), neocogi_com_sdk::NC_E_INVALID_ARG);
    }
    let mut trailing = saved;
    trailing.push(0);
    assert_eq!(registry.restore_object_tree(&trailing).unwrap_err(), neocogi_com_sdk::NC_E_INVALID_ARG);
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
    let value = Value::from_value(Value::new(1));
    for (mode, status) in [
        (1, neocogi_com_sdk::NC_E_NOINTERFACE),
        (2, neocogi_com_sdk::NC_E_BUSY),
        (3, neocogi_com_sdk::NC_E_OUT_OF_RANGE),
    ] {
        FAILURE.store(mode, Ordering::Relaxed);
        assert_eq!(save_object_tree(&runtime, &value), Err(status));
        assert_eq!(registry.restore_object_tree(&bytes()).unwrap_err(), status);
        assert_eq!((ROOTS.load(Ordering::Relaxed), FORMATS.load(Ordering::Relaxed)), (0, 0));
    }
    assert_eq!(SELECTIONS.load(Ordering::Relaxed), 6);
    assert_eq!(DEFAULTS.load(Ordering::Relaxed), 0);
    assert_eq!(VALIDATIONS.load(Ordering::Relaxed), 0);
    FAILURE.store(0, Ordering::Relaxed);
    REFUSE_SOURCE.store(true, Ordering::Relaxed);
    assert_eq!(registry.restore_object_tree(&bytes()).unwrap_err(), neocogi_com_sdk::NC_E_INVALID_ARG);
    assert_eq!(VALIDATIONS.load(Ordering::Relaxed), 0);
    assert_eq!((ROOTS.load(Ordering::Relaxed), FORMATS.load(Ordering::Relaxed)), (0, 0));
    REFUSE_SOURCE.store(false, Ordering::Relaxed);
    assert!(registry.restore_object_tree(&bytes()).is_ok());
}

/// The application can save an immutable view even when its live model advances
/// during a provider call. Both the old and new views reconstruct independently.
#[test]
fn frozen_view_restores_as_live_model_advances() {
    if isolated("frozen_view_restores_as_live_model_advances") {
        return;
    }
    let runtime = crate::initialize_runtime();
    // SAFETY: The descriptor and all reachable callbacks are linked process statics.
    unsafe { runtime.register_plugin_descriptor(plugin_descriptor()) }.unwrap();
    let registry = registry(&runtime);
    let live_revision = Rc::new(Cell::new(1));
    let first = Value::from_value(Value {
        revision: live_revision.get(),
        live_revision: Some(live_revision.clone()),
    });
    let first = save_object_tree(&runtime, &first).unwrap();
    assert_eq!(live_revision.get(), 2);
    let second = Value::from_value(Value::new(live_revision.get()));
    let second = save_object_tree(&runtime, &second).unwrap();

    for (expected, document) in [(1, first), (2, second)] {
        let restored = registry.restore_object_tree(&document).unwrap();
        let persistable = restored.query::<IPersistableAbi>().unwrap();
        assert_eq!(persistable.schema_version().unwrap(), expected);
        assert_eq!(persistable.save_state().unwrap().to_vec().unwrap(), [41 + expected as u8]);
    }
}
