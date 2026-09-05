//! A small C-ABI COM runtime for cross-platform Rust component interop.
//!
//! `nc-com` keeps the binary boundary explicit: interfaces are represented as
//! `#[repr(C)]` structs containing a vtable pointer and an opaque object
//! pointer, methods are `extern "C"` function pointers, and errors are returned
//! as [`NcStatus`] values. The Rust traits used with `com_interface` are the
//! implementation surface; the generated `*Abi` types are the portable ABI
//! surface.
//! `com_interface` also generates the `unsafe impl` of [`ComInterface`] that
//! certifies the header, vtable prefix, identity/hash, lifetime, and [`IUnknown`]
//! conversion invariants relied on by safe [`ComPtr`] operations. The trait is
//! unsealed for low-level FFI adapters, but handwritten implementations are an
//! unsafe extension point rather than the normal interface-definition path.
//!
//! The goal is Rust-to-Rust interoperability across independently built
//! artifacts: different crate graphs, compiler versions, optimization settings,
//! or linking modes. The crate borrows the useful cross-platform parts of COM
//! (`IUnknown`, `QueryInterface`, `AddRef`, `Release`, interface IDs, and
//! vtables), but it is not Windows COM and does not implement apartments,
//! registry activation, `HRESULT`, or system COM marshalling.
//!
//! This crate is also not a C SDK generator. The ABI is C-compatible, but the
//! runtime and macros are focused on defining and enforcing the Rust-side ABI
//! model.
//!
//! # Scope
//!
//! `nc-com` is Rust-to-Rust component ABI glue. It is intended for hosts and
//! components that are both written in Rust but are built as separate artifacts
//! with independent compiler versions, crate graphs, optimization settings, or
//! link modes.
//!
//! The explicit contract is the generated C-shaped surface: `#[repr(C)]`
//! interface headers, `extern "C"` vtable functions, explicit [`NcStatus`]
//! values, GUIDs, ABI hashes, reference-counted object lifetime, and host-owned
//! plugin/class metadata. Rust traits, generics, trait objects, references,
//! `String`, `Vec<T>`, slices, allocator ownership, and unwinding panics are
//! local implementation details and must not cross the ABI boundary.
//!
//! This is not Windows COM. There are no apartments, no Windows registry
//! activation, no `HRESULT`, no operating-system `IUnknown` contract, and no
//! system marshalling. `nc-com` uses a COM-style shape because it is a compact
//! cross-platform ABI pattern.
//!
//! With the `runtime` feature, `nc-com` exposes the process-global
//! `Runtime` facade backed by the linked `libnc_com_runtime` shared library.
//! Hosts still choose which plugin descriptors to accept, but successful dynamic
//! plugin loads remain mapped until process exit.
//!
//! Interface casts are performed through [`IUnknown`] / `QueryInterface`, not by
//! pointer casts. `QueryInterface` checks both interface ID and generated ABI
//! hash before returning a typed pointer. A single object can expose several
//! interfaces at different addresses inside the same allocation.
//!
//! COM reference counts are atomic, and the built-in registry is
//! thread-safe. User object method calls are not assumed to be thread-safe.
//! [`ComPtr<T>`](ComPtr) is thread-local by default. Classes declared with
//! `#[com_class(thread_safe, ...)]` produce
//! `ComPtr<T, ThreadSafe>`, which can be sent and shared across threads.
//! Generated [`ComClass::Threading`] implementations make that strongest class
//! capability the source for constructors and descriptor metadata. A
//! thread-safe pointer can be explicitly weakened to a thread-local owner or
//! borrow without changing the underlying COM reference.
//! `Runtime::construct_class` requests a conservative thread-local pointer;
//! `Runtime::construct_class_with_model` allows an explicit checked request
//! for `ThreadSafe`. The runtime rejects unsafe upgrades before invoking plugin
//! constructors.
//!
//! | Handle | Owns a COM reference | `Send` | `Sync` | Cross-thread capability |
//! |---|:---:|:---:|:---:|---|
//! | `ComPtr<T, ThreadLocal>` | Yes | No | No | None |
//! | `ComPtr<T, ThreadSafe>` | Yes | Yes | Yes | Calls, transfer, sharing, and final release |
//! | `ComRef<'a, T, ThreadLocal>` | No | No | No | None |
//! | `ComRef<'a, T, ThreadSafe>` | No | Yes | Yes | Calls, transfer, and sharing within `'a` |
//!
//! This marker cannot be inferred from `T`: an interface describes callable
//! methods, and both thread-local and thread-safe classes may implement the same
//! interface. It also cannot be recovered from the raw ABI pointer, whose
//! layout contains no threading tag. [`ComClass::Threading`] records the
//! concrete producer's strongest capability while the class is known; the
//! pointer marker preserves that evidence after construction and
//! `QueryInterface` erase the concrete class type. In particular, making the
//! reference count atomic protects object lifetime but says nothing about
//! concurrent method access or which thread may run the destructor.
//!
//! # Object Lifetime
//!
//! COM object lifetime is owned by reference counting. When `Release` transitions
//! the count from one to zero, generated storage is destroyed immediately. Final
//! release is not a fallible cleanup boundary, cannot defer destruction, and must
//! not resurrect the object. If an object owns resources whose cleanup can fail,
//! expose an explicit domain method such as `close`, `flush`, or `shutdown` and
//! require callers to invoke it while they still hold a COM reference.
//!
//! Generated reference counts are `u32` values. Debug builds assert if `AddRef`
//! would overflow or if `Release` observes an already-zero count. Release builds
//! keep the counter from wrapping: `AddRef` saturates at `u32::MAX`, and
//! `Release` refuses to underflow. Reaching either edge is still a broken COM
//! ownership contract; returned counts are diagnostic values, not a protocol for
//! managing object lifetime.
//!
//! # Build Compatibility
//!
//! ABI v1 is currently an experimental pre-alpha draft. The generated C-shaped
//! surface is the intended compatibility boundary, but cross-release
//! compatibility does not begin until an explicitly named future ratification
//! milestone. Draft golden vectors prevent accidental drift within the current
//! revision and may be intentionally updated before that freeze.
//!
//! These factors may differ between host and component artifacts built against
//! the same explicit ABI contract:
//! Rust compiler version, Cargo dependency graph, optimization level, LTO,
//! codegen units, debug assertions, static versus dynamic linking, allocator
//! choice, and `std`/`alloc`/`no_std` feature choices when the exposed ABI is
//! unchanged.
//!
//! These factors must match or be treated as a new ABI contract: target process
//! ABI, pointer width, endianness, C calling convention, `NC_COM_CORE_ABI_VERSION`,
//! interface GUIDs, method order, lowered method types, and interface ABI hashes.
//! Incompatible core ABI generations use a new core ABI version. A future ABI
//! may also use a different descriptor symbol if the entrypoint function shape
//! or discovery rules change.
//!
//! Once ratified, published interface contracts must not be mutated in place.
//! Adding a method, changing method order or lowered types, changing ownership
//! semantics, or changing the ABI hash grammar requires a new versioned
//! interface ID. Built-in status-code numbers are stable once published and
//! must not be renumbered.
//! or reused.
//!
//! Proc macros run at compile time and are not linked into the target artifact
//! that LTO optimizes. LTO can optimize inside an artifact, but the component
//! boundary remains the generated `extern "C"` vtable.
//!
//! Panics are not represented as [`NcStatus`]. With `panic=abort`, the process
//! aborts at the panic site. With unwinding, a panic that reaches an
//! `extern "C"` ABI thunk aborts under Rust's ABI unwind rules. Expected
//! failures should be returned as [`NcStatus`] values.
//!
//! # Features
//!
//! The runtime is `no_std` when the `std` feature is disabled. Generated
//! interfaces and [`ComPtr`] work without allocation. Generated classes require
//! the `alloc` feature because `#[com_class]` stores each COM object in a
//! reference-counted heap allocation. `Runtime` requires the `runtime`
//! feature, which also enables `std`; `std` enables `alloc`.
//!
//! ```
//! use nc_com::{com_class, com_interface, ComPtr, IUnknown, NcStatus};
//!
//! #[com_interface(auto_guid = "nc.docs.Calculator:v1")]
//! pub trait ICalculator {
//!     fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus>;
//! }
//!
//! #[com_class(interfaces(ICalculator))]
//! pub struct Calculator;
//!
//! impl ICalculator for Calculator {
//!     fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus> {
//!         Ok(a + b)
//!     }
//! }
//!
//! let unknown: ComPtr<IUnknown> = Calculator::create();
//! let calculator = unknown.query::<ICalculatorAbi>()?;
//! assert_eq!(calculator.add(2, 3)?, 5);
//! # Ok::<(), NcStatus>(())
//! ```
//!
//! Classes with fields are created with the generated `from_value` constructor:
//!
//! ```
//! use nc_com::{com_class, com_interface, ComPtr, IUnknown, NcStatus};
//!
//! #[com_interface(auto_guid = "nc.docs.Counter:v1")]
//! pub trait ICounter {
//!     fn value(&self) -> Result<i32, NcStatus>;
//! }
//!
//! #[com_class(interfaces(ICounter))]
//! pub struct Counter {
//!     value: i32,
//! }
//!
//! impl ICounter for Counter {
//!     fn value(&self) -> Result<i32, NcStatus> {
//!         Ok(self.value)
//!     }
//! }
//!
//! let unknown: ComPtr<IUnknown> = Counter::from_value(Counter { value: 7 });
//! let counter = unknown.query::<ICounterAbi>()?;
//! assert_eq!(counter.value()?, 7);
//! # Ok::<(), NcStatus>(())
//! ```
//!
//! Prefer `auto_guid = "stable.abi.Name:v1"` for new interfaces. The exact
//! qualified-name UTF-8 bytes are stored in the descriptor and hashed with the
//! specified `nc-com` namespace to produce a deterministic UUIDv5 interface
//! ID. Explicit UUID syntax additionally requires the same `name = "..."` and
//! serves as a compile-time golden assertion of that derivation. Generated
//! interface, class, and plugin descriptors validate their qualified-name and
//! version literals during macro expansion. Direct metadata producers can use
//! [`validate_qualified_name`] and [`validate_version_text`] to apply the same
//! allocation-free policy, including in const contexts.
//!
//! ```no_run
//! use nc_com::com_interface;
//!
//! #[com_interface(
//!     "bfec7c45-902c-5f94-84ec-77df282eb9f6",
//!     name = "example.Calculator:v1"
//! )]
//! pub trait IPinnedCalculator {}
//! ```
//!
//! COM interface method arguments and non-void successful return payloads must
//! be plain [`AbiType`] values or one of the explicitly modeled ABI passing
//! forms. `Result<(), NcStatus>` is the void-success form and lowers to a
//! status-only ABI function with no output pointer. A method's `Result::Err`
//! must contain a negative status. Generated thunks preserve negative errors
//! and normalize a non-negative error to [`NC_E_INVALID_ARG`] without writing
//! an output slot. The built-in ABI-safe value set includes `bool`, fixed-width
//! integers, pointer-sized integers,
//! floating-point scalars, [`NcStatus`], [`NcGuid`], fixed-size records generated by
//! `com_struct`, and transparent integer newtypes generated by
//! [`com_int_enum!`]. Borrowed interface arguments use [`ComRef`]; returned
//! interface references use [`ComPtr`]. Bounded call-scoped buffers use
//! [`NcCopyFromBufferOffset`] and [`NcCopyToBufferOffset`].
//! Methods named `query_interface`, `add_ref`, or `release` are reserved for
//! generated `IUnknown` lifecycle thunks. Parameter names remain readable in
//! generated ABI signatures; a focused macro error is emitted only when a name
//! would duplicate a lowered buffer slot or shadow a generated adapter binding.
//!
//! Owned or long-lived variable-size data such as strings, byte buffers, arrays,
//! streams, and documents should cross the ABI as COM interfaces. Do not pass
//! Rust `String`, `Vec<T>`, slices, references, raw buffers, or allocator-owned
//! memory across the boundary directly. Use [`INcStringAbi`] and `NcString` for
//! common read-only UTF-8 strings, and [`INcBytesAbi`] and `NcBytes` for common
//! read-only byte buffers. Both built-in data interfaces provide bounded bulk
//! copying into caller-owned storage. A borrowed `ComRef<'_, IFooAbi>` is valid
//! for one call; a returned `ComPtr<IFooAbi>` transfers one owned COM reference
//! to the caller.
//! For streaming data, expose bounded methods such as
//! `next_chunk(max_len) -> Result<ComPtr<IReadOnlyBytesAbi>, NcStatus>`; a
//! successful chunk must contain at most `max_len` bytes, and an empty chunk can
//! represent end-of-stream.
//!
//! Shared libraries expose plugins through the
//! [`NC_GET_PLUGIN_DESCRIPTOR_SYMBOL`] entrypoint. The descriptor records the
//! [`NC_COM_CORE_ABI_VERSION`], plugin identity, and a static [`NcClassTable`]
//! of [`NcClassDescriptor`] values. Generated interfaces expose
//! `ABI_HASH`, typed method descriptors, rendered diagnostic method signature
//! text, and [`NcInterfaceDescriptor`]
//! metadata; generated classes expose `COM_INTERFACES` metadata containing the
//! canonical [`IUnknown`] descriptor first, followed by their `interfaces(...)`
//! list in declaration order. `IUnknown` is implicit and must not be listed by
//! users. Constructors return owned [`IUnknown`] pointers.
//! Loaded plugin libraries are retained by the process-global `Runtime` and
//! can be inspected through process-local `LoadedModuleId` values.
//! `LoadedModuleInfo` combines module provenance (ID, path, and plugin ID)
//! with qualified name and version metadata read from the registered plugin
//! descriptor. Hosts can retrieve it again with
//! `Inspection::loaded_module`, or query descriptor metadata directly with
//! `Inspection::plugin_info`. Statically registered plugins have plugin
//! metadata but no loaded-module path.
//! Runtime ABI v1 retains at most [`NC_COM_RUNTIME_MAX_LOADED_MODULES`] dynamic
//! modules and permits at most
//! [`NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS`] simultaneous debug messenger
//! registrations. Exceeding the module limit is a fatal live-activation error;
//! debug messenger capacity exhaustion returns [`NC_E_OUT_OF_RANGE`].
//! Callers then use hash-checked `QueryInterface` to obtain the interface they
//! need.
//! ABI v1 plugin descriptors are declarative and have no initialization hook.
//! They may declare hard dependency plugin IDs. Registration requires those IDs
//! to exist already but never discovers paths or loads dependencies recursively.
//! Native loader dependencies remain distinct from nc-com dependencies.
//! Raw plugin descriptors can be checked with
//! [`validate_plugin_descriptor_with_issues`], which reports structured
//! descriptor paths and issue kinds for diagnostics. Descriptor validation is for
//! trusted plugin metadata shape; it is not hostile plugin isolation, does not
//! sandbox plugin code, and cannot make an untrusted shared library safe to load.
//!
//! Hosts can load required bundled plugins during startup with
//! `Runtime::load_plugin_libraries`, which loads paths sequentially in an
//! inspector-produced dependency order. Any load, descriptor, dependency, or
//! registration failure emits its diagnostic and aborts the process.
//! Required cross-plugin contracts should appear in constructor arguments,
//! method arguments, return values, or dependency/capability interfaces so the generated
//! interface IDs and ABI hashes describe the actual binary dependency.
//!
//! Loading a dynamic library can run platform loader code that mutates
//! process-global state. `nc-com` therefore does not support dynamic plugin
//! unload or hot reload. Successful modules remain mapped for the process
//! lifetime; failed post-open activation pins the module before aborting.
//!
//! Fixed-size plain records can use `com_struct`. The macro verifies that
//! every field implements [`AbiPlainValue`], owns the Rust representation, records
//! size/alignment/field offsets in [`ComStruct`], and then implements [`AbiType`]
//! and [`AbiPlainValue`] for the struct:
//!
//! ```
//! use nc_com::com_struct;
//!
//! #[com_struct]
//! #[derive(Debug, PartialEq)]
//! pub struct Rect {
//!     pub x: f32,
//!     pub y: f32,
//!     pub width: f32,
//!     pub height: f32,
//! }
//! ```
//!
//! Use `#[com_struct(packed)]`, `#[com_struct(packed(N))]`, or
//! `#[com_struct(align(N))]` only when that physical layout is part of the
//! contract. The generated aggregate ABI hash includes the struct size,
//! alignment, field order, field offsets, and field ABI types.
//!
//! Enum-like ABI scalar values should use transparent integer newtypes generated
//! by [`com_int_enum!`]:
//!
//! ```
//! use nc_com::com_int_enum;
//!
//! com_int_enum! {
//!     pub enum NodeType: u16 {
//!         ELEMENT = 1,
//!         TEXT = 3,
//!         COMMENT = 8,
//!         DOCUMENT = 9,
//!     }
//! }
//!
//! assert_eq!(NodeType::ELEMENT.raw(), 1);
//! assert!(NodeType::from_raw(42).is_unknown());
//! ```
//!
//! Tagged payload enum values can use `com_enum`. The Rust enum remains
//! available for local code, while the macro generates an opaque `NameAbi` with
//! an explicit `repr(C)` tag + payload-union binary layout. Payload fields must
//! implement [`AbiPlainValue`]. Safe code constructs known values with `From`,
//! inspects `tag()`, and decodes with `TryFrom`; arbitrary raw tag/payload pairing
//! is an unsafe FFI operation. Prefer explicit scalar handles instead of owning
//! Rust containers or borrowed raw pointers when carrying references to external
//! objects:
//!
//! ```
//! mod contract {
//!     use nc_com::com_enum;
//!
//!     #[com_enum(tag = u32)]
//!     #[derive(Debug, PartialEq)]
//!     pub enum Mutation {
//!         #[com_tag(10)]
//!         None,
//!         #[com_tag(20)]
//!         ChildAdded(u64),
//!         #[com_tag(30)]
//!         TextChanged { node: u64, len: u32 },
//!     }
//! }
//!
//! use contract::{Mutation, MutationAbi, MutationTag};
//!
//! let abi = MutationAbi::from(Mutation::TextChanged { node: 7, len: 12 });
//! assert_eq!(abi.tag(), MutationTag::TextChanged);
//! assert_eq!(abi.tag().raw(), 30);
//! assert_eq!(
//!     Mutation::try_from(abi).unwrap(),
//!     Mutation::TextChanged { node: 7, len: 12 },
//! );
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

