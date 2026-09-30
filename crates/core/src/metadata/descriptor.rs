//! Physical C-compatible interface metadata records.
//!
//! The structures in this module form the immutable pointer graph returned by
//! generated interface descriptions: methods, parameters, return contracts,
//! and static text. Accessor methods
//! validate null/count shapes before turning raw arrays into borrowed Rust views.
//!
//! Descriptor graphs are immutable, library-owned metadata. Relationships use
//! pointer/count pairs so hosts can inspect them without sharing Rust
//! allocation ownership. Accessors validate local shape such as null/count
//! consistency, but callers remain responsible for keeping the exporting
//! module loaded for every returned borrow.

use super::contract::{NcAbiPassingDescriptor, NcAbiTypeDescriptor, NcInterfaceId};
use crate::guid::NcGuid;
use crate::signature::NcAbiSignature;
use crate::status::{NC_E_INVALID_ARG, NC_E_POINTER, NcStatus};

/// C-stable typed ABI descriptor for one method parameter.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcMethodParamDescriptor {
    /// Static parameter name metadata for diagnostics.
    pub name: NcStaticUtf8,
    /// ABI passing mode for this parameter.
    pub passing: NcAbiPassingDescriptor,
    /// ABI type for this parameter.
    pub ty: NcAbiTypeDescriptor,
}

// SAFETY: Parameter descriptors are immutable static metadata.
unsafe impl Sync for NcMethodParamDescriptor {}

/// C-stable typed ABI descriptor for one method success return value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcMethodReturnDescriptor {
    /// ABI passing mode for this return.
    pub passing: NcAbiPassingDescriptor,
    /// ABI type for this return.
    pub ty: NcAbiTypeDescriptor,
}

/// Static UTF-8 metadata stored in descriptor tables.
///
/// This is a raw pointer/count view, not an owning string or a Rust borrow with
/// a tracked lifetime. Copying it does not copy its bytes or retain their module.
/// [`Self::as_str`] validates UTF-8 but is unsafe because the caller must keep
/// the backing allocation readable and immutable for the chosen return lifetime.
/// Public fields also allow foreign producers to supply invalid metadata.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcStaticUtf8 {
    /// Pointer to UTF-8 bytes that remain valid while the shared library is
    /// loaded.
    pub data: *const u8,
    /// Number of bytes at `data`.
    pub len: u32,
}

// SAFETY: This descriptor is immutable ABI metadata. Producers must keep `data`
// valid while the descriptor is visible to a host.
unsafe impl Sync for NcStaticUtf8 {}

impl NcStaticUtf8 {
    /// Empty static UTF-8 metadata.
    pub const EMPTY: Self = Self { data: core::ptr::null(), len: 0 };

    /// Creates static UTF-8 metadata from a string literal or other static
    /// string.
    ///
    /// The ABI stores the byte length with `value.len() as u32`; this constructor
    /// does not reject an overlong input. Descriptor authors should use the
    /// much smaller limits enforced by [`crate::validate_qualified_name`] and
    /// [`crate::validate_version_text`] for their respective metadata fields.
    pub const fn from_static(value: &'static str) -> Self {
        Self {
            data: value.as_ptr(),
            len: value.len() as u32,
        }
    }

    /// Returns the byte length.
    pub const fn len(self) -> u32 {
        self.len
    }

    /// Returns whether the metadata string is empty.
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Borrows the metadata bytes.
    ///
    /// This performs no allocation or UTF-8 validation. Zero length returns an
    /// empty slice; a nonzero length with a null pointer returns `NC_E_POINTER`.
    /// No check can establish the validity of a non-null foreign pointer.
    ///
    /// # Safety
    ///
    /// If `self.len != 0`, `self.data` must point to `self.len` bytes that
    /// remain readable and immutable for the returned lifetime and satisfy
    /// [`core::slice::from_raw_parts`]'s allocation and size requirements.
    /// The caller selects `'a`; it is not constrained by this copied descriptor.
    pub unsafe fn as_bytes<'a>(self) -> Result<&'a [u8], NcStatus> {
        if self.len == 0 {
            return Ok(&[]);
        }
        if self.data.is_null() {
            return Err(NC_E_POINTER);
        }
        // SAFETY: The caller guarantees that non-empty metadata points to
        // `len` bytes valid for the returned lifetime; null was rejected above.
        Ok(unsafe { core::slice::from_raw_parts(self.data, self.len as usize) })
    }

