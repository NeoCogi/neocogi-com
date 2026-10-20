//! Typed COM ownership, local implementation support and stateless runtime clients.
//!
//! Cross-component contracts are `.ncidl` sources compiled by `neocogi-idl`.
//! The resolver binds names and computes identities and signatures. A shared
//! typed backend emits complete Rust traits, explicit C wires, codecs, client
//! views, provider thunks and inspection metadata. No Rust declaration attribute
//! accepts another contract schema or an author-assigned signature.
//!
//! A contract source can declare:
//!
//! ```text
//! #[version(1)]
//! interface ICounter {
//!     fn value(&self) -> Result<u32>;
//!     fn replace(&mut self, value: u32) -> Result<()>;
//! }
//! ```
//!
//! Its contract crate runs `neocogi_idl::cargo::Module::directory(...).build()`
//! from `build.rs` and includes the generated module. The IDL module guide and
//! packaged `calculator` example document a complete source/build/provider flow.
//!
//! Concrete Rust state uses `com_object` with those generated interfaces.
//! The following runnable example uses the built-in immutable event contract:
//!
//! ```
//! use neocogi_com::{com_object, ComBox};
//! use neocogi_com::events::{IEvent, IEventAbi};
//!
//! #[com_object(neocogi_com::events::IEvent, ownership = exclusive)]
//! struct Activated;
//! impl IEvent for Activated {}
//!
//! let event: ComBox<IEventAbi> = Activated::into_box(Activated).query().unwrap();
//! let origin_observer = event.downgrade();
//! assert!(origin_observer.is_alive());
//! drop(event);
//! assert!(!origin_observer.is_alive());
//! ```
//!
//! Shared objects have `from_value` and `into_local`; exclusive objects have
//! `into_box` and `into_local_box`. Each takes a complete native value. Local
//! owners provide checked `try_borrow`/`try_borrow_mut` access to concrete state.
//! Factories can finish private weak-origin wiring before publishing a result.
//! Each allocation stores typed state, lifetime counters and one call-borrow
//! gate. Interface projections share its object address and use static tables.
//!
//! [`ComPtr`] owns a shared reference; [`ComBox`] owns the sole exclusive token.
//! [`ComRef`] lends a call-scoped interface. [`WeakComPtr`] promotes to shared
//! ownership, whereas [`WeakComBox`] observes an exclusive object's liveness
//! and enters its checked call gate without making another owner. `&mut self`
//! methods require exclusive call admission; conflicts report [`NC_E_BUSY`].
//! [`NC_E_DROPPED`] reports lost ownership, while [`NC_E_CLOSED`] reports an
//! explicitly closed live service. A dependency may propagate either status.
//!
//! Generated clients use the same `handle.method()` syntax for owned and borrowed
//! handles, including inherited methods. Querying a projection compares its
//! exact identity and signature. Public method names occupied by handle helpers
//! are rejected by IDL using the core-owned member inventory.
//!
//! Application records and enums have ordinary native Rust layout. [`ComValue`]
//! supplies a separate explicit associated `Wire` and typed backing `Storage`.
//! Copied positions require self-contained [`ComInlineValue`] wires; owning
//! records use [`NcValueRef`] for synchronous borrowing. A receiver explicitly
//! acquires owned fields with `to_owned()` to keep them beyond that call.
//! Identity borrowing requires the codec's recursive layout and validity proof;
//! equal size alone does not make a native value a wire representation.
//!
//! Readable buffers use [`NcCopyFromBufferOffset`]. Write-only outputs use
//! `NcOutputBuffer` or [`NcOutput::from_slots`]; generated providers receive an
//! [`NcOutputWriter`]. Old destination bytes are never input. Only a successful,
//! fully validated initialized prefix is published. Shared traversal budgets
//! charge actual selected fields; isolated consuming-output admission reserves
//! the declared maximum before any exclusive owner is accepted.
//!
//! [`ComSerialize`] and [`ComDeserialize`] are positional NCB persistence traits.
//! The optional derives apply to ordinary native data independently of a live
//! COM contract. Named fields are stored in declaration order and variants use
//! ordinals; NCB has no named-field or rename mode. Packed native records need
//! an explicit aligned-field serialization codec. IDL owns no native packing,
//! alignment, or external-field admission switches.
//!
//! With `runtime`, `Runtime::global` is a stateless facade over the runtime
//! chosen by the final assembly. Native hosts and plugins import the same shared
//! `nc_com` library and global registry. Libraries and facade features never
//! select a second implementation. A final monolith/browser explicitly retains
//! `neocogi-com-runtime`; plugin registration and typed acquisition still use
//! one catalog in that assembly. Caller-owned persistence registries and event
//! queues have separate ownership and do not replace the global plugin catalog.
//!
//! Built-in [`events`] contracts are always generated, independently of optional
//! macro support or runtime linkage. Posting transfers an immutable event owner
//! and captures a weak origin; every rejected transfer returns the original
//! owner. The standard channel queues facts for later handling. Unobserved
//! producers still construct and drop their events through the same ownership
//! path. Widgets derive origins from traversal; services retain their own weak
//! origins. Event and error payloads remain readable after producer destruction.
//!
//! Default features are `std` and `macros`. `alloc` enables owned values and
//! output buffers; `macros` exports implementation/persistence procedures.
//! `runtime` enables stateless clients and implies `std`, without choosing linkage.
//! With all defaults disabled, raw ABI types, event/persistence contracts and
//! allocation-free pointer/client support remain available.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

