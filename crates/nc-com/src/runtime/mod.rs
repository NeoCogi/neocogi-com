//! Safe Rust facade over the linked runtime's versioned C tables.
//!
//! `Runtime` is stateless: the linked `libnc_com_shared_runtime` owns all process-wide
//! data. The facade checks the mandatory runtime ABI generation before every
//! table lookup, validates each table's advertised size, and converts borrowed C
//! records into owned Rust values. Context views keep registry enumeration,
//! relationship inspection, and diagnostics separate from mutation operations.

use std::fmt;
use std::path::{Path, PathBuf};
use std::string::String;
use std::vec::Vec;

use crate::{
    ComClassContract, ComInterface, ComPtr, IUnknown, NC_COM_RUNTIME_ABI_VERSION,
    NC_E_INSUFFICIENT_BUFFER,
    NC_E_INVALID_ARG, NC_E_NOT_FOUND, NC_E_OUT_OF_MEMORY, NC_E_OUT_OF_RANGE, NC_E_POINTER,
    NcDebugApiV1, NcGuid, NcInspectionApiV1, NcInterfaceId, NcPluginDescriptor, NcPluginRegistryV1,
    NcRuntimeApiV1, NcStatus, NcTypeRegistryV1, NcUtf8View, ThreadLocal, ThreadingModel,
};
use nc_com_runtime_sys as runtime_sys;

mod debug;
mod inspection;
mod plugin_registry;
mod type_registry;

pub use debug::{Debug, DebugMessenger};
pub use inspection::{ClassInfo, Inspection, PluginInfo};
pub use plugin_registry::PluginRegistry;
pub use type_registry::TypeRegistry;

/// Maximum number of elements copied by one Rust runtime-facade enumeration.
///
/// The raw C count/fill ABI remains caller-controlled. This host-side limit
/// prevents an incompatible or broken runtime table from turning a reported
/// count directly into an unbounded Rust allocation.
pub const NC_COM_RUNTIME_MAX_ENUMERATION_ITEMS: usize = 65_536;

/// Maximum number of retries after the initial runtime enumeration fill call.
///
/// Append-only registration may legitimately race an enumeration. After this
/// many strictly growing retries, the facade returns
/// [`NC_E_INSUFFICIENT_BUFFER`] so callers can retry the complete operation
/// later instead of chasing an indefinitely growing registry.
pub const NC_COM_RUNTIME_MAX_ENUMERATION_RETRIES: usize = 8;

/// Maximum UTF-8 byte length copied from one runtime inspection view.
pub const NC_COM_RUNTIME_MAX_INSPECTION_TEXT_BYTES: usize = 1024 * 1024;

/// Handle for the process-global nc-com runtime.
///
/// The registry and all API tables are owned by the linked
/// `libnc_com_shared_runtime` shared library.
#[derive(Clone, Copy, Debug, Default)]
pub struct Runtime {
    /// Prevents external struct literals while keeping the handle zero-sized.
    _private: (),
}

/// Process-local identifier for a plugin library retained by the runtime.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LoadedModuleId(u64);

impl LoadedModuleId {
    /// Wraps a nonzero raw module handle returned by the runtime ABI.
    const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the process-local numeric module identifier.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for LoadedModuleId {
    /// Formats the process-local numeric identifier in decimal form.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Owned summary of a plugin library successfully loaded into the runtime.
///
/// The runtime ABI stores module provenance and plugin metadata separately:
/// module inspection supplies the module ID, path, and plugin ID, while plugin
/// inspection supplies the descriptor's qualified name and version. This
/// facade joins those records and copies their text, so the summary holds no
/// registry lock and borrows no descriptor memory.
///
/// Use [`Inspection::loaded_module`] to retrieve the summary again by
/// [`LoadedModuleId`]. Use [`Inspection::plugin_info`] with [`Self::plugin_id`]
/// when only plugin metadata is needed, including metadata for plugins that
/// were registered statically and therefore have no loaded module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedModuleInfo {
    /// Process-local identifier assigned to the retained library.
    module_id: LoadedModuleId,
    /// Owned copy of the path reported by runtime module inspection.
    path: PathBuf,
    /// Stable plugin identity read from the validated descriptor.
    plugin_id: NcGuid,
    /// Owned copy of the plugin's canonical qualified name.
    qualified_name: String,
    /// Owned copy of the plugin descriptor's version text.
    plugin_version: String,
}

impl LoadedModuleInfo {
    /// Returns the process-local loaded module identifier.
    pub fn module_id(&self) -> LoadedModuleId {
        self.module_id
    }