#[doc(hidden)]
extern crate self as nc_com;

#[cfg(feature = "alloc")]
#[doc(hidden)]
pub extern crate alloc as __alloc;

mod buffer;
#[cfg(feature = "alloc")]
mod data;
mod macros;
mod object;
#[cfg(feature = "runtime")]
mod runtime;

#[cfg(all(test, feature = "std"))]
mod tests;

pub use buffer::{NcCopyFromBufferOffset, NcCopyToBufferOffset};
#[cfg(feature = "alloc")]
pub use data::{
    INcBytesComPtrExt, INcStringComPtrExt, NC_COM_DEFAULT_OWNED_COPY_MAX_BYTES, NcBytes, NcString,
};
#[cfg(feature = "macros")]
pub use macros::{com_class, com_enum, com_interface, com_struct, plugin};
#[cfg(feature = "alloc")]
pub use nc_com_core::format_method_signature;
pub use nc_com_core::{
    __NcTaggedEnumAbiStorage, __nc_abi_com_struct_type, __nc_abi_plain_type, __nc_abi_type,
    AbiPlainValue, AbiType, ComInterface, ComPtr, ComRef, ComStruct, IID_INC_BYTES, IID_INC_STRING,
    IID_IUNKNOWN, INcBytesAbi, INcBytesVTable, INcStringAbi, INcStringVTable,
    IUNKNOWN_INTERFACE_ID, IUNKNOWN_QUALIFIED_NAME, IUnknown, IUnknownVTable,
    NC_ABI_PASSING_COM_PTR_OUT, NC_ABI_PASSING_COM_REF, NC_ABI_PASSING_COPY_FROM_BUFFER_OFFSET,
    NC_ABI_PASSING_COPY_TO_BUFFER_OFFSET, NC_ABI_PASSING_OUT, NC_ABI_PASSING_VALUE,
    NC_ABI_PASSING_VOID, NC_ABI_THREADING_THREAD_LOCAL, NC_ABI_THREADING_THREAD_SAFE,
    NC_ABI_TYPE_AGGREGATE, NC_ABI_TYPE_BOOL, NC_ABI_TYPE_F32, NC_ABI_TYPE_F64, NC_ABI_TYPE_I8,
    NC_ABI_TYPE_I16, NC_ABI_TYPE_I32, NC_ABI_TYPE_I64, NC_ABI_TYPE_INTERFACE, NC_ABI_TYPE_ISIZE,
    NC_ABI_TYPE_NC_ABI_HASH, NC_ABI_TYPE_NC_GUID, NC_ABI_TYPE_NC_STATUS, NC_ABI_TYPE_U8,
    NC_ABI_TYPE_U16, NC_ABI_TYPE_U32, NC_ABI_TYPE_U64, NC_ABI_TYPE_USIZE, NC_ABI_TYPE_VOID,
    NC_COM_CORE_ABI_VERSION, NC_COM_GUID_NAMESPACE, NC_COM_RUNTIME_ABI_VERSION,
    NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS, NC_COM_RUNTIME_MAX_LOADED_MODULES,
    NC_DEBUG_SEVERITY_ALL, NC_DEBUG_SEVERITY_ERROR, NC_DEBUG_SEVERITY_INFO,
    NC_DEBUG_SEVERITY_WARNING, NC_DEBUG_TYPE_ALL, NC_DEBUG_TYPE_PLUGIN, NC_DEBUG_TYPE_RUNTIME,
    NC_DEBUG_TYPE_VALIDATION, NC_DESCRIPTOR_QUALIFIED_NAME_MAX_LEN, NC_DESCRIPTOR_VERSION_MAX_LEN,
    NC_E_ABI_VERSION, NC_E_INSUFFICIENT_BUFFER, NC_E_INVALID_ARG, NC_E_INVALID_TAG,
    NC_E_LOAD_FAILED, NC_E_NOINTERFACE, NC_E_NOT_FOUND, NC_E_OUT_OF_MEMORY, NC_E_OUT_OF_RANGE,
    NC_E_POINTER, NC_GET_PLUGIN_DESCRIPTOR_SYMBOL, NC_GUID_NIL, NC_OK, NcAbiAggregateKind,
    NcAbiFieldLayout, NcAbiHash, NcAbiHashPart, NcAbiPassing, NcAbiPassingDescriptor,
    NcAbiStructLayout, NcAbiThreadingMode, NcAbiType, NcAbiTypeDescriptor, NcClassDescriptor,
    NcClassInfoV1, NcClassInterfaceDescriptor, NcClassTable, NcConstructorFn, NcDebugApiV1,
    NcDebugCallbackV1, NcDebugMessageV1, NcDebugMessengerCreateInfoV1, NcDescriptorPath,
    NcDescriptorValidationIssue, NcDescriptorValidationKind, NcGetPluginDescriptor, NcGuid,
    NcInspectPluginDescriptorCallbackV1, NcInspectionApiV1, NcInterfaceDescriptor, NcInterfaceId,
    NcMethodDescriptor, NcMethodParamDescriptor, NcMethodReturnDescriptor, NcModuleInfoV1,
    NcPluginDependencyDescriptor, NcPluginDescriptor, NcPluginInfoV1, NcPluginRegistryV1,
    NcQualifiedNameError, NcRuntimeApiV1, NcStaticUtf8, NcStatus, NcTypeRegistryV1, NcUtf8View,
    NcVersionTextError, ParseGuidError, QUALIFIED_NAME_INC_BYTES, QUALIFIED_NAME_INC_STRING,
    ThreadLocal, ThreadSafe, ThreadingModel, construct_class_checked, guid_from_qualified_name,
    query_constructor_arguments, raw, return_plugin_descriptor, validate_constructor_arguments,
    validate_plugin_descriptor_with_issues, validate_qualified_name, validate_version_text,
};
pub use object::{
    __com_add_ref_count, __com_release_ref_count, ComAbiObject, ComClass, ThreadSafeComObject,
};
#[cfg(feature = "runtime")]
pub use runtime::{
    ClassInfo, Debug, DebugMessenger, Inspection, LoadedModuleId, LoadedModuleInfo,
    NC_COM_RUNTIME_MAX_ENUMERATION_ITEMS, NC_COM_RUNTIME_MAX_ENUMERATION_RETRIES,
    NC_COM_RUNTIME_MAX_INSPECTION_TEXT_BYTES, PluginInfo, PluginRegistry, Runtime, TypeRegistry,
};