#[doc(hidden)]
extern crate self as neocogi_com;

#[cfg(feature = "alloc")]
#[doc(hidden)]
pub extern crate alloc as __alloc;

#[cfg(feature = "alloc")]
mod data;
/// Generated source-level descriptions used by typed authoring tools.
pub mod design;
/// Built-in typed queue contracts and construction-bound producer storage.
pub mod events;
mod macros;
mod object;
pub mod persistence;
#[cfg(feature = "runtime")]
mod runtime;

#[cfg(all(test, feature = "std"))]
mod tests;

pub use neocogi_com_core::lifetime;
pub use neocogi_com_core::{ComBox, WeakComBox};
pub use neocogi_com_core::{NcCopyFromBufferOffset, WeakComPtr, NcWeakRaw};
#[cfg(feature = "alloc")]
pub use data::{INcBytesComPtrExt, INcStringComPtrExt, NC_COM_DEFAULT_OWNED_COPY_MAX_BYTES, NcBytes, NcString};
#[cfg(feature = "macros")]
pub use macros::{ComDeserialize, ComSerialize, com_object, plugin};
#[cfg(feature = "alloc")]
pub use neocogi_com_core::{format_abi_signature, format_method_signature};
pub use neocogi_com_core::{
    __nc_abi_plain_type, __nc_abi_type, AbiPlainValue, AbiType, ComClientInterface, ComDeserialize, ComSerialize, ComInterface, ComPtr, ComRaw, ComRef,
    NcClientPointer, IID_INC_BYTES, IID_INC_STRING, IID_IUNKNOWN, INcBytesAbi, INcBytesVTable, INcStringAbi, INcStringVTable, IUNKNOWN_INTERFACE_ID,
    IUNKNOWN_QUALIFIED_NAME, IUnknown, IUnknownVTable, NC_ABI_PASSING_COM_PTR_OUT, NC_ABI_PASSING_COM_BOX_IN, NC_ABI_PASSING_COM_BOX_OUT,
    NC_ABI_PASSING_WEAK_COM_BOX_OUT, NC_ABI_PASSING_WEAK_COM_PTR_OUT, NC_ABI_PASSING_COM_REF, NC_ABI_PASSING_COPY_FROM_BUFFER_OFFSET,
    NC_ABI_PASSING_COPY_TO_BUFFER_OFFSET, NC_ABI_PASSING_OUTPUT_BUFFER, NC_ABI_PASSING_OUT, NC_ABI_PASSING_BORROWED_VALUE, NC_ABI_PASSING_VALUE,
    NC_ABI_PASSING_VOID, NC_ABI_RECEIVER_MUTABLE, NC_ABI_RECEIVER_SHARED, NC_ABI_THREADING_THREAD_LOCAL, NC_ABI_THREADING_THREAD_SAFE, NC_ABI_TYPE_AGGREGATE,
    NC_ABI_TYPE_BOOL, NC_ABI_TYPE_F32, NC_ABI_TYPE_F64, NC_ABI_TYPE_I8, NC_ABI_TYPE_I16, NC_ABI_TYPE_I32, NC_ABI_TYPE_I64, NC_ABI_TYPE_INTERFACE,
    NC_ABI_TYPE_ISIZE, NC_ABI_TYPE_NC_GUID, NC_ABI_TYPE_NC_STATUS, NC_ABI_TYPE_U8, NC_ABI_TYPE_U16, NC_ABI_TYPE_U32, NC_ABI_TYPE_U64, NC_ABI_TYPE_USIZE,
    NC_ABI_TYPE_VOID, NC_COM_CORE_ABI_VERSION, NC_COM_GUID_NAMESPACE, NC_COM_RUNTIME_ABI_VERSION, NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS,
    NC_COM_RUNTIME_MAX_DYNAMIC_PLUGINS, NC_DEBUG_SEVERITY_ALL, NC_DEBUG_SEVERITY_ERROR, NC_DEBUG_SEVERITY_INFO, NC_DEBUG_SEVERITY_WARNING, NC_DEBUG_TYPE_ALL,
    NC_DEBUG_TYPE_PLUGIN, NC_DEBUG_TYPE_RUNTIME, NC_DEBUG_TYPE_VALIDATION, NC_DESCRIPTOR_QUALIFIED_NAME_MAX_LEN, NC_DESCRIPTOR_VERSION_MAX_LEN,
    NC_E_ABI_VERSION, NC_E_BUSY, NC_E_ALREADY_BOUND, NC_E_CLOSED, NC_E_DROPPED, NC_E_INSUFFICIENT_BUFFER, NC_E_INVALID_ARG, NC_E_INVALID_TAG, NC_E_LOAD_FAILED,
    NC_E_NOINTERFACE, NC_E_NOT_FOUND, NC_E_OUT_OF_MEMORY, NC_E_OUT_OF_RANGE, NC_E_POINTER, NC_PLUGIN_DESCRIPTOR_ABI_VERSION, NC_GET_PLUGIN_DESCRIPTOR_SYMBOL,
    NC_GUID_NIL, NC_OK, NcAbiSignature, NcAbiPassing, NcAbiPassingDescriptor, NcAbiReceiver, NcAbiThreadingMode, NcAbiType, NcAbiTypeDescriptor,
    NcAcquirePluginFactory, NcDebugApiV1, NcDebugCallbackV1, NcDebugMessageV1, NcDebugMessengerCreateInfoV1, NcPluginDescriptor, NcGetPluginDescriptor, NcGuid,
    NcInspectPluginDescriptorCallbackV1, NcInterfaceDescriptor, NcInterfaceId, NcMethodDescriptor, NcMethodParamDescriptor, NcPersistenceApiV1,
    NcPersistenceContext, NcPersistenceLimitsV1, NcPersistenceWriteBytesV1, NcMethodReturnDescriptor, NcPluginDependencyDescriptor, NcFactoryInfoV1,
    NcPluginInfoV1, NcQualifiedNameError, NcRuntimeApiV1, NcStaticUtf8, NcStatus, NcUtf8View, NcVersionTextError, ParseGuidError, QUALIFIED_NAME_INC_BYTES,
    QUALIFIED_NAME_INC_STRING, ThreadLocal, ThreadSafe, ThreadingModel, PluginRootAcquire, write_plugin_root, guid_from_qualified_name, raw,
    return_plugin_descriptor, validate_plugin_descriptor, validate_qualified_name, validate_version_text, validate_rust_path, NcRustPathError,
    NC_DESCRIPTOR_RUST_PATH_MAX_LEN,
};
pub use object::{
    __ComCallGate, __ComSharedCall, __ComExclusiveCall, __ComExclusiveLease, __ComSharedLease, __com_add_ref_count, __com_release_ref_count, ComObject,
};
#[cfg(feature = "alloc")]
#[doc(hidden)]
pub use object::{__ComBoxStorage, __ComSharedStorage, __ComStorage};
mod local;
pub use local::{LocalBox, WeakLocalBox, LocalCom, WeakLocalCom, LocalMut, LocalRef};
#[doc(hidden)]
pub use neocogi_com_core::__ComSupportsInterfaceThreading;
#[cfg(feature = "runtime")]
pub use runtime::{
    Debug, DebugMessenger, NC_COM_RUNTIME_MAX_ENUMERATION_ITEMS, NC_COM_RUNTIME_MAX_ENUMERATION_RETRIES, NC_COM_RUNTIME_MAX_INSPECTION_TEXT_BYTES, FactoryInfo,
    PluginInfo, PluginOrigin, Runtime,
};

