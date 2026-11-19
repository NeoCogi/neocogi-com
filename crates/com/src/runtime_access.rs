//! Allocation-free access admitted from one host's immutable C service graph.
//!
//! The handle is available without std or the optional runtime API methods so
//! a provider can receive/retain supplied access without discovering or owning an
//! implementation. Rich runtime and persistence API wrappers remain capability-gated.

use core::fmt;

use crate::{NC_COM_RUNTIME_ABI_VERSION, NcDebugApiV3, NcHostApiV3, NcPersistenceApiV3, NcPluginApiV3, NcStatus, validate_host_api};

/// Copyable access to the selected host's process-lived runtime services.
///
/// Copying this handle copies three immutable table references. It initializes,
/// allocates and owns no registry. Plugin acquisition receives it from the host;
/// reusable libraries accept it from their caller. There is no Default/global
/// discovery or safe constructor from arbitrary foreign storage. Access may cross
/// threads: admitted host callbacks support concurrent calls. Caller-owned
/// persistence contexts retain their separate thread-local/exclusive contract.
#[derive(Clone, Copy)]
pub struct RuntimeHandle {
    /// Admitted plugin/catalog operations whose callbacks execute in the host.
    pub(crate) runtime: &'static NcPluginApiV3,
    /// Diagnostics operations from the same immutable host service graph.
    pub(crate) debug: &'static NcDebugApiV3,
    /// Persistence operations retained by every context created through access.
    pub(crate) persistence: &'static NcPersistenceApiV3,
}

impl RuntimeHandle {
    /// Admits the raw host description for host implementation/bootstrap code.
    ///
    /// Prefix checks reject null/short/incompatible tables before complete typed
    /// borrows. Admission invokes no callbacks and retains no outer-record
    /// identity; the handle stores only references to the three service tables.
    ///
    /// # Safety
    ///
    /// The pointer satisfies `validate_host_api`'s readable-prefix contract. On
    /// supported layouts the complete reachable tables and callable code remain
    /// immutable and valid for process/Wasm-instance lifetime and describe the
    /// same host owner. Host entrypoints support concurrent access through handle
    /// copies; opaque caller-owned contexts retain their documented thread and
    /// exclusive-borrow requirements. Checks cannot prove these obligations;
    /// a static outer record alone does not establish raw child lifetimes.
    #[doc(hidden)]
    pub unsafe fn __from_host_api(host: *const NcHostApiV3) -> Result<Self, NcStatus> {
        // SAFETY: The caller supplies the validator's prefix/backing obligations.
        let status = unsafe { validate_host_api(host) };
        if status.is_err() {
            return Err(status);
        }
        // SAFETY: Admission established all current sizes/versions. The caller
        // separately guarantees valid immutable process-lived child tables/code.
        let host = unsafe { &*host };
        Ok(Self {
            // SAFETY: Complete child tables and their lifetimes are promised above.
            runtime: unsafe { &*host.runtime },
            // SAFETY: Same immutable process-lived host graph.
            debug: unsafe { &*host.debug },
            // SAFETY: Same immutable process-lived host graph.
            persistence: unsafe { &*host.persistence },
        })
    }

    /// Returns the admitted runtime family selected by [`NC_COM_RUNTIME_ABI_VERSION`].
    pub const fn runtime_abi_version(&self) -> u32 {
        NC_COM_RUNTIME_ABI_VERSION
    }

    /// Returns the core object/descriptor ABI admitted for these operations.
    pub const fn core_abi_version(&self) -> u32 {
        self.runtime.core_abi_version
    }
}

impl fmt::Debug for RuntimeHandle {
    /// Describes admitted layouts without executing host callbacks or discovery.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeHandle")
            .field("runtime_abi_version", &self.runtime_abi_version())
            .field("core_abi_version", &self.core_abi_version())
            .field("runtime_table_bytes", &self.runtime.size)
            .field("debug_table_bytes", &self.debug.size)
            .field("persistence_table_bytes", &self.persistence.size)
            .finish()
    }
}
