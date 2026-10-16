//! Safe Rust facade over the linked runtime's versioned C tables.
//!
//! `Runtime` is stateless: the linked `libnc_com` owns all process-wide
//! data. The facade checks the mandatory runtime ABI generation before every
//! table lookup, validates each table's advertised size, and converts borrowed C
//! records into owned Rust values. Plugin records combine metadata, factory
//! contracts and dependencies. Diagnostics retain a separate callback context with their own lifecycle.

use std::fmt;
use std::path::Path;
use std::vec::Vec;

use crate::{
    ComInterface, ComPtr, IUnknown, NC_COM_RUNTIME_ABI_VERSION, NC_E_INSUFFICIENT_BUFFER, NC_E_INVALID_ARG, NC_E_NOT_FOUND,
    NC_E_OUT_OF_MEMORY, NC_E_OUT_OF_RANGE, NC_E_POINTER, NC_ABI_THREADING_THREAD_LOCAL, NcDebugApiV1, NcPluginDescriptor, NcGuid, NcRuntimeApiV1, NcStatus,
    NcUtf8View, ThreadingModel,
};
use self::ffi as runtime_sys;

/// Linkage declarations shared with the persistence client, without runtime state.
pub(crate) mod ffi;

mod debug;
mod plugin;

pub use debug::{Debug, DebugMessenger};
pub use plugin::{FactoryInfo, PluginInfo, PluginOrigin};

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

