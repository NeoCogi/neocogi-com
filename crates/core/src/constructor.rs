//! Shared validation and ownership rules for descriptor entrypoints and constructors.
//!
//! Plugin macros and generated class thunks use these helpers to publish static
//! descriptors, validate typed constructor frames, and convert successful Rust
//! construction into one ABI-owned `IUnknown` reference while keeping failure
//! outputs null.

use core::ffi::c_void;

use crate::{
    IUnknown, NC_E_INVALID_ARG, NC_E_POINTER, NC_OK, NcAbiSignature, NcClassDescriptor, NcPluginDescriptor, NcStatus, validate_plugin_descriptor_with_issues,
};

use crate::ptr::{ComPtr, ThreadingModel};

/// Writes a static plugin descriptor through the standard descriptor entrypoint ABI.
///
/// # Safety
///
/// `out` must be null or valid for writing one descriptor pointer. `plugin`
/// must point to static metadata that remains valid while the shared library is
/// loaded.
pub unsafe fn return_plugin_descriptor(out: *mut *const NcPluginDescriptor, plugin: &'static NcPluginDescriptor) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }

    // SAFETY: `out` was checked non-null and is valid for one pointer write by
    // this function's safety contract.
    unsafe { *out = core::ptr::null() };

    // SAFETY: `plugin` is a static descriptor supplied by the caller; validation
    // only reads descriptor metadata.
    let status = unsafe { validate_plugin_descriptor_with_issues(plugin, |_| {}) };
    if status.is_err() {
        return status;
    }

    // SAFETY: `out` remains valid for the duration of this function and the
    // descriptor has just been validated.
    unsafe { *out = plugin };
    NC_OK
}

/// Implements a checked class constructor wrapper.
///
/// The wrapper validates the descriptor pointer, class ID, constructor-frame
/// signature, declared threading mode, and output pointer before
/// calling `build`. On success it writes one owned `IUnknown` reference to
/// `out`; on failure it leaves `out` null. `build` must return a negative
/// [`NcStatus`] in `Result::Err`. A non-negative error value violates that Rust
/// contract and is normalized to [`NC_E_INVALID_ARG`].
///
/// # Safety
///
/// `class` must be null or point to a readable class descriptor. A non-null
/// `arguments` pointer must identify a frame matching `expected_arguments` and
/// remain readable for `build`. `out` must be null or valid for writing one
/// owned `IUnknown` pointer.
pub unsafe fn construct_class_checked<M, F>(
    class: *const NcClassDescriptor,
    arguments: *const c_void,
    out: *mut *mut IUnknown,
    expected_class_id: crate::NcGuid,
    expected_arguments: NcAbiSignature,
    build: F,
) -> NcStatus
where
    M: ThreadingModel,
    F: FnOnce(*const c_void) -> Result<ComPtr<IUnknown, M>, NcStatus>,
{
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: `out` was checked non-null and the caller guarantees it is valid
    // for one `IUnknown*` write. Clear it before any fallible validation.
    unsafe { *out = core::ptr::null_mut() };

    if class.is_null() {
        return NC_E_POINTER;
    }

    // SAFETY: The caller guarantees that non-null `class` points to readable
    // descriptor metadata.
    let class = unsafe { &*class };
    if class.class_id != expected_class_id {
        return NC_E_INVALID_ARG;
    }
    if class.constructor_abi_signature != expected_arguments {
        return NC_E_INVALID_ARG;
    }
    if class.threading != M::ABI_THREADING_MODE.as_raw() {
        return NC_E_INVALID_ARG;
    }
    if class.construct.is_none() {
        return NC_E_INVALID_ARG;
    }

    // An empty signature denotes the one valid empty frame representation: null.
    // Every typed frame has a non-empty signature and requires a non-null pointer.
    if arguments.is_null() != (expected_arguments == NcAbiSignature::EMPTY) {
        return NC_E_INVALID_ARG;
    }

    match build(arguments) {
        Ok(unknown) => {
            // SAFETY: `out` is valid for one pointer write and `into_raw`
            // transfers the owned reference to the ABI caller.
            unsafe { *out = unknown.into_raw() };
            NC_OK
        }
        Err(status) => status.normalize_error(),
    }
}