#[doc(hidden)]
pub use neocogi_com_core::{
    __ComProjection, __nc_interface_chain, __nc_object_interface_count, __nc_projection_table, __nc_projection_vtables, __nc_find_interface,
};

pub use neocogi_com_core::binary;

/// Compile the runnable README examples against this crate's real public API.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme_examples {}

// Recursive value transport is independent of whether a value is used as a message.
pub use neocogi_com_core::{
    ComValue, ComValueType, ComInlineValue, NcValueRef, ValueBudget, ValueCost, ValueArray, ValueOption, ValueSlice, encode_inline, decode_inline,
    check_inline_output, check_buffer_output, MAX_VALUE_BYTES, MAX_VALUE_COUNT, MAX_VALUE_DEPTH,
};
#[cfg(feature = "alloc")]
pub use neocogi_com_core::{ValueSequenceStorage, encode_value_buffer, decode_value_buffer};

#[doc(hidden)]
pub use neocogi_com_core::{EncodedInput, DecodedInput};
pub use neocogi_com_core::{NcOutput, NcOutputWriter, PreparedOutput};
#[cfg(feature = "alloc")]
pub use neocogi_com_core::NcOutputBuffer;

// Only this test executable retains an implementation, independently of facade features.
#[cfg(all(test, feature = "runtime"))]
extern crate nc_com as _;