/// Handle for the process-global neocogi-com runtime.
///
/// The registry and all API tables are owned by the linked
/// `libnc_com` shared library.
#[derive(Clone, Copy, Debug, Default)]
pub struct Runtime {
    /// Prevents external struct literals while keeping the handle zero-sized.
    _private: (),
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
        // Do not cache a table before checking the mandatory generation: a
        // future runtime may expose an incompatible discovery/table contract.
        self.assert_runtime_abi();
        // SAFETY: The runtime version was checked before calling the v1 getter,
        // whose result follows the static size-prefix table contract.
        let api = unsafe { table(runtime_sys::nc_com_get_runtime_api_v1()) };
        assert_eq!(api.core_abi_version, crate::NC_COM_CORE_ABI_VERSION);
        api
    }

    /// Returns the linked debug-messenger table.
    fn debug_api(&self) -> &'static NcDebugApiV1 {
        self.assert_runtime_abi();
        // SAFETY: The runtime version was checked before calling the v1 getter,
        // whose result follows the static size-prefix table contract.
        unsafe { table(runtime_sys::nc_com_get_debug_api_v1()) }
    }

    /// Returns callback-based runtime diagnostics operations.
    pub fn debug(&self) -> Debug<'_> {
        Debug::new(self)
    }

    /// Registers one plugin descriptor without constructing its factory.
    ///
    /// Registration is atomic: no plugin record becomes
    /// visible unless its metadata, provider dependencies, and process identity
    /// mapping are valid. Factory compatibility is checked during acquisition.
    /// The runtime retains the descriptor and copies dependency GUIDs; it does
    /// not own the provider's metadata allocation. Inspection through
    /// [`Self::plugin_info`] returns a separate record with first registration
    /// provenance.
    /// Registering the same descriptor again preserves its first origin.
    ///
    /// # Errors
    ///
    /// Returns metadata/ABI validation failures, [`crate::NC_E_NOT_FOUND`] for a
    /// missing hard dependency, or [`crate::NC_E_INVALID_ARG`] when a different
    /// descriptor already claims the plugin identity. No partial record is published.
    ///
    /// # Safety
    ///
    /// `plugin`, its signature/text/dependency storage, and bootstrap code must
    /// remain immutable and loaded for the process lifetime. The outer static
    /// reference alone does not establish the lifetime of reachable raw pointers.
    /// The producer must honor the declared factory threading capability.
    pub unsafe fn register_plugin_descriptor(&self, plugin: &'static NcPluginDescriptor) -> Result<(), NcStatus> {
        // SAFETY: The static reference and its referenced metadata satisfy the raw
        // table's process-lifetime registration requirement.
        status_to_result(unsafe { (self.runtime_api().register_plugin_descriptor)(plugin) })
    }

    /// Acquires one registered plugin's exact typed root IDL factory.
    ///
    /// The runtime checks the requested factory IID, full ABI signature, and
    /// threading capability before this method constructs a typed owner.
    pub fn acquire_plugin_root<T>(&self, plugin_id: NcGuid) -> Result<ComPtr<T, <T as ComInterface>::Threading>, NcStatus>
    where
        T: ComInterface,
    {
        let mut out = crate::ComRaw::<IUnknown>::null();
        // SAFETY: The local output slot is writable and the associated IDL
        // threading marker supplies the exact requested runtime capability.
        let status = unsafe {
            (self.runtime_api().acquire_factory_plugin)(
                plugin_id,
                T::INTERFACE_ID,
                <<T as ComInterface>::Threading as ThreadingModel>::ABI_THREADING_MODE.as_raw(),
                &mut out,
            )
        };
        status_to_result(status)?;
        // SAFETY: Successful runtime acquisition transfers one owned live COM
        // reference with the requested threading capability.
        unsafe { ComPtr::<T, <T as ComInterface>::Threading>::__nc_from_raw_owned_checked(out.cast()) }
    }

    /// Acquires a registered plugin's root factory without requiring the host
    /// to specify its expected interface identity.
    ///
    /// The runtime queries the registered plugin's exact factory interface from
    /// its retained description, acquires it, and returns a canonical [`IUnknown`] handle.
    /// The owner stays on the calling thread even when the factory supports sharing.
    ///
    /// # Errors
    ///
    /// Returns [`NC_E_NOT_FOUND`] if `plugin_id` is not registered, or any runtime status
    /// returned by root factory acquisition.
    pub fn acquire_registered_plugin_root(&self, plugin_id: NcGuid) -> Result<ComPtr<IUnknown>, NcStatus> {
        let plugin = self.raw_plugin_info(plugin_id)?.ok_or(NC_E_NOT_FOUND)?;
        let root = plugin.factory.interface;
        let mut out = crate::ComRaw::<IUnknown>::null();
        // SAFETY: The local slot is writable. The runtime verifies the exact
        // root identity and its threading capability before transferring it.
        let status = unsafe { (self.runtime_api().acquire_factory_plugin)(plugin_id, root, NC_ABI_THREADING_THREAD_LOCAL, &mut out) };
        status_to_result(status)?;
        // The dynamically selected factory has a common lifecycle prefix, but
        // its table identity is not IUnknown. Query canonical IUnknown before
        // constructing that safe typed handle; erasing the raw table type alone
        // is not an interface conversion.
        // SAFETY: the runtime supplies a live factory pair. The common query
        // checks canonical identity, same-object ownership, and rejected results.
        let identity = unsafe { crate::raw::__nc_query_interface(out, &IUnknown::INTERFACE_ID) };
        // SAFETY: the original factory reference is consumed on every result path.
        unsafe { crate::raw::release(out) };
        // SAFETY: the common query returned exactly one validated IUnknown owner.
        unsafe { ComPtr::<IUnknown>::from_raw_owned(identity?) }.ok_or(crate::NC_E_POINTER)
    }

    /// Loads a trusted plugin shared library into the process-global runtime.
    ///
    /// The path must have a nonempty UTF-8 representation without interior NUL
    /// bytes. This is required plugin loading: any path conversion, loading,
    /// description, dependency, or registration failure prints/emits its
    /// diagnostic and aborts the process. Failure never returns. This policy also
    /// applies to designer startup: loading has no recoverable `try_` alternative.
    /// Plugin loading calls must not be nested: native loader initialization
    /// and the plugin description entrypoint must not directly or indirectly
    /// call this operation while another load is in progress.
    /// The runtime retains at most
    /// [`crate::NC_COM_RUNTIME_MAX_DYNAMIC_PLUGINS`] distinct dynamic plugins; attempting
    /// another load is also a fatal loading failure.
    ///
    /// Returns the registered plugin GUID. Inspect it through
    /// [`Self::plugin_info`] for identity, release version and provenance.
    /// Repeated loads of the same descriptor reuse its first registration;
    /// conflicting descriptors abort through the mandatory loading policy.
    ///
    /// # Safety
    ///
    /// `path` must identify trusted neocogi-com factory plugin code. The runtime
    /// requires the standard generated plugin-descriptor export.
    pub unsafe fn load_plugin_library(&self, path: impl AsRef<Path>) -> NcGuid {
        // Live loading is mandatory for every caller, including interactive tools.
        // Reject local path conversion before entering the same fatal C operation.
        let path = path.as_ref();
        let path = path
            .to_str()
            .unwrap_or_else(|| fatal_plugin_load(format_args!("path is not valid UTF-8: {}", path.display())));
        let mut plugin_id = NcGuid::nil();
        // SAFETY: the UTF-8 path and output slot live through this call; the
        // caller accepted the trusted-library requirement. Native load errors
        // abort in the runtime; raw argument-validation errors return a status.
        let status = unsafe { (self.runtime_api().load_plugin_library_utf8)(path.as_ptr(), path.len(), &mut plugin_id) };
        if status.is_err() {
            fatal_plugin_load(status);
        }
        plugin_id
    }

    /// Loads trusted plugin libraries sequentially.
    ///
    /// ABI v1 performs no path discovery or recursive loading. Callers must
    /// supply a dependency-first order, such as one produced by
    /// `neocogi-inspect plan`. Native libraries mapped as
    /// platform dependencies are not automatically registered as neocogi-com
    /// plugins.
    ///
    /// A later failure aborts the process rather than returning partial state.
    /// Runtime ABI v1 intentionally has no transactional batch-loading call.
    ///
    /// # Safety
    ///
    /// Every path must identify trusted neocogi-com plugin code.
    pub unsafe fn load_plugin_libraries<I, P>(&self, paths: I) -> Vec<NcGuid>
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

    /// Aborts the operation if the linked library does not implement ABI v1.
    fn assert_runtime_abi(&self) {
        // ABI mismatch is a linkage/configuration error, not a recoverable
        // plugin status. Fail before dereferencing any version-specific table.
        let found = self.runtime_abi_version();
        assert_eq!(
            found, NC_COM_RUNTIME_ABI_VERSION,
            "incompatible neocogi-com runtime ABI: expected {}, found {found}",
            NC_COM_RUNTIME_ABI_VERSION
        );
    }
}