    /// Borrows the metadata as UTF-8 text.
    ///
    /// No string is allocated. Invalid UTF-8 returns `NC_E_INVALID_ARG`; pointer
    /// errors are forwarded from [`Self::as_bytes`]. The raw view does not expose
    /// a safe, infallible `as_str(&self) -> &str` method.
    ///
    /// ```
    /// use neocogi_com_core::NcStaticUtf8;
    ///
    /// let name = NcStaticUtf8::from_static("example.widget");
    /// // SAFETY: The literal's immutable bytes remain live for the whole program.
    /// let text = unsafe { name.as_str() }?;
    /// assert_eq!(text, "example.widget");
    /// # Ok::<(), neocogi_com_core::NcStatus>(())
    /// ```
    ///
    /// # Safety
    ///
    /// The same pointer validity requirements as [`NcStaticUtf8::as_bytes`]
    /// apply.
    pub unsafe fn as_str<'a>(self) -> Result<&'a str, NcStatus> {
        // SAFETY: The caller supplied the same metadata lifetime and pointer
        // guarantees required by `as_bytes`; UTF-8 is validated afterward.
        core::str::from_utf8(unsafe { self.as_bytes()? }).map_err(|_| NC_E_INVALID_ARG)
    }
}

/// Static descriptor for one ABI method.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcMethodDescriptor {
    /// Static method name metadata.
    pub name: NcStaticUtf8,
    /// One of the `NC_ABI_RECEIVER_*` values describing Rust dispatch access.
    pub receiver: u32,
    /// Number of logical method parameters.
    pub param_count: u32,
    /// Pointer to `param_count` typed parameter descriptors.
    pub params: *const NcMethodParamDescriptor,
    /// Typed success-return descriptor.
    pub returns: NcMethodReturnDescriptor,
}

// SAFETY: Method descriptors are immutable static metadata; pointer fields
// reference other immutable metadata owned by the exporting component.
unsafe impl Sync for NcMethodDescriptor {}

/// Static descriptor for one ABI interface.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcInterfaceDescriptor {
    /// Stable interface ID derived from `qualified_name`, except for `IUnknown`.
    pub interface_id: NcGuid,
    /// Normative qualified name from which `interface_id` is derived; the
    /// canonical root interface instead uses the reserved name `IUnknown`.
    pub qualified_name: NcStaticUtf8,
    /// Precomputed validation signature for this interface declaration.
    ///
    /// The typed `methods` remain available for checking and diagnostics, while
    /// compatibility compares this generator-owned identity byte-for-byte. The
    /// signature incorporates the exact transitive IDL module revision.
    pub abi_signature: NcAbiSignature,
    /// Explicit generated base contract, or `NONE` for the implicit IUnknown root.
    /// Methods below describe only the slots introduced by this interface.
    pub base: NcInterfaceId,
    /// Number of locally introduced method descriptors.
    pub method_count: u32,
    /// Pointer to `method_count` descriptors.
    pub methods: *const NcMethodDescriptor,
}

// SAFETY: Interface descriptors are immutable static metadata; `methods` is a
// read-only table that must remain live while the descriptor is exported.
unsafe impl Sync for NcInterfaceDescriptor {}

impl NcInterfaceDescriptor {
    /// Borrows the normative qualified interface name.
    ///
    /// # Safety
    ///
    /// `self.qualified_name` must reference metadata that remains valid for the
    /// returned lifetime.
    pub unsafe fn qualified_name_str<'a>(&self) -> Result<&'a str, NcStatus> {
        // SAFETY: The caller guarantees the interface descriptor and its
        // static string metadata remain valid for the returned lifetime.
        unsafe { self.qualified_name.as_str() }
    }
}

impl NcInterfaceDescriptor {
    /// Borrows the method descriptor slice.
    ///
    /// # Safety
    ///
    /// If `self.method_count != 0`, `self.methods` must point to
    /// `self.method_count` descriptors that remain valid for the returned
    /// lifetime.
    pub unsafe fn methods<'a>(&self) -> Result<&'a [NcMethodDescriptor], NcStatus> {
        if self.method_count == 0 {
            return Ok(&[]);
        }
        if self.methods.is_null() {
            return Err(NC_E_POINTER);
        }
        // SAFETY: The caller guarantees `methods` points to `method_count`
        // descriptors valid for the returned lifetime; null/count was checked.
        Ok(unsafe { core::slice::from_raw_parts(self.methods, self.method_count as usize) })
    }
}