    /// Returns the loaded library path reported by module inspection.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the plugin identifier from the descriptor.
    pub fn plugin_id(&self) -> NcGuid {
        self.plugin_id
    }

    /// Returns the plugin qualified name copied from descriptor-backed plugin
    /// inspection.
    pub fn qualified_name(&self) -> &str {
        &self.qualified_name
    }

    /// Returns the plugin version text copied from descriptor-backed plugin
    /// inspection.
    pub fn plugin_version(&self) -> &str {
        &self.plugin_version
    }
}

static RUNTIME: Runtime = Runtime { _private: () };

impl Runtime {
    /// Returns a handle to the process-global runtime.
    pub fn global() -> &'static Self {
        &RUNTIME
    }

    /// Returns the runtime shared-library ABI generation.
    pub fn runtime_abi_version(&self) -> u32 {
        // SAFETY: The unversioned version function is the stable discovery
        // entrypoint shared by runtime ABI generations.
        unsafe { runtime_sys::nc_com_runtime_abi_version() }
    }

    /// Returns the core plugin ABI version accepted by the linked runtime.
    pub fn core_abi_version(&self) -> u32 {
        self.runtime_api().core_abi_version
    }

    /// Returns the linked runtime-operation table.
    fn runtime_api(&self) -> &'static NcRuntimeApiV1 {
        // Do not cache a v1 table before checking the mandatory generation: a
        // future runtime may expose an incompatible discovery/table contract.
        self.assert_runtime_v1();
        // SAFETY: The runtime version was checked before calling the v1 getter,
        // whose result follows the static size-prefix table contract.
        let api = unsafe { table(runtime_sys::nc_com_get_runtime_api_v1()) };
        assert_eq!(api.core_abi_version, crate::NC_COM_CORE_ABI_VERSION);
        api
    }

    /// Returns the linked plugin-registry table.
    fn plugin_registry_api(&self) -> &'static NcPluginRegistryV1 {
        self.assert_runtime_v1();
        // SAFETY: The runtime version was checked before calling the v1 getter,
        // whose result follows the static size-prefix table contract.
        unsafe { table(runtime_sys::nc_com_get_plugin_registry_v1()) }
    }

    /// Returns the linked type-registry table.
    fn type_registry_api(&self) -> &'static NcTypeRegistryV1 {
        self.assert_runtime_v1();
        // SAFETY: The runtime version was checked before calling the v1 getter,
        // whose result follows the static size-prefix table contract.
        unsafe { table(runtime_sys::nc_com_get_type_registry_v1()) }
    }

    /// Returns the linked inspection table.
    fn inspection_api(&self) -> &'static NcInspectionApiV1 {
        self.assert_runtime_v1();
        // SAFETY: The runtime version was checked before calling the v1 getter,
        // whose result follows the static size-prefix table contract.
        unsafe { table(runtime_sys::nc_com_get_inspection_api_v1()) }
    }

    /// Returns the linked debug-messenger table.
    fn debug_api(&self) -> &'static NcDebugApiV1 {
        self.assert_runtime_v1();
        // SAFETY: The runtime version was checked before calling the v1 getter,
        // whose result follows the static size-prefix table contract.
        unsafe { table(runtime_sys::nc_com_get_debug_api_v1()) }
    }

    /// Returns the registered-plugin and loaded-module catalog.
    pub fn plugin_registry(&self) -> PluginRegistry<'_> {
        PluginRegistry::new(self)
    }

    /// Returns the registered class and interface catalog.
    pub fn type_registry(&self) -> TypeRegistry<'_> {
        TypeRegistry::new(self)
    }

    /// Returns runtime relationship and metadata inspection operations.
    pub fn inspection(&self) -> Inspection<'_> {
        Inspection::new(self)
    }

    /// Returns callback-based runtime diagnostics operations.
    pub fn debug(&self) -> Debug<'_> {
        Debug::new(self)
    }

    /// Registers a static plugin descriptor in the process-global runtime.
    ///
    /// Registration is one blocking validation, duplicate-check, and insertion
    /// transaction. It does not invoke plugin code or load dependencies.
    /// Declared hard dependency IDs must already be registered.
    ///
    /// # Safety
    ///
    /// `plugin` and its descriptor graph must remain readable while registered.
    pub unsafe fn register_plugin_descriptor(&self, plugin: *const NcPluginDescriptor) -> NcStatus {
        // SAFETY: The caller supplies the descriptor lifetime contract.
        unsafe { (self.runtime_api().register_plugin_descriptor)(plugin) }
    }

    /// Loads a trusted plugin shared library into the process-global runtime.
    ///
    /// The path must have a nonempty UTF-8 representation without interior NUL
    /// bytes. This is required live activation: any path conversion, loading,
    /// descriptor, dependency, or registration failure prints/emits its
    /// diagnostic and aborts the process. Failure never returns.
    /// Live activation calls must not be nested: native loader initialization
    /// and the plugin descriptor entrypoint must not directly or indirectly
    /// call this operation while another activation is in progress.
    /// The runtime retains at most
    /// [`crate::NC_COM_RUNTIME_MAX_LOADED_MODULES`] dynamic modules; attempting
    /// another load is also a fatal activation failure.
    ///
    /// The returned [`LoadedModuleInfo`] is an owned snapshot. Its path comes
    /// from module inspection; its qualified name and version come from the
    /// registered plugin descriptor through plugin inspection. Call
    /// [`Runtime::inspection`] to query the same module or plugin later.
    ///
    /// # Safety
    ///
    /// `path` must identify trusted nc-com plugin code.
    pub unsafe fn load_plugin_library(&self, path: impl AsRef<Path>) -> LoadedModuleInfo {
        let path = path.as_ref();
        let path = path
            .to_str()
            .unwrap_or_else(|| fatal_plugin_load(format_args!("path is not valid UTF-8: {}", path.display())));
        let mut module_id = 0;
        // SAFETY: The UTF-8 path is readable for this call and the output slot
        // is valid. The caller accepted the trusted-library requirement.
        let status = unsafe {
            (self.runtime_api().load_plugin_library_utf8)(path.as_ptr(), path.len(), &mut module_id)
        };
        if status.is_err() {
            fatal_plugin_load(status);
        }
        // The C load returns only an opaque module ID. Inspection supplies an
        // owned Rust summary without widening the C operation table.
        self.inspection()
            .loaded_module(LoadedModuleId::from_raw(module_id))
            .and_then(|module| module.ok_or(NC_E_NOT_FOUND))
            .unwrap_or_else(|status| fatal_plugin_load(status))
    }

    /// Loads trusted plugin libraries sequentially.
    ///
    /// ABI v1 performs no path discovery or recursive loading. Callers must
    /// supply a dependency-first order, such as one produced by
    /// `nc-com-inspect plan`. Native libraries mapped as
    /// platform dependencies are not automatically registered as nc-com
    /// plugins.
    ///
    /// A later failure aborts the process rather than returning partial state.
    /// The runtime ABI v1 intentionally has no transactional batch-loading call.
    ///
    /// # Safety
    ///
    /// Every path must identify trusted nc-com plugin code.
    pub unsafe fn load_plugin_libraries<I, P>(&self, paths: I) -> Vec<LoadedModuleInfo>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut loaded = Vec::new();
        for path in paths {
            // SAFETY: The caller accepted the trusted-library requirement for
            // every path supplied to this convenience helper.
            loaded.push(unsafe { self.load_plugin_library(path) });
        }
        loaded
    }

    /// Verifies that a registered class supports an interface ABI identity.
    pub fn require_interface(
        &self,
        class_id: NcGuid,
        interface_id: NcInterfaceId,
    ) -> Result<(), NcStatus> {
        // SAFETY: Both identities are passed by value.
        status_to_result(unsafe { (self.runtime_api().require_interface)(class_id, interface_id) })
    }

    /// Verifies that a registered class supports generated interface `T`.
    pub fn require_interface_type<T: ComInterface>(
        &self,
        class_id: NcGuid,
    ) -> Result<(), NcStatus> {
        self.require_interface(class_id, T::INTERFACE_ID)
    }

    /// Constructs the registered class described by callable contract `C`.
    ///
    /// The shared class contract supplies both registry identity and primary
    /// callable ABI, so the caller neither repeats a class ID nor queries a
    /// separately declared one-off interface. This entrypoint accepts the
    /// no-argument constructor contract represented by `C`.
    pub fn construct<C: ComClassContract>(&self) -> Result<ComPtr<C>, NcStatus> {
        // SAFETY: A no-argument class contract requires a null argument pointer.
        // `construct_class_as` validates the returned primary projection against
        // C's complete generated ABI identity before exposing it to safe code.
        unsafe { self.construct_class_as::<C>(C::CLASS_ID, core::ptr::null_mut()) }
    }

    /// Constructs an object from a registered class.
    ///
    /// # Safety
    ///
    /// `arguments`, when non-null, must be a valid borrowed `IUnknown` pointer.
    pub unsafe fn construct_class(
        &self,
        class_id: NcGuid,
        arguments: *mut IUnknown,
    ) -> Result<ComPtr<IUnknown>, NcStatus> {
        // SAFETY: This forwards the caller's borrowed argument contract while
        // requesting the conservative thread-local pointer capability.
        unsafe { self.construct_class_with_model::<ThreadLocal>(class_id, arguments) }
    }

    /// Constructs an object with threading capability `M`.
    ///
    /// A thread-local request is valid for every registered class. A
    /// thread-safe request succeeds only when the class descriptor promises
    /// that constructed objects support cross-thread calls and destruction.
    ///
    /// # Safety
    ///
    /// `arguments`, when non-null, must be a valid borrowed `IUnknown` pointer.
    pub unsafe fn construct_class_with_model<M: ThreadingModel>(
        &self,
        class_id: NcGuid,
        arguments: *mut IUnknown,
    ) -> Result<ComPtr<IUnknown, M>, NcStatus> {
        let mut out = core::ptr::null_mut();
        let requested_threading = M::ABI_THREADING_MODE.as_raw();
        // SAFETY: The caller supplies the borrowed argument contract and `out`
        // is valid for one pointer write.
        let status = unsafe {
            (self.runtime_api().construct_class)(class_id, requested_threading, arguments, &mut out)
        };
        status_to_result(status)?;
        // SAFETY: A successful runtime construction returns one owned reference.
        unsafe { ComPtr::<IUnknown, M>::__nc_from_raw_owned_checked(out) }
    }

    /// Constructs an object, then validates and queries generated interface `T`.
    ///
    /// # Safety
    ///
    /// `arguments`, when non-null, must be a valid borrowed `IUnknown` pointer.
    pub unsafe fn construct_class_as<T: ComInterface>(
        &self,
        class_id: NcGuid,
        arguments: *mut IUnknown,
    ) -> Result<ComPtr<T>, NcStatus> {
        // SAFETY: This forwards the caller's borrowed argument contract.
        let unknown = unsafe { self.construct_class(class_id, arguments)? };
        self.query_validated::<T, _>(class_id, &unknown)
    }

    /// Constructs an object with threading capability `M`, then validates and
    /// queries generated interface `T` while preserving that capability.
    ///
    /// # Safety
    ///
    /// `arguments`, when non-null, must be a valid borrowed `IUnknown` pointer.
    pub unsafe fn construct_class_as_with_model<T, M>(
        &self,
        class_id: NcGuid,
        arguments: *mut IUnknown,
    ) -> Result<ComPtr<T, M>, NcStatus>
    where
        T: ComInterface,
        M: ThreadingModel,
    {
        // SAFETY: This forwards the caller's borrowed argument contract.
        let unknown = unsafe { self.construct_class_with_model::<M>(class_id, arguments)? };
        self.query_validated::<T, M>(class_id, &unknown)
    }

    /// Validates and queries an existing object for generated interface `T`.
    pub fn query_validated<T, M>(
        &self,
        class_id: NcGuid,
        object: &ComPtr<IUnknown, M>,
    ) -> Result<ComPtr<T, M>, NcStatus>
    where
        T: ComInterface,
        M: ThreadingModel,
    {
        self.require_interface_type::<T>(class_id)?;
        object.query::<T>()
    }

    /// Adopts an owned `IUnknown`, then validates and queries interface `T`.
    ///
    /// # Safety
    ///
    /// `object` must be null or one owned `IUnknown` reference.
    pub unsafe fn adopt_unknown_as<T: ComInterface>(
        &self,
        class_id: NcGuid,
        object: *mut IUnknown,
    ) -> Result<ComPtr<T>, NcStatus> {
        // SAFETY: The caller transfers one owned reference when non-null.
        let unknown = unsafe { ComPtr::<IUnknown>::__nc_from_raw_owned_checked(object) }?;
        self.query_validated::<T, _>(class_id, &unknown)
    }

    /// Aborts the operation if the linked library does not implement ABI v1.
    fn assert_runtime_v1(&self) {
        // ABI mismatch is a linkage/configuration error, not a recoverable
        // plugin status. Fail before dereferencing any version-specific table.
        let found = self.runtime_abi_version();
        assert_eq!(
            found, NC_COM_RUNTIME_ABI_VERSION,
            "incompatible nc-com runtime ABI: expected {}, found {found}",
            NC_COM_RUNTIME_ABI_VERSION
        );
    }
}