/// Reports an unrecoverable plugin-loading failure and aborts the process.
fn fatal_plugin_load(detail: impl fmt::Display) -> ! {
    eprintln!("fatal neocogi-com plugin load failure: {detail}");
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
    assert!(!table.is_null(), "compatible neocogi-com runtime returned a null API table");

    // Read only the common size prefix through the raw pointer. Creating `&T`
    // before this check would already require the backing allocation to contain
    // all of `T`, even when the table reports insufficient storage.
    // SAFETY: The caller guarantees a non-null, aligned pointer readable for the
    // leading `usize`; null was rejected above.
    let found = unsafe { table.cast::<usize>().read() };
    let required = core::mem::size_of::<T>();
    // A shorter table cannot contain every function pointer this facade calls.
    assert!(
        found >= required,
        "compatible neocogi-com runtime returned a short API table: required {required} bytes, found {found}"
    );

    // SAFETY: The size check established that the immutable static allocation
    // contains the complete known `T` prefix promised by the caller.
    unsafe { &*table }
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
fn read_vec<T: Copy>(count: impl Fn() -> usize, fill: impl Fn(*mut T, usize, *mut usize) -> NcStatus) -> Result<Vec<T>, NcStatus> {
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
    values.try_reserve_exact(capacity).map_err(|_| NC_E_OUT_OF_MEMORY)?;
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
        NC_COM_RUNTIME_MAX_ENUMERATION_ITEMS, NC_COM_RUNTIME_MAX_ENUMERATION_RETRIES, NC_COM_RUNTIME_MAX_INSPECTION_TEXT_BYTES, read_utf8_view, read_vec, table,
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
        let table = unsafe { table(core::ptr::from_ref(&EXTENDED_TEST_API_TABLE).cast::<TestApiTable>()) };

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
            unsafe { table::<TestApiTable>(core::ptr::from_ref(&SHORT_TEST_API_TABLE).cast::<TestApiTable>()) }
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