impl NcMethodDescriptor {
    /// Borrows the parameter descriptor slice.
    ///
    /// # Safety
    ///
    /// If `self.param_count != 0`, `self.params` must point to
    /// `self.param_count` descriptors that remain valid for the returned
    /// lifetime.
    pub unsafe fn params<'a>(&self) -> Result<&'a [NcMethodParamDescriptor], NcStatus> {
        if self.param_count == 0 {
            return Ok(&[]);
        }
        if self.params.is_null() {
            return Err(NC_E_POINTER);
        }
        // SAFETY: The caller guarantees `params` points to `param_count`
        // descriptors valid for the returned lifetime; null/count was checked.
        Ok(unsafe { core::slice::from_raw_parts(self.params, self.param_count as usize) })
    }
}

/// One hard neocogi-com plugin dependency.
///
/// Dependency metadata is declarative. It names a plugin that must already be
/// registered before the descriptor containing this entry may be registered;
/// it does not cause the runtime to locate or load another library.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcPluginDependencyDescriptor {
    /// Stable identity of the required plugin.
    pub plugin_id: NcGuid,
}

#[cfg(test)]
mod tests {
    use crate::{NC_ABI_RECEIVER_MUTABLE, NC_ABI_RECEIVER_SHARED, NcAbiReceiver, NcGuid};
    use crate::{
        NC_ABI_THREADING_THREAD_LOCAL, NC_ABI_THREADING_THREAD_SAFE, NC_COM_CORE_ABI_VERSION, NcAbiSignature, NcAbiThreadingMode, NcInterfaceDescriptor,
        NcInterfaceId, NcPluginDependencyDescriptor,
    };

    #[test]
    fn core_abi_identity_is_stable() {
        assert_eq!(NC_COM_CORE_ABI_VERSION, 14);
    }

    #[test]
    fn interface_id_none_requires_nil_iid_and_empty_signature() {
        /// Valid nonempty digest used to distinguish an attributed interface.
        static NONEMPTY_SIGNATURE: [u8; 32] = [1; 32];

        assert!(NcInterfaceId::NONE.is_none());
        assert!(!NcInterfaceId::new(NcGuid::nil(), NcAbiSignature::from_static(&NONEMPTY_SIGNATURE)).is_none());
    }

    #[test]
    fn threading_mode_raw_values_are_stable_and_checked() {
        assert_eq!(NcAbiThreadingMode::ThreadLocal.as_raw(), NC_ABI_THREADING_THREAD_LOCAL);
        assert_eq!(NcAbiThreadingMode::ThreadSafe.as_raw(), NC_ABI_THREADING_THREAD_SAFE);
        assert_eq!(
            NcAbiThreadingMode::from_raw(NC_ABI_THREADING_THREAD_LOCAL),
            Some(NcAbiThreadingMode::ThreadLocal)
        );
        assert_eq!(NcAbiThreadingMode::from_raw(NC_ABI_THREADING_THREAD_SAFE), Some(NcAbiThreadingMode::ThreadSafe));
        assert_eq!(NcAbiThreadingMode::from_raw(u32::MAX), None);
    }

    #[test]
    fn receiver_modes_are_stable_and_checked() {
        assert_eq!(NcAbiReceiver::Shared.as_raw(), NC_ABI_RECEIVER_SHARED);
        assert_eq!(NcAbiReceiver::Mutable.as_raw(), NC_ABI_RECEIVER_MUTABLE);
        assert_eq!(NcAbiReceiver::from_raw(2), None);
    }

    #[test]
    fn interface_descriptor_c_layout_includes_qualified_name() {
        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(core::mem::size_of::<NcInterfaceDescriptor>(), 96);
            assert_eq!(core::mem::align_of::<NcInterfaceDescriptor>(), 8);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, interface_id), 0);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, qualified_name), 16);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, abi_signature), 32);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, method_count), 80);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, methods), 88);
        }

        #[cfg(target_pointer_width = "32")]
        {
            assert_eq!(core::mem::size_of::<NcInterfaceDescriptor>(), 64);
            assert_eq!(core::mem::align_of::<NcInterfaceDescriptor>(), 4);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, interface_id), 0);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, qualified_name), 16);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, abi_signature), 24);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, method_count), 56);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, methods), 60);
        }
    }

    #[test]
    fn plugin_dependency_c_layout_is_stable() {
        assert_eq!(core::mem::size_of::<NcPluginDependencyDescriptor>(), 16);
        assert_eq!(core::mem::align_of::<NcPluginDependencyDescriptor>(), 4);
        assert_eq!(core::mem::offset_of!(NcPluginDependencyDescriptor, plugin_id), 0);
    }
}