/// Reports an unrecoverable live-activation failure and aborts the process.
fn fatal_plugin_load(detail: impl fmt::Display) -> ! {
    eprintln!("fatal nc-com plugin activation failure: {detail}");
    std::process::abort()
}

/// Converts a runtime-owned static table pointer into a checked shared reference.
///
/// # Safety
///
/// A non-null pointer must be aligned for `T` and refer to immutable storage that
/// remains live for the duration of the process. The storage must contain a
/// readable leading `usize` reporting its actual byte size. When that size is at
/// least `size_of::<T>()`, the known prefix must be a valid `T`.
unsafe fn table<T>(table: *const T) -> &'static T {
    assert!(
        !table.is_null(),
        "compatible nc-com runtime returned a null API table"
    );

    // Read only the common size prefix through the raw pointer. Creating `&T`
    // before this check would already require the backing allocation to contain
    // all of `T`, defeating append-only compatibility with an older short table.
    // SAFETY: The caller guarantees a non-null, aligned pointer readable for the
    // leading `usize`; null was rejected above.
    let found = unsafe { table.cast::<usize>().read() };
    assert_table_size(found, core::mem::size_of::<T>());

    // SAFETY: The size check established that the immutable static allocation
    // contains the complete known `T` prefix promised by the caller.
    unsafe { &*table }
}

