//! Typed COM clients and object implementation helpers for Rust component interop.
//!
//! `neocogi-com` keeps the binary boundary explicit: interfaces are represented as
//! `#[repr(C)]` object/table pairs. Methods receive the object address directly
//! through `extern "C"` function pointers, and errors are returned
//! as [`NcStatus`] values. IDL-generated Rust traits are the implementation
//! surface; the generated `*Abi` types are the portable ABI surface.
//! `com_interface` also generates the `unsafe impl` of [`ComInterface`] that
//! certifies the pair, vtable prefix, identity/signature, lifetime, and lifecycle
//! invariants relied on by safe [`ComPtr`] operations. The trait is
//! unsealed for low-level FFI adapters, but handwritten implementations are an
//! unsafe extension point rather than the normal interface-definition path.
//!
//! The goal is Rust-to-Rust interoperability across independently built
//! artifacts: different crate graphs, compiler versions, optimization settings,
//! or linking modes. The crate borrows the useful cross-platform parts of COM
//! (`IUnknown`, `QueryInterface`, `AddRef`, `Release`, interface IDs, and
//! vtables), but it is not Windows COM and does not implement apartments,
//! registry acquisition, `HRESULT`, or system COM marshalling.
//!
//! This crate is also not a C SDK generator. The ABI is C-compatible, but the
//! runtime and generator-facing macros enforce the Rust-side ABI model.
//!
//! # Scope
//!
//! `neocogi-com` is Rust-to-Rust component ABI glue. It is intended for hosts and
//! components that are both written in Rust but are built as separate artifacts
//! with independent compiler versions, crate graphs, optimization settings, or
//! link modes.
//!
//! The explicit contract is the generated C-shaped surface: `#[repr(C)]`
//! object/table pairs, `extern "C"` vtable functions, explicit [`NcStatus`]
//! values, GUIDs, ABI signatures, reference-counted object lifetime, and
//! immutable contract/plugin descriptions. Rust traits, generics, trait objects, references,
//! `String`, `Vec<T>`, slices, allocator ownership, and unwinding panics are
//! local implementation details and must not cross the ABI boundary.
//!
//! This is not Windows COM. There are no apartments, no Windows registry
//! acquisition, no `HRESULT`, no operating-system `IUnknown` contract, and no
//! system marshalling. `neocogi-com` uses a COM-style shape because it is a compact
//! cross-platform ABI pattern.
//!
//! With the `runtime` feature, `neocogi-com` exposes the process-global
//! `Runtime` facade backed by the linked `libnc_com` shared library.
//! Hosts still choose which factory plugins to accept, but successful dynamic
//! plugin libraries remain mapped until process exit.
//! The facade and its internal link declarations contain no registry state. Cargo
//! feature unification may compile the facade into multiple dependents, but
//! their external calls resolve to the same loader-owned shared object and
//! therefore the same process registry. Static runtime linkage is reserved for
//! a single monolithic executable or browser module.
//!
//! Interface casts are performed through [`IUnknown`] / `QueryInterface`, not by
//! pointer casts. `QueryInterface` checks both interface ID and generated ABI
//! signature before returning a typed pointer. A single object can expose several
//! interfaces at different addresses inside the same allocation.
//!
//! COM reference counts are atomic, and the built-in registry is
//! thread-safe. User object method calls are not assumed to be thread-safe.
//! [`ComPtr<T>`](ComPtr) is thread-local by default. Objects declared with
//! `#[com_object(thread_safe, ...)]` produce
//! `ComPtr<T, ThreadSafe>`, which can be sent and shared across threads.
//! Generated [`ComObject::Threading`] implementations make that strongest object
//! capability the source for typed pointers and factory metadata. A
//! thread-safe pointer can be explicitly weakened to a thread-local owner or
//! borrow without changing the underlying COM reference.
//! `Runtime::acquire_plugin_factory` checks the generated root factory's
//! identity, signature, and threading capability before returning its typed
//! pointer. Later construction is expressed by ordinary IDL factory methods.
//!
//! | Handle | Owns a COM reference | `Send` | `Sync` | Cross-thread capability |
//! |---|:---:|:---:|:---:|---|
//! | `ComPtr<T, ThreadLocal>` | Yes | No | No | None |
//! | `ComPtr<T, ThreadSafe>` | Yes | Yes | Yes | Calls, transfer, sharing, and final release |
//! | `ComRef<'a, T, ThreadLocal>` | No | No | No | None |
//! | `ComRef<'a, T, ThreadSafe>` | No | Yes | Yes | Calls, transfer, and sharing within `'a` |
//!
//! This marker cannot be inferred from `T`: an interface describes callable
//! methods, and both thread-local and thread-safe objects may implement the same
//! interface. It also cannot be recovered from the raw ABI pointer, whose
//! layout contains no threading tag. [`ComObject::Threading`] records the
//! concrete producer's strongest capability while the implementation is known;
//! the pointer marker preserves that evidence after a factory return and
//! `QueryInterface` erase the concrete implementation type. In particular, making the
//! reference count atomic protects object lifetime but says nothing about
//! concurrent method access or which thread may run the destructor.
//!
//! # Object Lifetime
//!
//! COM object lifetime is owned by reference counting. When `Release` transitions
//! the strong count from one to zero, the concrete value is destroyed immediately.
//! Weak references may retain the empty allocation shell, but never its resources. Final
//! release is not a fallible cleanup boundary, cannot defer destruction, and must
//! not resurrect the object. If an object owns resources whose cleanup can fail,
//! expose an explicit domain method such as `close`, `flush`, or `shutdown` and
//! require callers to invoke it while they still hold a COM reference.
//!
//! Generated objects use a common COM strong/weak control header. Public returned counts
//! are diagnostic u32 snapshots, clamped if the private count is wider; they are
//! not a lifetime protocol. Weak promotion acquires ownership before accessing
//! state and never resurrects a destroyed value. See [`WeakComPtr`]. Handwritten
//! built-in providers use their documented reference-count primitives separately.
//! Both implementations record every returned strong owner or terminate at their
//! non-unwinding C entrypoint on counter exhaustion. Clamping a diagnostic return
//! value never permits saturation of the actual ownership count. This invariant
//! also applies to optimized builds with debug assertions disabled.
//!
//! # Build Compatibility
//!
//! [`NC_COM_CORE_ABI_VERSION`] selects the experimental pre-alpha core draft. The generated C-shaped
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
//! interface GUIDs, method order, lowered method types, and interface ABI signatures.
//! Incompatible core ABI generations use a new core ABI version. A future ABI
//! may also use a different descriptor symbol if the entrypoint function shape
//! or discovery rules change.
//!
//! Once ratified, published interface contracts must not be mutated in place.
//! Adding a method, changing method order or lowered types, changing ownership
//! semantics, or changing the canonical IDL fingerprint format requires a new versioned
//! interface ID. Built-in status-code numbers are stable once published and
//! must not be renumbered or reused.
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
//! This facade is `no_std` when the `std` feature is disabled. The separate
//! runtime implementation always requires `std`. Generated
//! interfaces and [`ComPtr`] work without allocation. Generated objects require
//! the `alloc` feature because `#[com_object]` stores each COM object in a
//! reference-counted heap allocation. `Runtime` requires the `runtime`
//! feature, which also enables `std`; `std` enables `alloc`.
//! Defaults enable `std` and `macros`, but do not link a runtime. `static-runtime`
//! implies `runtime` and selects the implementation rlib instead of the shared
//! library. Consequently, `--all-features` uses static runtime linkage.
//!
//! Cross-component declarations belong in `.ncidl`. `neocogi-idl`
//! invokes the macros below with precomputed 32-byte declaration signatures;
//! rustc never hashes a contract. The explicit `assigned_signature` values in
//! these low-level macro examples are test fixtures, not a manual authoring
//! workflow for published contracts.
//!
//! ```
//! use neocogi_com::{com_interface, com_object, ComPtr, IUnknown, NcStatus};
//!
//! #[com_interface(auto_guid = "nc.docs.Calculator:v1", assigned_signature = b"00000000000000000000000000000001")]
//! pub trait ICalculator {
//!     fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus>;
//! }
//!
//! #[com_object(interfaces(ICalculator))]
//! pub struct Calculator;
//!
//! impl ICalculator for Calculator {
//!     fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus> {
//!         Ok(a + b)
//!     }
//! }
//!
//! let unknown: ComPtr<IUnknown> = Calculator::from_value(Calculator);
//! let calculator = unknown.query::<ICalculatorAbi>()?;
//! assert_eq!(calculator.add(2, 3)?, 5);
//! # Ok::<(), NcStatus>(())
//! ```
//!
//! Objects with fields are created with the generated `from_value` constructor:
//!
//! ```
//! use neocogi_com::{com_interface, com_object, ComPtr, IUnknown, NcStatus};
//!
//! #[com_interface(auto_guid = "nc.docs.Counter:v1", assigned_signature = b"00000000000000000000000000000002")]
//! pub trait ICounter {
//!     fn value(&self) -> Result<i32, NcStatus>;
//! }
//!
//! #[com_object(interfaces(ICounter))]
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
//! For low-level macro fixtures, `auto_guid = "stable.abi.Name:v1"` stores the exact
//! qualified-name UTF-8 bytes in the descriptor and hashes them with the
//! `neocogi-com` namespace to produce a deterministic UUIDv5 interface
//! ID. Explicit UUID syntax additionally requires the same `name = "..."` and
//! serves as a compile-time golden assertion of that derivation. Generated
//! interface and factory-plugin descriptors validate their qualified names;
//! plugin release-version literals are validated during macro expansion too.
//! IDL declarations instead use `assigned_guid` from the resolver, whose identity
//! derivation includes the declaration name and binary version. Direct metadata
//! producers can use [`validate_qualified_name`] and [`validate_version_text`] to apply the same
//! allocation-free policy, including in const contexts.
//!
//! ```no_run
//! use neocogi_com::com_interface;
//!
//! #[com_interface(
//!     "bfec7c45-902c-5f94-84ec-77df282eb9f6",
//!     name = "example.Calculator:v1",
//!     assigned_signature = b"00000000000000000000000000000003"
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
//! [`NcCopyFromBufferOffset`] for readable input and [`NcOutputWriter`] for
//! fresh provider output. Generated clients take [`NcOutput`] supplied by
//! [`NcOutputBuffer`] or allocation-free identity slots. Old output values are
//! never input; true transformations take separate input and output arguments.
//! Shared `&self` methods may overlap. Mutable `&mut self` methods require a
//! mutable smart-pointer view and receive exclusive generated object access;
//! conflicting or reentrant calls return [`NC_E_BUSY`].
//! Methods named `query_interface`, `add_ref`, or `release` are reserved for
//! generated `IUnknown` lifecycle thunks. Parameter names remain readable in
//! generated method descriptors and enter the IDL module fingerprint; a focused macro error is emitted only when a name
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
//! New IDL plugins expose [`NC_GET_FACTORY_PLUGIN_DESCRIPTION_SYMBOL`] through
//! [`export_factory_plugin!`]. Their [`NcFactoryPluginDescription`] contains
//! the exact root interface identity/signature and one factory bootstrap,
//! with no class table or erased argument frame. `factory_plugin!` derives
//! root identity and threading from the generated IDL factory associated by
//! `com_factory`. Static hosts call
//! `Runtime::register_factory_plugin_description`; dynamic hosts call
//! `Runtime::load_plugin_library`, which requires the factory-description
//! entrypoint. Hosts obtain the typed root with
//! `Runtime::acquire_plugin_factory` and perform later construction through
//! ordinary typed factory methods.
//!
//! Generated interfaces expose
//! `ABI_SIGNATURE`, typed method descriptors, rendered diagnostic method signature
//! text, and [`NcInterfaceDescriptor`]
//! metadata. IDL interfaces and factories share [`ComInterface`]; factories also
//! implement [`ComFactoryInterface`]. No separate object description is emitted.
//! Object construction is an implementation detail or
//! an ordinary typed factory method.
//! Library loading returns the stable plugin GUID. `Inspection::plugin_info`
//! copies its descriptor identity, release version and first registration origin.
//! `PluginOrigin::Static` represents static registration; dynamic provenance owns
//! the first library path. Native handles and mappings stay private to `nc_com`.
//! There is no separate module identity or module inspection join.
//! Runtime ABI v1 tracks at most [`NC_COM_RUNTIME_MAX_DYNAMIC_PLUGINS`] distinct
//! dynamic providers and permits at most
//! [`NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS`] simultaneous debug messenger
//! registrations. Exceeding the dynamic-provider limit is a fatal live-acquisition error;
//! debug messenger capacity exhaustion returns [`NC_E_OUT_OF_RANGE`].
//! Callers then use signature-checked `QueryInterface` to obtain the interface they
//! need.
//! Factory descriptions may declare hard dependency plugin IDs. Registration
//! requires those IDs to exist already but never discovers paths or loads
//! dependencies recursively.
//! Native loader dependencies remain distinct from neocogi-com dependencies.
//! Description validation checks trusted metadata shape; it is not hostile
//! plugin isolation and cannot make an untrusted shared library safe to load.
//!
//! Hosts can load required bundled plugins during startup with
//! `Runtime::load_plugin_libraries`, which loads paths sequentially in an
//! inspector-produced dependency order. Any load, descriptor, dependency, or
//! registration failure emits its diagnostic and aborts the process.
//! Required cross-plugin contracts should appear in factory methods, ordinary
//! method arguments, return values, or dependency/capability interfaces so the generated
//! interface IDs and ABI signatures describe the actual binary dependency.
//!
//! Loading a dynamic library can run platform loader code that mutates
//! process-global state. `neocogi-com` therefore does not support dynamic plugin
//! unload or hot reload. Successful modules remain mapped for the process
//! lifetime; failed post-open acquisition pins the module before aborting.
//!
//! Fixed-size plain records can use `com_struct`. The macro owns the Rust
//! representation and checked wire conversion, publishes the IDL-assigned nominal
//! identity, and implements [`AbiType`] and [`AbiPlainValue`] for the struct. By default, it
//! verifies that every field implements [`AbiPlainValue`]:
//!
//! ```
//! use neocogi_com::com_struct;
//!
//! #[com_struct(assigned_signature = b"00000000000000000000000000000004")]
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
//! local representation. Application-memory offsets are not published. Generated
//! wire records and checked codecs define foreign storage; direct borrowing
//! requires a separate field-by-field layout proof. Compatibility uses the
//! precomputed IDL contract fingerprint.
//!
//! `#[com_struct(c_struct)]` allows fields without generated COM nominal metadata,
//! but every field still needs a checked [`ComValue`] codec and the
//! [`ComInlineValue`] proof required by copied method slots. For a type owned by
//! another crate, provide these implementations on a local adapter/newtype:
//!
//! ```
//! use neocogi_com::{com_struct, ComInlineValue, ComValue, NcStatus, ValueBudget};
//!
//! /// Local adapter for a C-compatible pair of unrestricted floating-point values.
//! #[repr(C)]
//! #[derive(Clone, Copy)]
//! pub struct Vec2f {
//!     /// Horizontal component.
//!     pub x: f32,
//!     /// Vertical component.
//!     pub y: f32,
//! }
//!
//! // SAFETY: the raw record contains only f32 fields, whose initialized bit
//! // patterns are all valid. It contains no references or ownership duties.
//! unsafe impl ComValue for Vec2f {
//!     /// Application and wire layouts are identical and need no validation.
//!     const BORROW_WIRE: bool = true;
//!     /// Both components already have unrestricted C-compatible wire storage.
//!     type Wire = Self;
//!     /// No indirect backing allocation is needed.
//!     type Storage = ();
//!     /// Charges the record and both scalar fields to the shared work budget.
//!     fn encode_value(&self, budget: &mut ValueBudget) -> Result<(Self, ()), NcStatus> {
//!         budget.value(|budget| {
//!             let (x, ()) = self.x.encode_value(budget)?;
//!             let (y, ()) = self.y.encode_value(budget)?;
//!             Ok((Self { x, y }, ()))
//!         })
//!     }
//!     /// Acquires the copied components under the same recursive budget.
//!     unsafe fn decode_value(wire: Self, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
//!         budget.value(|budget| {
//!             // SAFETY: each initialized f32 bit pattern is a valid scalar wire.
//!             let x = unsafe { f32::decode_value(wire.x, budget)? };
//!             let y = unsafe { f32::decode_value(wire.y, budget)? };
//!             Ok(Self { x, y })
//!         })
//!     }
//! }
//! // SAFETY: the copied wire owns no allocation and borrows no producer storage.
//! unsafe impl ComInlineValue for Vec2f {}
//!
//! #[com_struct(c_struct, assigned_signature = b"00000000000000000000000000000005")]
//! /// A generated record recursively composes the explicit field codec.
//! pub struct Vertex {
//!     /// Geometry location.
//!     pub position: Vec2f,
//!     /// Texture location.
//!     pub uv: Vec2f,
//! }
//! ```
//!
//! The compiler additionally checks nested FFI layouts through
//! `improper_ctypes_definitions`, promoted to an error. That check does not prove
//! wire validity: an adapter containing a boolean must use a raw byte and reject
//! values other than zero and one before constructing the boolean. Such an
//! adapter cannot claim `BORROW_WIRE = true`.
//!
//! The enclosing record's assigned contract signature remains authoritative;
//! external field internals do not acquire independent generated identities.
//! Changing an adapter's raw representation or validation contract requires a
//! corresponding contract revision. The option can be combined with packing or
//! alignment options, but neither layout option bypasses checked conversion.
//!
//! Enum-like ABI scalar values should use transparent integer newtypes generated
//! by [`com_int_enum!`]:
//!
//! ```
//! use neocogi_com::com_int_enum;
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
//! Tagged payload enums use the native Rust enum in fields and generated calls.
//! `com_enum` supplies a separate [`ComValue::Wire`] containing an integer tag and
//! recursively lowered fields. Generated thunks validate the tag and active fields
//! before constructing the native enum; Rust enum layout never crosses COM.
//! Copied fields must implement [`AbiPlainValue`] and [`ComInlineValue`]. Owned
//! fields such as strings use `com_value` and borrowed [`NcValueRef`] parameters.
//!
//! ```
//! mod contract {
//!     use neocogi_com::com_enum;
//!
//!     #[com_enum(tag = u32, assigned_signature = b"00000000000000000000000000000006")]
//!     #[derive(Debug, PartialEq)]
//!     pub enum Mutation {
//!         None,
//!         ChildAdded(u64),
//!         TextChanged { node: u64, len: u32 },
//!     }
//! }
//!
//! use contract::Mutation;
//! use neocogi_com::ComValue;
//!
//! let value = Mutation::TextChanged { node: 7, len: 12 };
//! let (wire, _storage) = value.encode_value(&mut Default::default()).unwrap();
//! assert_eq!(wire.tag, 2);
//! // SAFETY: the encoder produced initialized, self-contained wire storage.
//! let decoded = unsafe { Mutation::decode_value(wire, &mut Default::default()) }.unwrap();
//! assert_eq!(decoded, value);
//! ```

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
#[cfg(feature = "macros")]
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
pub use macros::{ComDeserialize, ComSerialize, com_enum, com_class, com_factory, com_interface, com_object, com_struct, com_value, factory_plugin};
#[cfg(feature = "alloc")]
pub use neocogi_com_core::{format_abi_signature, format_method_signature};
pub use neocogi_com_core::{
    __nc_abi_plain_type, __nc_abi_type, AbiPlainValue, AbiType, ComClientInterface, ComDeserialize, ComSerialize, ComFactoryBootstrap, ComFactoryInterface,
    ComClassInterface, ComFactoryImplementation, ComInterface, ComPtr, ComRaw, ComRef, NcClientPointer, IID_INC_BYTES, IID_INC_STRING, IID_IUNKNOWN,
    INcBytesAbi, INcBytesVTable, INcStringAbi, INcStringVTable, IUNKNOWN_INTERFACE_ID, IUNKNOWN_QUALIFIED_NAME, IUnknown, IUnknownVTable,
    NC_ABI_PASSING_COM_PTR_OUT, NC_ABI_PASSING_COM_BOX_IN, NC_ABI_PASSING_COM_BOX_OUT, NC_ABI_PASSING_WEAK_COM_BOX_OUT, NC_ABI_PASSING_WEAK_COM_PTR_OUT,
    NC_ABI_PASSING_COM_REF, NC_ABI_PASSING_COPY_FROM_BUFFER_OFFSET, NC_ABI_PASSING_COPY_TO_BUFFER_OFFSET, NC_ABI_PASSING_OUTPUT_BUFFER, NC_ABI_PASSING_OUT,
    NC_ABI_PASSING_BORROWED_VALUE, NC_ABI_PASSING_VALUE, NC_ABI_PASSING_VOID, NC_ABI_RECEIVER_MUTABLE, NC_ABI_RECEIVER_SHARED, NC_ABI_THREADING_THREAD_LOCAL,
    NC_ABI_THREADING_THREAD_SAFE, NC_ABI_TYPE_AGGREGATE, NC_ABI_TYPE_BOOL, NC_ABI_TYPE_F32, NC_ABI_TYPE_F64, NC_ABI_TYPE_I8, NC_ABI_TYPE_I16, NC_ABI_TYPE_I32,
    NC_ABI_TYPE_I64, NC_ABI_TYPE_INTERFACE, NC_ABI_TYPE_ISIZE, NC_ABI_TYPE_NC_GUID, NC_ABI_TYPE_NC_STATUS, NC_ABI_TYPE_U8, NC_ABI_TYPE_U16, NC_ABI_TYPE_U32,
    NC_ABI_TYPE_U64, NC_ABI_TYPE_USIZE, NC_ABI_TYPE_VOID, NC_COM_CORE_ABI_VERSION, NC_COM_GUID_NAMESPACE, NC_COM_RUNTIME_ABI_VERSION,
    NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS, NC_COM_RUNTIME_MAX_DYNAMIC_PLUGINS, NC_DEBUG_SEVERITY_ALL, NC_DEBUG_SEVERITY_ERROR, NC_DEBUG_SEVERITY_INFO,
    NC_DEBUG_SEVERITY_WARNING, NC_DEBUG_TYPE_ALL, NC_DEBUG_TYPE_PLUGIN, NC_DEBUG_TYPE_RUNTIME, NC_DEBUG_TYPE_VALIDATION, NC_DESCRIPTOR_QUALIFIED_NAME_MAX_LEN,
    NC_DESCRIPTOR_VERSION_MAX_LEN, NC_E_ABI_VERSION, NC_E_BUSY, NC_E_ALREADY_BOUND, NC_E_CLOSED, NC_E_DROPPED, NC_E_INSUFFICIENT_BUFFER, NC_E_INVALID_ARG,
    NC_E_INVALID_TAG, NC_E_LOAD_FAILED, NC_E_NOINTERFACE, NC_E_NOT_FOUND, NC_E_OUT_OF_MEMORY, NC_E_OUT_OF_RANGE, NC_E_POINTER, NC_FACTORY_PLUGIN_ABI_VERSION,
    NC_GET_FACTORY_PLUGIN_DESCRIPTION_SYMBOL, NC_GUID_NIL, NC_OK, NcAbiSignature, NcAbiPassing, NcAbiPassingDescriptor, NcAbiReceiver, NcAbiThreadingMode,
    NcAbiType, NcAbiTypeDescriptor, NcAcquirePluginFactory, NcDebugApiV1, NcDebugCallbackV1, NcDebugMessageV1, NcDebugMessengerCreateInfoV1,
    NcFactoryPluginDescription, NcGetFactoryPluginDescription, NcGuid, NcInspectFactoryPluginDescriptionCallbackV1, NcInspectionApiV1, NcInterfaceDescriptor,
    NcInterfaceId, NcMethodDescriptor, NcMethodParamDescriptor, NcPersistenceApiV1, NcPersistenceContext, NcPersistenceLimitsV1, NcPersistenceWriteBytesV1,
    NcMethodReturnDescriptor, NcPluginDependencyDescriptor, NcPluginFactoryInfoV1, NcPluginInfoV1, NcPluginRegistryV1, NcQualifiedNameError, NcRuntimeApiV1,
    NcStaticUtf8, NcStatus, NcUtf8View, NcVersionTextError, ParseGuidError, QUALIFIED_NAME_INC_BYTES, QUALIFIED_NAME_INC_STRING, ThreadLocal, ThreadSafe,
    ThreadingModel, acquire_factory_implementation, guid_from_qualified_name, raw, return_factory_plugin_description, validate_factory_plugin_description,
    validate_qualified_name, validate_version_text, validate_rust_path, NcRustPathError, NC_DESCRIPTOR_RUST_PATH_MAX_LEN,
};
pub use object::{
    __ComCallGate, __ComSharedCall, __ComExclusiveCall, __ComExclusiveLease, __ComSharedLease, __com_add_ref_count, __com_release_ref_count, ComAbiObject,
    ComObject, ThreadSafeComObject,
};
mod local;
pub use local::{LocalBox, WeakLocalBox, LocalCom, WeakLocalCom, LocalMut, LocalRef};
#[doc(hidden)]
pub use neocogi_com_core::__ComSupportsInterfaceThreading;
#[cfg(feature = "runtime")]
pub use runtime::{
    Debug, DebugMessenger, Inspection, NC_COM_RUNTIME_MAX_ENUMERATION_ITEMS, NC_COM_RUNTIME_MAX_ENUMERATION_RETRIES, NC_COM_RUNTIME_MAX_INSPECTION_TEXT_BYTES,
    PluginFactoryInfo, PluginInfo, PluginRegistry, PluginOrigin, Runtime,
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
