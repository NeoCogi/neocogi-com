//! Validated immutable descriptor ownership at the process-runtime boundary.
use neocogi_com_core::{NcGuid, NcPluginDependencyDescriptor, NcPluginDescriptor, NcStatus, validate_plugin_descriptor};

/// A descriptor whose complete immutable graph and code remain process-lived.
/// Construction validates the raw schema once. Every catalog projection borrows
/// this same source; no independent ID or dependency allocation can diverge.
pub(crate) struct ValidatedPluginDescriptor {
    /// Static storage or a native mapping pinned by the platform loader itself.
    description: &'static NcPluginDescriptor,
}

impl ValidatedPluginDescriptor {
    /// Validates one process-lived raw descriptor before it enters the catalog.
    ///
    /// # Safety
    /// A nonnull pointer and every referenced buffer/function must be readable,
    /// immutable and valid for the process lifetime. Native opening must already
    /// pin its mapping, including on validation or later publication failure.
    pub(crate) unsafe fn new(description: *const NcPluginDescriptor) -> Result<Self, NcStatus> {
        // SAFETY: the caller supplies raw readability and reachable lifetimes.
        let status = unsafe { validate_plugin_descriptor(description) };
        if status.is_err() {
            return Err(status);
        }
        // SAFETY: validation rejected null; the caller promises process lifetime.
        Ok(Self { description: unsafe { &*description } })
    }

    /// Borrows the one validated description, including its executable entry.
    pub(crate) fn get(&self) -> &'static NcPluginDescriptor {
        self.description
    }

    /// Derives registry identity directly from the normative description.
    pub(crate) fn id(&self) -> NcGuid {
        self.description.plugin_id
    }

    /// Exact pointer identity distinguishes repeat publication from a conflict.
    pub(crate) fn as_ptr(&self) -> *const NcPluginDescriptor {
        self.description
    }

    /// Borrows declared dependencies without copying or representation casts.
    pub(crate) fn dependencies(&self) -> &'static [NcPluginDependencyDescriptor] {
        if self.description.dependency_count == 0 {
            return &[];
        }
        // SAFETY: validation proved the aligned pointer/count table's shape;
        // construction retains its immutable process lifetime. Empty may be null.
        unsafe { core::slice::from_raw_parts(self.description.dependencies, self.description.dependency_count as usize) }
    }
}
