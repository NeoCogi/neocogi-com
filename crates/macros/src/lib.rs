//! Procedural macros for `neocogi-com`.
//!
//! This crate turns a Rust trait/struct pair into a small COM-like ABI for
//! cross-platform Rust component interop:
//!
//! - [`com_interface`] keeps the input trait as the Rust implementation
//!   contract and generates a C-compatible `*Abi` interface type, `*VTable`,
//!   safe `ComPtr` extension methods, static interface ABI metadata, and the
//!   `unsafe impl neocogi_com::ComInterface` that certifies their raw invariants.
//! - [`com_class`] declares a callable class contract with a typed constructor.
//! - [`com_object`] wraps a Rust implementation struct in generated storage containing
//!   `IUnknown`, one header per exposed interface, an atomic reference count,
//!   and the original Rust value. It also generates object interface metadata
//!   from `interfaces(...)`. Objects are thread-local by default; add
//!   `thread_safe` to generate `ComPtr<_, ThreadSafe>` constructors. Generated
//!   class descriptors record the same mode for checked dynamic construction.
//! - [`com_struct`] marks a fixed-size record as ABI plain data, owns its
//!   `repr(C)`/packing/alignment choice, and records its physical layout. It
//!   checks fields through `neocogi_com::AbiPlainValue` by default, or through
//!   compiler FFI layout checks with the `c_struct` option.
//! - [`com_enum`] keeps a Rust payload enum for ergonomic code and generates an
//!   explicit tag + union ABI representation next to it.
//! - [`plugin`] assembles generated class descriptors into one static plugin
//!   descriptor and exports the descriptor entrypoint for shared libraries.
//!
//! The macros intentionally generate a portable C representation rather than
//! exposing Rust trait objects. The generated ABI methods are
//! `unsafe extern "C"` function pointers, return `neocogi_com::NcStatus`, and
//! write non-void successful return values through an out pointer.
//!
//! The target is Rust-to-Rust interoperability across independently built
//! artifacts: different crate graphs, compiler versions, optimization settings,
//! or linking modes. The generated ABI is C-compatible, but these macros are not
//! C header generation tooling and they are not Windows COM bindings. Generated
//! aggregate hashes are runtime compatibility fingerprints, not reflection
//! metadata from which source declarations can be reconstructed.
//!
//! Owned or long-lived variable-size data such as strings, byte buffers, arrays,
//! streams, and documents should be modeled as COM interfaces. Do not pass Rust
//! `String`, `Vec<T>`, slices, references, raw buffers, or allocator-owned memory
//! across the generated ABI boundary directly. Use `neocogi_com::INcStringAbi` and
//! `neocogi_com::INcBytesAbi` for common read-only strings and byte buffers. Bounded
//! call-scoped buffers use `neocogi_com::NcCopyFromBufferOffset` and
//! `neocogi_com::NcCopyToBufferOffset`. For streaming data, expose bounded methods such as
//! `next_chunk(max_len) -> Result<ComPtr<INcBytesAbi>, NcStatus>`; a successful
//! chunk must contain at most `max_len` bytes, and an empty chunk can represent
//! end-of-stream.
//!
//! # Current ABI lowering
//!
//! This first version supports methods shaped like:
//!
//! ```text
//! fn method(&self, arg: AbiSafeType) -> Result<AbiSafeReturn, neocogi_com::NcStatus>;
//! ```
//!
//! Plain method arguments and non-void successful return payloads must implement
//! `neocogi_com::AbiType`; generated code asserts this at compile time before the
//! values are placed in an `extern "C"` signature. `Result<(), NcStatus>` is
//! the void-success form and lowers to a status-only ABI function with no output
//! pointer. `Result::Err` must contain a negative status. Generated thunks
//! preserve negative errors and normalize a non-negative error to
//! `neocogi_com::NC_E_INVALID_ARG` without writing an output slot. COM interface
//! arguments use `neocogi_com::ComRef<'_, IFooAbi>` and lower
//! to borrowed raw interface pointers. The anonymous `'_` lifetime is required;
//! named lifetimes, including `'static`, are rejected so the borrow cannot escape
//! the generated trait call. COM interface returns use
//! `neocogi_com::ComPtr<IFooAbi>` and lower to owned raw interface pointers. Safe
//! buffer parameters use `neocogi_com::NcCopyFromBufferOffset<'_, T>` or
//! `neocogi_com::NcCopyToBufferOffset<'_, T>` and lower to offset, length, and raw
//! buffer pointer slots. These parameters likewise require `'_` so their safe
//! slices remain call-scoped.
//!
//! The generated ABI function becomes:
//!
//! ```text
//! unsafe extern "C" fn(
//!     this: *mut IFooAbi,
//!     arg: AbiSafeType,
//!     borrowed: *mut IFooAbi,
//!     buffer_offset: u64,
//!     buffer_len: u64,
//!     buffer_ptr: *mut u8,
//!     out: *mut AbiSafeReturn,
//! ) -> neocogi_com::NcStatus
//! ```
//!
//! Void success methods omit the output pointer:
//!
//! ```text
//! unsafe extern "C" fn(
//!     this: *mut IFooAbi,
//!     arg: AbiSafeType,
//! ) -> neocogi_com::NcStatus
//! ```
//!
//! Methods named `query_interface`, `add_ref`, or `release` conflict with the
//! generated `IUnknown` lifecycle thunks and are rejected. Parameter names stay
//! source-derived; the macro reports a focused error only when one would
//! duplicate a lowered buffer slot or shadow a generated adapter binding.
//! Other unsupported method shapes likewise fail with span-aware `syn::Error`
//! diagnostics before any ABI items are generated.
//!
//! # Interface IDs
//!
//! Interfaces can use `#[com_interface(auto_guid = "nc.dom.Node:v1")]` to
//! derive a deterministic UUIDv5 ID from a qualified ABI name. The checked pin
//! form uses `#[com_interface("...", name = "...")]` with the UUID derived
//! from that same name. Complete compile-tested examples live in the primary
//! `neocogi-com` crate-level documentation.
//!
//! The qualified-name string is normative descriptor data and the sole UUIDv5
//! input. Explicit UUID syntax is a checked pin of that same derivation and
//! therefore also requires `name = "..."`. It should not be a Rust module path
//! unless you intend refactors to change the interface ID.
//! Interface, class, and plugin qualified-name literals are validated during
//! macro expansion against the ABI descriptor text policy. Class and plugin
//! version literals are checked at the same time, so generated descriptors
//! cannot defer malformed text failures until runtime activation.
//!
//! Generated code uses the canonical `::neocogi_com` crate path. Use
//! `neocogi_com = path` on a macro invocation only when generated code must target
//! an explicit facade or re-export path. The macros never parse Cargo metadata.

