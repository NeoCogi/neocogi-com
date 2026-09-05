//! Shared validation and ownership rules for descriptor entrypoints and constructors.
//!
//! Plugin macros and generated class thunks use these helpers to publish static
//! descriptors, validate constructor argument interfaces, query typed argument
//! objects, and convert successful Rust construction into one ABI-owned
//! `IUnknown` reference while keeping failure outputs null.

use crate::{
    ComInterface, IUnknown, NC_E_INVALID_ARG, NC_E_POINTER, NC_OK, NcClassDescriptor,
    NcInterfaceId, NcPluginDescriptor, NcStatus, validate_plugin_descriptor_with_issues,
};

use crate::ptr::{ComPtr, ThreadingModel, raw};

/// Writes a static plugin descriptor through the standard descriptor entrypoint ABI.
///
/// # Safety
///
/// `out` must be null or valid for writing one descriptor pointer. `plugin`
/// must point to static metadata that remains valid while the shared library is
/// loaded.
pub unsafe fn return_plugin_descriptor(
    out: *mut *const NcPluginDescriptor,
    plugin: &'static NcPluginDescriptor,
) -> NcStatus {
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

/// Validates constructor arguments against a class descriptor's expected
/// argument interface ID and ABI hash.
///
/// # Safety
///
/// `arguments`, when non-null, must be a valid borrowed `IUnknown` pointer.
pub unsafe fn validate_constructor_arguments(
    arguments: *mut IUnknown,
    expected_args: NcInterfaceId,
) -> NcStatus {
    if expected_args.iid.is_nil() {
        if expected_args.abi_hash != crate::NcAbiHash::ZERO {
            return NC_E_INVALID_ARG;
        }
        return if arguments.is_null() {
            NC_OK
        } else {
            NC_E_INVALID_ARG
        };
    }

    if arguments.is_null() {
        return NC_E_INVALID_ARG;
    }

    let mut out = core::ptr::null_mut();
    // SAFETY: `arguments` was checked non-null and the caller guarantees it is a
    // live borrowed `IUnknown`; `out` is a local pointer slot for the returned
    // owned reference.
    let status =
        unsafe { ((*(*arguments).vtable).query_interface)(arguments, &expected_args, &mut out) };
    if status.is_err() {
        return status;
    }
    if out.is_null() {
        return NC_E_POINTER;
    }

    // SAFETY: A successful `QueryInterface` returned one owned reference in
    // `out`; validation only needed to prove support, so release it now.
    unsafe { raw::release::<IUnknown>(out.cast::<IUnknown>()) };
    NC_OK
}

/// Queries typed constructor arguments from a borrowed raw `IUnknown` pointer.
///
/// # Safety
///
/// `arguments` must be null or a valid borrowed `IUnknown` pointer. The returned
/// [`ComPtr`] owns a new reference.
pub unsafe fn query_constructor_arguments<T: ComInterface>(
    arguments: *mut IUnknown,
) -> Result<ComPtr<T>, NcStatus> {
    if arguments.is_null() {
        return Err(NC_E_POINTER);
    }

    let mut out = core::ptr::null_mut();
    // SAFETY: `arguments` was checked non-null and the caller guarantees a live
    // borrowed `IUnknown`. On success, `QueryInterface` writes one owned
    // reference to `out`.
    let status =
        unsafe { ((*(*arguments).vtable).query_interface)(arguments, &T::INTERFACE_ID, &mut out) };
    if status.is_err() {
        return Err(status);
    }
    // SAFETY: Successful `QueryInterface` transfers one owned reference for the
    // requested interface; null is still handled defensively as `NC_E_POINTER`.
    unsafe { ComPtr::<T>::__nc_from_raw_owned_checked(out.cast::<T>()) }
}

/// Implements a checked class constructor wrapper.
///
/// The wrapper validates the descriptor pointer, class ID, constructor argument
/// interface identity, declared threading mode, and output pointer before
/// calling `build`. On success it writes one owned `IUnknown` reference to
/// `out`; on failure it leaves `out` null. `build` must return a negative
/// [`NcStatus`] in `Result::Err`. A non-negative error value violates that Rust
/// contract and is normalized to [`NC_E_INVALID_ARG`].
///
/// # Safety
///
/// `class` must be null or point to a readable class descriptor. `arguments`,
/// when non-null, must be a valid borrowed `IUnknown` pointer. `out` must be
/// null or valid for writing one owned `IUnknown` pointer.
pub unsafe fn construct_class_checked<M, F>(
    class: *const NcClassDescriptor,
    arguments: *mut IUnknown,
    out: *mut *mut IUnknown,
    expected_class_id: crate::NcGuid,
    expected_args: NcInterfaceId,
    build: F,
) -> NcStatus
where
    M: ThreadingModel,
    F: FnOnce(*mut IUnknown) -> Result<ComPtr<IUnknown, M>, NcStatus>,
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
    if class.constructor_args != expected_args {
        return NC_E_INVALID_ARG;
    }
    if class.threading != M::ABI_THREADING_MODE.as_raw() {
        return NC_E_INVALID_ARG;
    }
    if class.construct.is_none() {
        return NC_E_INVALID_ARG;
    }

    // SAFETY: The caller guarantees `arguments`, when non-null, is a live
    // borrowed `IUnknown`; this helper only validates the expected interface
    // identity contract.
    let status = unsafe { validate_constructor_arguments(arguments, expected_args) };
    if status.is_err() {
        return status;
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
