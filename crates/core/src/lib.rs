//! Core ABI types for `neocogi-com`.
//!
//! This crate contains the small C-shaped binary contract shared by hosts and
//! plugins: status codes, GUIDs, `IUnknown`, descriptor metadata, versioned
//! runtime table layouts, common read-only bytes/string raw interfaces, and ABI version
//! constants. Higher-level Rust ergonomics and allocation-backed objects live in `neocogi-com`;
//! process-global dynamic loading and registry state live in `neocogi-com-runtime`.
//!
//! Descriptor metadata supports runtime validation and inspection. It exposes
//! explicit diagnostic method records and nominal aggregate identities, but not
//! application-memory layout tables or a reflection graph for
//! reconstructing Rust source declarations or generating complete source-level
//! headers from a plugin binary. ABI signatures are fixed views over hashes
//! already computed by the IDL build tool; this crate does not hash contracts.
//!
//! [`ComDeserialize`] constructs persistent values through the concrete bounded
//! NCB [`binary::Reader`]. Generated fields follow declaration order, independently
//! of checked live-call wire layouts. Scalar and fixed-size codecs work without
//! allocation; owned values and growable output require the `alloc` feature.
//!
//! [`NC_COM_CORE_ABI_VERSION`] selects the experimental pre-alpha core draft. This version
//! identifies its current binary generation, not a ratified compatibility
//! baseline. Cross-release compatibility begins only at an explicitly named
//! future ratification milestone.

#![no_std]
#![deny(missing_docs)]

mod abi;
mod abi_hash;
#[doc(hidden)]
pub use abi_hash::__nc_abi_digest;
pub mod binary;
mod buffer;
mod client;
mod method_names;
pub use method_names::{NC_COM_HANDLE_METHOD_NAMES, is_reserved_com_method};
mod generated_names;
pub use generated_names::{NcComDenseEnumMember, NcComInterfaceCompanion};
mod data;
mod decode;
mod encode;
mod guid;
mod plugin_descriptor;
mod value;
mod value_cost;
pub use value_cost::ValueCost;
mod boxed;
mod metadata;
mod object;
pub mod persistence;
mod ptr;
pub use boxed::{ComBox, WeakComBox};
/// Common allocation header and non-generic strong/weak lifetime operations.
pub mod lifetime;
#[path = "raw.rs"]
mod raw_handle;
mod ref_count;
mod weak;
pub use weak::{WeakComPtr, NcWeakRaw};
mod output;
mod value_buffer;
pub use output::{NcOutput, NcOutputWriter, PreparedOutput};
#[cfg(feature = "alloc")]
pub use output::NcOutputBuffer;
#[doc(hidden)]
pub use ref_count::{__com_add_ref_count, __com_release_ref_count};
pub mod runtime_abi;
mod signature;
mod status;

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(any(test, feature = "std"))]
extern crate std;

