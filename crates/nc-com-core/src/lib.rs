//! Core ABI types for `nc-com`.
//!
//! This crate contains the small C-shaped binary contract shared by hosts and
//! plugins: status codes, GUIDs, `IUnknown`, descriptor metadata, versioned
//! runtime table layouts, common read-only bytes/string raw interfaces, and ABI version
//! constants. It also owns plugin descriptor validation through
//! [`validate_plugin_descriptor_with_issues`], which reports structured
//! descriptor issues and remains available without allocation support. Builds
//! with `alloc` use ordered sets to accelerate duplicate detection. Higher-level
//! Rust ergonomics and allocation-backed objects live in `nc-com`;
//! process-global dynamic loading and registry state live in `nc-com-runtime`.
//!
//! Descriptor metadata supports runtime validation and inspection. Aggregate
//! hashes are deliberately opaque compatibility fingerprints, not a reflection
//! graph for reconstructing source declarations or generating headers from a
//! plugin binary.
//!
//! core ABI v3 is currently an experimental pre-alpha draft. The core ABI version
//! identifies its current binary generation, not a ratified compatibility
//! baseline. Cross-release compatibility begins only at an explicitly named
//! future ratification milestone.

#![no_std]
#![deny(missing_docs)]

mod abi;
mod constructor;
mod client;
mod data;
mod guid;
mod hash;
mod metadata;
mod object;
mod ptr;
pub mod runtime_abi;
mod status;

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(any(test, feature = "std"))]
extern crate std;

pub use abi::{
    __NcTaggedEnumAbiStorage, __nc_abi_com_struct_type, __nc_abi_plain_type, __nc_abi_type,
    AbiPlainValue, AbiType, ComStruct,
};
pub use client::{ComClientInterface, NcClientPointer};
pub use constructor::{
    construct_class_checked, query_constructor_arguments, return_plugin_descriptor,
    validate_constructor_arguments,
};
pub use data::{
    IID_INC_BYTES, IID_INC_STRING, INcBytesAbi, INcBytesVTable, INcStringAbi, INcStringVTable,
    QUALIFIED_NAME_INC_BYTES, QUALIFIED_NAME_INC_STRING,
};
pub use guid::{
    NC_COM_GUID_NAMESPACE, NC_GUID_NIL, NcGuid, ParseGuidError, guid_from_qualified_name,
};
pub use hash::NcAbiHash;
#[cfg(feature = "alloc")]
pub use metadata::format_method_signature;
pub use metadata::{
    NC_ABI_PASSING_COM_PTR_OUT, NC_ABI_PASSING_COM_REF, NC_ABI_PASSING_COPY_FROM_BUFFER_OFFSET,
    NC_ABI_PASSING_COPY_TO_BUFFER_OFFSET, NC_ABI_PASSING_OUT, NC_ABI_PASSING_VALUE,
    NC_ABI_PASSING_VOID, NC_ABI_THREADING_THREAD_LOCAL, NC_ABI_THREADING_THREAD_SAFE,
    NC_ABI_TYPE_AGGREGATE, NC_ABI_TYPE_BOOL, NC_ABI_TYPE_F32, NC_ABI_TYPE_F64, NC_ABI_TYPE_I8,
    NC_ABI_TYPE_I16, NC_ABI_TYPE_I32, NC_ABI_TYPE_I64, NC_ABI_TYPE_INTERFACE, NC_ABI_TYPE_ISIZE,
    NC_ABI_TYPE_NC_ABI_HASH, NC_ABI_TYPE_NC_GUID, NC_ABI_TYPE_NC_STATUS, NC_ABI_TYPE_U8,
    NC_ABI_TYPE_U16, NC_ABI_TYPE_U32, NC_ABI_TYPE_U64, NC_ABI_TYPE_USIZE, NC_ABI_TYPE_VOID,
    NC_COM_CORE_ABI_VERSION, NC_DESCRIPTOR_QUALIFIED_NAME_MAX_LEN, NC_DESCRIPTOR_VERSION_MAX_LEN,
    NC_GET_PLUGIN_DESCRIPTOR_SYMBOL, NcAbiAggregateKind, NcAbiFieldLayout, NcAbiHashPart,
    NcAbiPassing, NcAbiPassingDescriptor, NcAbiStructLayout, NcAbiThreadingMode, NcAbiType,
    NcAbiTypeDescriptor, NcClassDescriptor, NcClassInterfaceDescriptor, NcClassTable,
    NcConstructorFn, NcGetPluginDescriptor, NcInterfaceDescriptor, NcInterfaceId,
    NcMethodDescriptor, NcMethodParamDescriptor, NcMethodReturnDescriptor,
    NcPluginDependencyDescriptor, NcPluginDescriptor, NcQualifiedNameError, NcStaticUtf8,
    NcVersionTextError, validate_qualified_name, validate_version_text,
};
pub use metadata::{
    NcDescriptorPath, NcDescriptorValidationIssue, NcDescriptorValidationKind,
    validate_plugin_descriptor_with_issues,
};
pub use object::{
    ComInterface, IID_IUNKNOWN, IUNKNOWN_INTERFACE_ID, IUNKNOWN_QUALIFIED_NAME, IUnknown,
    IUnknownVTable,
};
pub use ptr::{ComPtr, ComRef, ThreadLocal, ThreadSafe, ThreadingModel, raw};
pub use runtime_abi::{
    NC_COM_RUNTIME_ABI_VERSION, NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS,
    NC_COM_RUNTIME_MAX_LOADED_MODULES, NC_DEBUG_SEVERITY_ALL, NC_DEBUG_SEVERITY_ERROR,
    NC_DEBUG_SEVERITY_INFO, NC_DEBUG_SEVERITY_WARNING, NC_DEBUG_TYPE_ALL, NC_DEBUG_TYPE_PLUGIN,
    NC_DEBUG_TYPE_RUNTIME, NC_DEBUG_TYPE_VALIDATION, NcClassInfoV1, NcDebugApiV1,
    NcDebugCallbackV1, NcDebugMessageV1, NcDebugMessengerCreateInfoV1,
    NcInspectPluginDescriptorCallbackV1, NcInspectionApiV1, NcModuleInfoV1, NcPluginInfoV1,
    NcPluginRegistryV1, NcRuntimeApiV1, NcTypeRegistryV1, NcUtf8View,
};
pub use status::{
    NC_E_ABI_VERSION, NC_E_INSUFFICIENT_BUFFER, NC_E_INVALID_ARG, NC_E_INVALID_TAG,
    NC_E_LOAD_FAILED, NC_E_NOINTERFACE, NC_E_NOT_FOUND, NC_E_OUT_OF_MEMORY, NC_E_OUT_OF_RANGE,
    NC_E_POINTER, NC_OK, NcStatus,
};

#[doc(hidden)]
pub use metadata::{__nc_interface_chain, __nc_class_interface_count, __nc_class_interfaces};