#![deny(missing_docs)]

use proc_macro::TokenStream;
use syn::{DeriveInput, Item, ItemEnum, ItemStruct, ItemTrait, parse_macro_input};

mod abi_method;
mod args;
mod class_contract;
mod deserialize;
mod enums;
mod interface;
mod model;
mod object;
mod plain_data;
mod plugin;
mod structs;
mod util;

use args::{ClassArgs, ClassContractArgs, EnumArgs, InterfaceArgs, StructArgs};
use class_contract::expand_com_class_contract;
use deserialize::expand_derive as expand_com_deserialize_derive;
use enums::expand_com_enum;
use interface::expand_com_interface;
use object::expand_com_object;
use plugin::{PluginArgs, expand_plugin};
use structs::expand_com_struct;

/// Derives format-neutral, statically typed decoding for a local Rust value.
///
/// Named structs decode as records, tuple structs decode as sequences, and
/// fieldless enums decode by variant name. Use
/// `#[com_deserialize(default)]` on a named struct to retain `Default` values
/// for omitted fields, `#[com_deserialize(sequence)]` to decode a named struct
/// positionally, or `#[com_deserialize(rename_all = "snake_case")]` on an enum.
/// COM ABI records should normally use `#[com_struct(deserialize)]` instead so
/// one attribute owns both layout and decoding generation.
#[proc_macro_derive(ComDeserialize, attributes(com_deserialize))]
pub fn derive_com_deserialize(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    match expand_com_deserialize_derive(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Defines a COM interface from a Rust trait.
///
/// The macro keeps the trait as the Rust implementation surface and generates
/// C-compatible ABI items next to it:
///
/// - `ITraitAbi`: a `#[repr(C)]` interface pointer header.
/// - `ITraitVTable`: a `#[repr(C)]` vtable.
/// - `ITraitComPtrExt`: safe call helpers for `neocogi_com::ComPtr<ITraitAbi>`.
/// - An `unsafe impl neocogi_com::ComInterface` for the generated header.
///
/// Generated items inherit the trait visibility, so `pub`, `pub(crate)`, and
/// private interfaces can be used to model public and internal contracts.
/// The generated unsafe implementation certifies the header layout, vtable
/// prefix, interface identity/hash, and `IUnknown` pointer conversion required
/// by safe `ComPtr` operations. Defining an interface through this macro does
/// not require user-written unsafe code.
///
/// `auto_guid = "qualified.Name:v1"` stores that exact qualified name and
/// derives its UUIDv5 identity. Explicit UUID syntax additionally requires
/// `name = "qualified.Name:v1"`; the literal is accepted only when it equals
/// the same derivation. The qualified name must satisfy the ABI descriptor text
/// policy; invalid literals are rejected during macro expansion.
#[proc_macro_attribute]
pub fn com_interface(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as InterfaceArgs);
    let input = parse_macro_input!(item as ItemTrait);
    match expand_com_interface(attr, input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Defines a callable COM class contract on a trait.
///
/// The class declaration owns the primary callable surface, typed constructor,
/// and stable registry identity shared by hosts and implementations. Trait
/// supertraits declare the interface capabilities that every implementation
/// must expose through `QueryInterface`.
///
/// ```text
/// #[com_class(auto_guid = "example.Calculator:v1")]
/// pub trait Calculator {
///     #[constructor]
///     fn new(initial: i32) -> Result<Self, NcStatus>;
///
///     fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus>;
/// }
///
/// #[com_object(Calculator)]
/// pub struct CalculatorObject {
///     value: i32,
/// }
/// ```
///
/// The trait's one `#[constructor]` function is not a vtable method. Instead,
/// the macro generates a typed borrowed frame and an `arguments(...)` builder
/// on `CalculatorAbi`. Hosts activate it with
/// `Runtime::construct::<CalculatorAbi>(arguments)`. No constructor argument
/// interface or one-off primary interface declaration is involved.
///
/// A callable contract's `auto_guid` text supplies both its normative name and
/// stable class identity. Its descriptor version is the declaring crate's
/// package version.
#[proc_macro_attribute]
pub fn com_class(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as Item);
    let input = match item {
        Item::Trait(input) => input,
        Item::Struct(input) => {
            return syn::Error::new_spanned(
                input.ident,
                "com_class declares only callable class traits; use com_object for an implementation struct",
            )
            .to_compile_error()
            .into();
        }
        other => {
            return syn::Error::new_spanned(other, "com_class can only declare a callable class trait")
                .to_compile_error()
                .into();
        }
    };
    let args = parse_macro_input!(attr as ClassContractArgs);
    match expand_com_class_contract(args, input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Defines concrete COM object storage for a Rust implementation struct.
///
/// A bare `Class` path implements one callable class contract without repeating
/// its identity, constructor, or required protocol list. `interfaces(...)`
/// exposes an object made only from shared interface contracts. Object storage
/// is thread-local by default; `thread_safe` requires the stored Rust value to
/// satisfy the corresponding concurrency contract.
///
/// ```text
/// #[com_object(Calculator)]
/// pub struct CalculatorObject {
///     value: i32,
/// }
///
/// #[com_object(interfaces(IEventSink))]
/// struct EventSink {
///     pending: bool,
/// }
/// ```
///
/// The generated allocation owns its reference count, `IUnknown` header,
/// queryable projections, and original Rust value. Only the callable-class form
/// produces a class descriptor that `plugin!` can register.
#[proc_macro_attribute]
pub fn com_object(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ClassArgs);
    let input = parse_macro_input!(item as ItemStruct);
    expand_com_object(args, input).into()
}

/// Defines static plugin metadata from generated class descriptors.
///
/// `plugin!` is intentionally a small assembly macro: classes own their class
/// IDs, interface tables, and constructors through `#[com_object(...)]`; this
/// macro copies those descriptors into a plugin-level class table and provides
/// `pub fn plugin_descriptor() -> &'static NcPluginDescriptor` for in-process
/// hosts. A final shared library calls `neocogi_com::export_plugin!` separately to
/// export `nc_get_plugin_descriptor`; static assemblies omit that export.
///
/// core ABI v5 plugin descriptors contain declarative metadata only. The optional
/// `dependencies = [PLUGIN_ID, ...]` list declares hard dependencies that must
/// already be registered. It never causes recursive loading.
/// Every `plugin_id(...)` form is checked against the plugin's normative
/// `name`; constant paths should be defined with
/// `neocogi_com::guid_from_qualified_name`. Plugin `name` and `version` literals are
/// validated during macro expansion.
#[proc_macro]
pub fn plugin(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as PluginArgs);
    expand_plugin(args).into()
}

/// Defines a fixed-size ABI-safe record.
///
/// The macro owns the struct representation, derives `Clone, Copy` when those
/// derives are not already listed, records the physical size/alignment/field
/// offsets in `neocogi_com::ComStruct`, and implements `neocogi_com::AbiType` and
/// `neocogi_com::AbiPlainValue` for the struct. By default, it verifies every field
/// implements `neocogi_com::AbiPlainValue`.
///
/// Accepted layout forms are `#[com_struct]`, `#[com_struct(packed)]`,
/// `#[com_struct(packed(N))]`, and `#[com_struct(align(N))]`. Do not also write
/// a raw `#[repr(...)]` attribute; the macro rejects it so the representation and
/// generated layout metadata have one source of truth.
///
/// `#[com_struct(c_struct)]` accepts external C-layout field types without COM
/// traits or macro annotations. The option can be combined with packing and
/// alignment options. The compiler checks the record's FFI layout, including
/// nested field layouts, through `improper_ctypes_definitions`, promoted to an
/// error. An incompatible Rust-layout field fails to compile; `Copy` and syntactic
/// field checks still apply. The author must ensure fields contain no hidden
/// pointers, references, or ownership duties, which C layout alone cannot prove.
/// In this mode, every field is fingerprinted by size and alignment, with field
/// order and offsets recorded by the enclosing struct. Field internals and
/// scalar type identities are not hashed. An incompatible change preserving
/// this metadata requires a new interface identity. Dependency versions may
/// differ only while their actual layouts and value semantics remain compatible.
/// The checks add no runtime conversion or allocation.
///
/// Use this for small plain records. Variable-size data, borrowed data, raw
/// addresses, and values with ownership or release obligations should be modeled
/// as COM interfaces instead.
///
/// Add `deserialize` to generate a format-neutral [`ComDeserialize`] constructor.
/// Named records reject missing fields by default; combine it with `default` to
/// retain `Default` values for omitted fields, or with `sequence` to decode the
/// fields positionally. These options affect local source decoding only and do
/// not change the COM ABI layout.
#[proc_macro_attribute]
pub fn com_struct(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as StructArgs);
    let input = parse_macro_input!(item as ItemStruct);
    match expand_com_struct(args, input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Defines a tagged payload enum with an explicit COM ABI representation.
///
/// The input Rust enum remains available for local Rust code. Apply
/// `#[com_enum(tag = u32)]` to an ordinary payload enum; complete
/// compile-tested construction, conversion, and tag-pinning examples live in
/// the primary `neocogi-com` crate-level documentation.
///
/// The macro generates `MutationAbi`, `MutationTag`, `MutationPayload`, and one
/// payload struct per data-carrying variant. `MutationAbi` has a `#[repr(C)]`
/// tag + union binary shape, but its fields are opaque so safe Rust cannot pair
/// a known tag with the wrong or uninitialized union member. Convert known
/// variants with `MutationAbi::from(value)` and `Mutation::try_from(abi)`. Use
/// `MutationAbi::tag()` to inspect the tag and `from_unknown_tag` to construct an
/// unknown value that converts to `NC_E_INVALID_TAG`.
///
/// Low-level FFI adapters can use `unsafe from_raw_parts_unchecked`. For a known
/// tag, the caller must provide the corresponding initialized union member and
/// every selected field must contain a valid Rust value. `TryFrom` safely relies
/// on that construction invariant; it rejects unknown tags but is not a parser
/// for arbitrary or uninitialized bytes.
///
/// Tags default to one-based source order. Use `#[com_tag(value)]` on a variant
/// to pin an explicit ABI value, for example `#[com_tag(10)]`.
///
/// Caveat: reordering unpinned variants changes their ABI tag. Public ABI
/// enums should pin every variant with `#[com_tag(...)]`.
///
/// Variant payload fields must implement `neocogi_com::AbiPlainValue`. Owning Rust values
/// such as `String`, `Vec<T>`, and `neocogi_com::ComPtr<T>` are rejected because the
/// generated ABI representation has no drop or release hook. Prefer explicit
/// scalar handles for references to external objects. Raw pointers are rejected
/// in payloads because they carry lifetime, nullability, and ownership rules not
/// represented by a durable tagged value.
///
/// Add `deserialize` to generate a format-neutral constructor for the original
/// Rust enum. Fieldless, newtype, tuple, and named variants remain statically
/// typed throughout decoding; this option does not alter the generated ABI.
#[proc_macro_attribute]
pub fn com_enum(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as EnumArgs);
    let input = parse_macro_input!(item as ItemEnum);
    match expand_com_enum(args, input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