/// Verifies that an append-only versioned table contains every required field.
fn assert_table_size(found: usize, required: usize) {
    // V1 tables may grow by appending fields. A larger table is compatible; a
    // shorter one cannot contain every function pointer this facade calls.
    assert!(
        found >= required,
        "compatible nc-com runtime returned a short API table: required {required} bytes, found {found}"
    );
}

/// Converts an ABI status to `Result` while preserving its exact error code.
fn status_to_result(status: NcStatus) -> Result<(), NcStatus> {
    // Preserve the exact status value so callers can distinguish pointer,
    // identity, interface, and ABI-version failures.
    if status.is_ok() { Ok(()) } else { Err(status) }
}

/// Reads a count/fill ABI enumeration into an initialized, bounded Rust vector.
///
/// Append-only registry growth is retried only while the required capacity
/// grows strictly and remains within the facade's resource policy.
fn read_vec<T: Copy>(
    count: impl Fn() -> usize,
    fill: impl Fn(*mut T, usize, *mut usize) -> NcStatus,
) -> Result<Vec<T>, NcStatus> {
    // Registry state is append-only and may grow after `count`. Retry only when
    // the required capacity makes strict progress, and never expose partially
    // initialized vectors.
    let mut capacity = checked_enumeration_capacity(count())?;
    let mut retries = 0;
    loop {
        let mut values = try_vec_with_capacity(capacity)?;
        let mut out_len = 0;
        let status = fill(values.as_mut_ptr(), capacity, &mut out_len);
        if status == NC_E_INSUFFICIENT_BUFFER {
            if retries == NC_COM_RUNTIME_MAX_ENUMERATION_RETRIES {
                return Err(NC_E_INSUFFICIENT_BUFFER);
            }
            // Re-read the count as well as trusting out_len so progress remains
            // monotonic if another registration races this retry.
            let required = out_len.max(count());
            if required <= capacity {
                return Err(NC_E_INSUFFICIENT_BUFFER);
            }
            capacity = checked_enumeration_capacity(required)?;
            retries += 1;
            continue;
        }
        status_to_result(status)?;
        if out_len > capacity {
            return Err(NC_E_INSUFFICIENT_BUFFER);
        }
        // SAFETY: The runtime wrote exactly `out_len <= capacity` initialized
        // values on success.
        unsafe { values.set_len(out_len) };
        return Ok(values);
    }
}