pub use abi::{__nc_abi_plain_type, __nc_abi_type, AbiPlainValue, AbiType};
pub use value::{
    encode_inline, decode_inline, check_inline_output, check_buffer_output, ComValue, ComValueType, ComInlineValue, NcValueRef, ValueBudget, ValueArray,
    ValueOption, ValueSlice, MAX_VALUE_BYTES, MAX_VALUE_COUNT, MAX_VALUE_DEPTH,
};
#[cfg(feature = "alloc")]
pub use value::{ValueSequenceStorage, encode_value_buffer, decode_value_buffer};
pub use buffer::{NcCopyFromBufferOffset};
pub use client::{ComClientInterface, NcClientPointer};
pub use data::{IID_INC_BYTES, IID_INC_STRING, INcBytesAbi, INcBytesVTable, INcStringAbi, INcStringVTable, QUALIFIED_NAME_INC_BYTES, QUALIFIED_NAME_INC_STRING};
pub use encode::ComSerialize;
pub use decode::ComDeserialize;
pub use guid::{NC_COM_GUID_NAMESPACE, NC_GUID_NIL, NcGuid, ParseGuidError, guid_from_name, guid_from_qualified_name};
pub use signature::NcAbiSignature;
#[cfg(feature = "alloc")]
pub use metadata::{format_abi_signature, format_method_signature};
pub use metadata::{
    NC_ABI_PASSING_COM_PTR_OUT, NC_ABI_PASSING_COM_BOX_IN, NC_ABI_PASSING_COM_BOX_OUT, NC_ABI_PASSING_WEAK_COM_BOX_OUT, NC_ABI_PASSING_WEAK_COM_PTR_OUT,
    NC_ABI_PASSING_COM_REF, NC_ABI_PASSING_COPY_FROM_BUFFER_OFFSET, NC_ABI_PASSING_COPY_TO_BUFFER_OFFSET, NC_ABI_PASSING_OUTPUT_BUFFER, NC_ABI_PASSING_OUT,
    NC_ABI_PASSING_BORROWED_VALUE, NC_ABI_PASSING_OPTIONAL_COM_REF, NC_ABI_PASSING_VALUE, NC_ABI_PASSING_VOID, NC_ABI_RECEIVER_MUTABLE, NC_ABI_RECEIVER_SHARED,
    NC_ABI_THREADING_THREAD_LOCAL, NC_ABI_THREADING_THREAD_SAFE, NC_ABI_TYPE_AGGREGATE, NC_ABI_TYPE_BOOL, NC_ABI_TYPE_F32, NC_ABI_TYPE_F64, NC_ABI_TYPE_I8,
    NC_ABI_TYPE_I16, NC_ABI_TYPE_I32, NC_ABI_TYPE_I64, NC_ABI_TYPE_INTERFACE, NC_ABI_TYPE_ISIZE, NC_ABI_TYPE_NC_GUID, NC_ABI_TYPE_NC_STATUS, NC_ABI_TYPE_U8,
    NC_ABI_TYPE_U16, NC_ABI_TYPE_U32, NC_ABI_TYPE_U64, NC_ABI_TYPE_USIZE, NC_ABI_TYPE_VOID, NC_COM_CORE_ABI_VERSION, NC_DESCRIPTOR_QUALIFIED_NAME_MAX_LEN,
    NC_DESCRIPTOR_VERSION_MAX_LEN, NC_DESCRIPTOR_RUST_PATH_MAX_LEN, NcRustPathError, validate_rust_path, NcAbiPassing, NcAbiPassingDescriptor, NcAbiReceiver,
    NcAbiThreadingMode, NcAbiType, NcAbiTypeDescriptor, NcInterfaceDescriptor, NcInterfaceId, NcMethodDescriptor, NcMethodParamDescriptor,
    NcMethodReturnDescriptor, NcPluginDependencyDescriptor, NcQualifiedNameError, NcStaticUtf8, NcVersionTextError, validate_qualified_name,
    validate_version_text,
};
pub use object::{
    __nc_find_interface, __ComSupportsInterfaceThreading, ComInterface, IID_IUNKNOWN, IUNKNOWN_INTERFACE_ID, IUNKNOWN_QUALIFIED_NAME, IUnknown, IUnknownVTable,
};
pub use raw_handle::ComRaw;
pub use ptr::{ComPtr, ComRef, ThreadLocal, ThreadSafe, ThreadingModel, raw};
pub use runtime_abi::{
    NcPersistenceApiV1, NcPersistenceContext, NcPersistenceLimitsV1, NcPersistenceWriteBytesV1, NC_COM_RUNTIME_ABI_VERSION,
    NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS, NC_COM_RUNTIME_MAX_DYNAMIC_PLUGINS, NC_DEBUG_SEVERITY_ALL, NC_DEBUG_SEVERITY_ERROR, NC_DEBUG_SEVERITY_INFO,
    NC_DEBUG_SEVERITY_WARNING, NC_DEBUG_TYPE_ALL, NC_DEBUG_TYPE_PLUGIN, NC_DEBUG_TYPE_RUNTIME, NC_DEBUG_TYPE_VALIDATION, NcDebugApiV1, NcDebugCallbackV1,
    NcDebugMessageV1, NcDebugMessengerCreateInfoV1, NcInspectPluginDescriptorCallbackV1, NcFactoryInfoV1, NcPluginInfoV1, NcRuntimeApiV1, NcUtf8View,
};
pub use status::{
    NC_E_ABI_VERSION, NC_E_BUSY, NC_E_ALREADY_BOUND, NC_E_CLOSED, NC_E_DROPPED, NC_E_INSUFFICIENT_BUFFER, NC_E_INVALID_ARG, NC_E_INVALID_TAG, NC_E_LOAD_FAILED,
    NC_E_NOINTERFACE, NC_E_NOT_FOUND, NC_E_OUT_OF_MEMORY, NC_E_OUT_OF_RANGE, NC_E_POINTER, NC_OK, NcStatus,
};

#[doc(hidden)]
pub use metadata::{__ComProjection, __nc_interface_chain, __nc_object_interface_count, __nc_projection_table, __nc_projection_vtables};
pub use plugin_descriptor::{
    NC_PLUGIN_DESCRIPTOR_ABI_VERSION, NC_GET_PLUGIN_DESCRIPTOR_SYMBOL, NcAcquirePluginFactory, NcPluginDescriptor, NcGetPluginDescriptor, PluginRootAcquire,
    write_plugin_root, return_plugin_descriptor, validate_plugin_descriptor,
};

pub use value_buffer::{EncodedInput, DecodedInput};
