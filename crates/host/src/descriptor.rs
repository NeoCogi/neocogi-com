//! Validated immutable descriptor ownership at the process-runtime boundary.
use neocogi_com_core::{NcAbiThreadingMode, NcGuid, NcPluginDependencyDescriptor, NcPluginDescriptor, NcStaticUtf8, NcStatus, validate_plugin_descriptor};

/// A descriptor whose complete immutable graph and code remain process-lived.
/// Construction validates the raw schema once. Every inspection/catalog projection borrows
/// this same source; no independent ID or dependency allocation can diverge.
pub(crate) struct ValidatedPluginDescriptor {
    /// Static storage or a native mapping pinned by the platform loader itself.
    description: &'static NcPluginDescriptor,
}

impl ValidatedPluginDescriptor {
    /// Validates one process-lived raw descriptor before inspection or catalog publication.
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

    /// Borrows the admitted qualified name without repeating UTF-8 or naming validation.
    pub(crate) fn qualified_name(&self) -> &'static str {
        // SAFETY: construction admitted this immutable, process-lived text view.
        unsafe { validated_text(self.description.qualified_name) }
    }

    /// Borrows the admitted version without repeating UTF-8 or version validation.
    pub(crate) fn version(&self) -> &'static str {
        // SAFETY: construction admitted this immutable, process-lived text view.
        unsafe { validated_text(self.description.plugin_version) }
    }

    /// Borrows the required complete factory signature from its original storage.
    pub(crate) fn factory_signature(&self) -> &'static [u8; 32] {
        // SAFETY: construction proved presence; the complete digest is immutable and process-lived.
        unsafe { self.description.root_factory.abi_signature.as_digest() }.expect("validated plugin descriptor has a factory signature")
    }

    /// Projects the admitted integer into its typed threading mode.
    /// The assertion records an internal invariant, not another metadata rejection path.
    pub(crate) fn factory_threading(&self) -> NcAbiThreadingMode {
        NcAbiThreadingMode::from_raw(self.description.root_factory_threading).expect("validated plugin descriptor has a factory threading mode")
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

/// Borrows already-admitted text; arbitrary raw descriptor fields must use validation first.
///
/// # Safety
/// The view's extent/readability, UTF-8 and immutable process lifetime must have
/// been established by descriptor admission. This helper performs no validation.
unsafe fn validated_text(value: NcStaticUtf8) -> &'static str {
    if value.len == 0 {
        return "";
    }
    // SAFETY: admission proved the nonempty region and representable length.
    let bytes = unsafe { core::slice::from_raw_parts(value.data, value.len as usize) };
    // SAFETY: descriptor validation checked UTF-8 before this immutable view was published.
    unsafe { core::str::from_utf8_unchecked(bytes) }
}