/// Rejects an ABI-reported element count above the host allocation policy.
fn checked_enumeration_capacity(capacity: usize) -> Result<usize, NcStatus> {
    if capacity > NC_COM_RUNTIME_MAX_ENUMERATION_ITEMS {
        Err(NC_E_OUT_OF_RANGE)
    } else {
        Ok(capacity)
    }
}

/// Reserves an exact vector capacity and maps allocation failure to ABI status.
fn try_vec_with_capacity<T>(capacity: usize) -> Result<Vec<T>, NcStatus> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(capacity)
        .map_err(|_| NC_E_OUT_OF_MEMORY)?;
    Ok(values)
}

/// Copies a bounded runtime-owned UTF-8 view into an owned Rust string.
fn read_utf8_view(view: NcUtf8View) -> Result<String, NcStatus> {
    if view.len == 0 {
        return Ok(String::new());
    }
    if view.len > NC_COM_RUNTIME_MAX_INSPECTION_TEXT_BYTES {
        return Err(NC_E_OUT_OF_RANGE);
    }
    if view.data.is_null() {
        return Err(NC_E_POINTER);
    }
    // Copy immediately: the C view is runtime-owned and has no Rust lifetime
    // that could safely escape this helper.
    // SAFETY: Runtime inspection views are readable for their advertised length
    // while the runtime remains loaded.
    let mut bytes = try_vec_with_capacity(view.len)?;
    unsafe {
        core::ptr::copy_nonoverlapping(view.data, bytes.as_mut_ptr(), view.len);
        bytes.set_len(view.len);
    }
    String::from_utf8(bytes).map_err(|_| NC_E_INVALID_ARG)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::panic::catch_unwind;

    use super::{
        NC_COM_RUNTIME_MAX_ENUMERATION_ITEMS, NC_COM_RUNTIME_MAX_ENUMERATION_RETRIES,
        NC_COM_RUNTIME_MAX_INSPECTION_TEXT_BYTES, read_utf8_view, read_vec, table,
    };
    use crate::{NC_E_INSUFFICIENT_BUFFER, NC_E_OUT_OF_RANGE, NC_OK, NcStatus, NcUtf8View};

    #[repr(C)]
    struct TestApiTable {
        size: usize,
        value: u32,
    }

    #[repr(C)]
    struct ExtendedTestApiTable {
        prefix: TestApiTable,
        appended: u64,
    }

    #[repr(C)]
    struct TestApiTableSizePrefix {
        size: usize,
    }

    static EXACT_TEST_API_TABLE: TestApiTable = TestApiTable {
        size: core::mem::size_of::<TestApiTable>(),
        value: 42,
    };

    static EXTENDED_TEST_API_TABLE: ExtendedTestApiTable = ExtendedTestApiTable {
        prefix: TestApiTable {
            size: core::mem::size_of::<ExtendedTestApiTable>(),
            value: 42,
        },
        appended: 73,
    };

    static SHORT_TEST_API_TABLE: TestApiTableSizePrefix = TestApiTableSizePrefix {
        size: core::mem::size_of::<TestApiTableSizePrefix>(),
    };

    #[test]
    fn table_accepts_an_exact_sized_static_table() {
        // SAFETY: The pointer addresses a complete immutable static table.
        let table = unsafe { table(core::ptr::from_ref(&EXACT_TEST_API_TABLE)) };

        assert_eq!(table.value, 42);
    }

    #[test]
    fn table_accepts_an_append_only_extension() {
        // SAFETY: The extended allocation begins with a complete `TestApiTable`
        // prefix and remains immutable for the process lifetime.
        let table =
            unsafe { table(core::ptr::from_ref(&EXTENDED_TEST_API_TABLE).cast::<TestApiTable>()) };

        assert_eq!(table.value, 42);
    }

    #[test]
    fn table_rejects_a_null_pointer() {
        let result = catch_unwind(|| {
            // SAFETY: Null is an explicitly supported defensive input that must
            // be rejected before dereference.
            unsafe { table::<TestApiTable>(core::ptr::null()) }
        });

        assert!(result.is_err());
    }

    #[test]
    fn table_rejects_a_short_allocation_before_forming_a_full_reference() {
        let result = catch_unwind(|| {
            // SAFETY: The allocation is immutable and process-static, aligned for
            // the table prefix, and readable for its advertised leading `size`.
            // It intentionally does not contain the remainder of `TestApiTable`;
            // the checked conversion must reject it using only that prefix.
            unsafe {
                table::<TestApiTable>(
                    core::ptr::from_ref(&SHORT_TEST_API_TABLE).cast::<TestApiTable>(),
                )
            }
        });

        assert!(result.is_err());
    }

    #[test]
    fn runtime_facade_resource_policy_values_are_stable() {
        assert_eq!(NC_COM_RUNTIME_MAX_ENUMERATION_ITEMS, 65_536);
        assert_eq!(NC_COM_RUNTIME_MAX_ENUMERATION_RETRIES, 8);
        assert_eq!(NC_COM_RUNTIME_MAX_INSPECTION_TEXT_BYTES, 1024 * 1024);
    }

    #[test]
    fn read_vec_rejects_counts_above_the_host_limit_before_fill() {
        let fill_calls = Cell::new(0);
        let result = read_vec::<u32>(
            || NC_COM_RUNTIME_MAX_ENUMERATION_ITEMS + 1,
            |_, _, _| {
                fill_calls.set(fill_calls.get() + 1);
                NC_OK
            },
        );

        assert_eq!(result.unwrap_err(), NC_E_OUT_OF_RANGE);
        assert_eq!(fill_calls.get(), 0);
    }

    #[test]
    fn read_vec_rejects_insufficient_buffer_without_capacity_progress() {
        let fill_calls = Cell::new(0);
        let result = read_vec::<u32>(
            || 2,
            |_, capacity, out_len| {
                fill_calls.set(fill_calls.get() + 1);
                // SAFETY: `read_vec` supplies one writable length slot.
                unsafe { *out_len = capacity };
                NC_E_INSUFFICIENT_BUFFER
            },
        );

        assert_eq!(result.unwrap_err(), NC_E_INSUFFICIENT_BUFFER);
        assert_eq!(fill_calls.get(), 1);
    }

    #[test]
    fn read_vec_retries_legitimate_registry_growth() {
        let fill_calls = Cell::new(0);
        let result = read_vec::<u32>(
            || 1,
            |out, capacity, out_len| {
                let call = fill_calls.get();
                fill_calls.set(call + 1);
                if call == 0 {
                    // SAFETY: `read_vec` supplies one writable length slot.
                    unsafe { *out_len = 2 };
                    return NC_E_INSUFFICIENT_BUFFER;
                }
                assert_eq!(capacity, 2);
                // SAFETY: The retried vector has capacity for these two values,
                // and `out_len` is a writable length slot.
                unsafe {
                    out.write(10);
                    out.add(1).write(20);
                    *out_len = 2;
                }
                NC_OK
            },
        );

        assert_eq!(result.unwrap(), vec![10, 20]);
        assert_eq!(fill_calls.get(), 2);
    }

    #[test]
    fn read_vec_stops_chasing_a_continuously_growing_registry() {
        let fill_calls = Cell::new(0);
        let result = read_vec::<u32>(
            || 1,
            |_, capacity, out_len| {
                fill_calls.set(fill_calls.get() + 1);
                // SAFETY: `read_vec` supplies one writable length slot.
                unsafe { *out_len = capacity + 1 };
                NC_E_INSUFFICIENT_BUFFER
            },
        );

        assert_eq!(result.unwrap_err(), NC_E_INSUFFICIENT_BUFFER);
        assert_eq!(fill_calls.get(), NC_COM_RUNTIME_MAX_ENUMERATION_RETRIES + 1);
    }

    #[test]
    fn read_utf8_view_rejects_oversized_runtime_text_before_reading_it() {
        let view = NcUtf8View {
            data: core::ptr::NonNull::<u8>::dangling().as_ptr().cast_const(),
            len: NC_COM_RUNTIME_MAX_INSPECTION_TEXT_BYTES + 1,
        };

        assert_eq!(read_utf8_view(view).unwrap_err(), NC_E_OUT_OF_RANGE);
    }

    #[test]
    fn read_vec_propagates_fill_errors() {
        const TEST_ERROR: NcStatus = NcStatus::from_code(-42);
        let result = read_vec::<u32>(|| 0, |_, _, _| TEST_ERROR);

        assert_eq!(result.unwrap_err(), TEST_ERROR);
    }
}
