//! Procedural macros for `neocogi-com`.
//!
//! This crate turns a Rust trait/struct pair into a small COM-like ABI for
//! cross-platform Rust component interop:
//!
//! - [`com_interface`] keeps the input trait as the Rust implementation
//!   contract and generates a C-compatible `*Abi` interface type, `*VTable`,
//!   one safe `*Client` view lent by both `ComPtr` and `ComRef`, static interface
//!   ABI metadata, and the `unsafe impl neocogi_com::ComInterface` that certifies their raw invariants.
//! - [`com_object`] wraps a Rust implementation struct in generated storage containing
//!   `IUnknown`, one header per exposed interface, an atomic reference count,
//!   and the original Rust value. It derives public projections from an IDL
//!   interface or an explicit local `interfaces(...)` list. Objects are
//!   thread-local by default; add `thread_safe` to generate
//!   `ComPtr<_, ThreadSafe>` values for local objects.
//! - [`com_factory`] associates an implementation with one generated IDL
//!   factory interface.
//! - [`factory_plugin`] creates immutable plugin metadata for one typed root
//!   factory; it never generates an object table or erased argument frame.
//! - [`com_struct`] marks a fixed-size record as ABI plain data, owns its
//!   `repr(C)`/packing/alignment choice, and records its physical layout. It
//!   checks fields through `neocogi_com::AbiPlainValue` by default, or through
//!   compiler FFI layout checks with the `c_struct` option.
//! - [`com_enum`] keeps a Rust payload enum for ergonomic code and generates an
//!   explicit tag + union ABI representation next to it.
//!
//! The macros intentionally generate a portable C representation rather than
//! exposing Rust trait objects. The generated ABI methods are
//! `unsafe extern "C"` function pointers, return `neocogi_com::NcStatus`, and
//! write non-void successful return values through an out pointer.
//!
//! The target is Rust-to-Rust interoperability across independently built
//! artifacts: different crate graphs, compiler versions, optimization settings,
//! or linking modes. The generated ABI is C-compatible, but these macros are not
//! C header generation tooling and they are not Windows COM bindings. The IDL
//! build tool supplies each macro expansion with an already-computed,
//! declaration-scoped signature. Macro expansion emits typed metadata and fixed
//! bytes; it does not serialize or hash a contract in rustc.
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
//! Callable methods use shared or mutable receivers and typed results, for example:
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
//! `neocogi-idl` supplies the deterministic interface GUID, qualified
//! name, and `assigned_signature` byte string. Direct macro invocations are
//! low-level generator/test infrastructure and must also provide a complete
//! precomputed signature; there is intentionally no compiler-side fallback.
//!
//! The generator supplies `assigned_guid` after resolving the IDL declaration
//! name and version, including an explicit ID when authored. Low-level
//! `auto_guid` instead derives a UUIDv5 directly from its qualified-name text;
//! a positional UUID literal is a checked pin of that derivation and requires
//! `name = "..."`. These are distinct identity paths. Interface/plugin names
//! and plugin release-version literals are validated against the descriptor
//! text policy during macro expansion.
//!
//! Generated code uses the canonical `::neocogi_com` crate path. Use
//! `neocogi_com = path` on a macro invocation only when generated code must target
//! an explicit facade or re-export path. The macros never parse Cargo metadata.

#![deny(missing_docs)]

use proc_macro::TokenStream;
use syn::{DeriveInput, ItemEnum, ItemStruct, ItemTrait, parse_macro_input};

mod abi_method;
mod args;
mod class;
mod deserialize;
mod enums;
mod factory_plugin;
mod interface;
mod model;
mod object;
mod plain_data;
mod serialize;
mod structs;
mod value;
mod util;

use args::{ObjectArgs, EnumArgs, InterfaceArgs, StructArgs};
use deserialize::expand_derive as expand_com_deserialize_derive;
use enums::expand_com_enum;
use factory_plugin::{FactoryPluginArgs, expand_factory_plugin};
use interface::expand_com_interface;
use object::expand_com_object;
use structs::expand_com_struct;

/// Derives format-neutral, statically typed decoding for a local Rust value.
///
/// Named structs decode as records, tuple structs decode as sequences, and
/// enums decode by variant name and typed payload. Use
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

/// Derives field-by-field serialization for owned initialization records and enums.
/// Uses the same com_deserialize naming/sequence policy as the matching decoder.
#[proc_macro_derive(ComSerialize, attributes(com_deserialize))]
pub fn derive_com_serialize(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    match serialize::expand_derive(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Defines a COM interface from an IDL-generated Rust trait.
///
/// The macro keeps the trait as the Rust implementation surface and generates
/// C-compatible ABI items next to it:
///
/// - `ITraitAbi`: a `#[repr(C)]` interface pointer header.
/// - `ITraitVTable`: a `#[repr(C)]` vtable.
/// - `ITraitClient`: the safe method view lent by owning and borrowed pointers.
/// - An `unsafe impl neocogi_com::ComInterface` for the generated header.
///
/// Generated items inherit the trait visibility.
/// The generated unsafe implementation certifies the header layout, vtable
/// prefix, interface identity/signature, and `IUnknown` pointer conversion required
/// by safe `ComPtr` operations. Defining an interface through this macro does
/// not require user-written unsafe code.
///
/// The required `assigned_signature = b"..."` is the 32-byte declaration
/// signature computed by `neocogi-idl`. `auto_guid =
/// "qualified.Name:v1"` stores that exact qualified name and derives its UUIDv5
/// identity. The qualified name must satisfy the ABI descriptor text policy;
/// invalid literals are rejected during macro expansion.
#[proc_macro_attribute]
pub fn com_interface(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as InterfaceArgs);
    let input = parse_macro_input!(item as ItemTrait);
    match expand_com_interface(attr, input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Defines concrete COM object storage for a Rust implementation struct.
///
/// A bare `Interface` path implements one IDL interface without
/// repeating its identity, threading, or required protocol list.
/// `interfaces(...)` exposes a local
/// object made only from shared interface contracts. Local object storage is
/// thread-local by default; an IDL interface owns its threading capability.
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
/// queryable projections, and original Rust value. Construction remains local
/// to the owning implementation or is exposed through an IDL factory method.
#[proc_macro_attribute]
pub fn com_object(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ObjectArgs);
    let input = parse_macro_input!(item as ItemStruct);
    expand_com_object(args, input, false).into()
}

/// Defines concrete COM storage for an implementation of an IDL factory.
///
/// The factory interface owns its public identity, methods, threading, and
/// required interfaces. This macro owns only private Rust storage and COM
/// lifetime mechanics; it does not publish a globally callable constructor.
#[proc_macro_attribute]
pub fn com_factory(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ObjectArgs);
    let input = parse_macro_input!(item as ItemStruct);
    if args.primary_interface.is_none() {
        return syn::Error::new_spanned(input.ident, "com_factory requires one IDL factory interface path")
            .to_compile_error()
            .into();
    }
    expand_com_object(args, input, true).into()
}

/// Defines one factory-only plugin from exact IDL modules and a typed root.
///
/// The plugin ID is derived from `name` unless `plugin_id(...)` supplies a
/// checked pin. The root ID and threading capability always come from the IDL
/// interface associated with `factory`; they cannot be overridden here.
#[proc_macro]
pub fn factory_plugin(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as FactoryPluginArgs);
    expand_factory_plugin(args).into()
}

/// Defines a fixed-size ABI-safe record from generated IDL bindings.
///
/// The macro owns the struct representation, derives `Clone, Copy` when those
/// derives are not already listed, records the physical size/alignment/field
/// offsets in `neocogi_com::ComStruct`, and implements `neocogi_com::AbiType` and
/// `neocogi_com::AbiPlainValue` for the struct. By default, it verifies every field
/// implements `neocogi_com::AbiPlainValue`.
///
/// The IDL generator always supplies `assigned_signature = b"..."`; its 32
/// bytes are stored directly and never hashed by rustc. Accepted layout forms are `#[com_struct]`, `#[com_struct(packed)]`,
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
/// In this mode, every field is represented by size and alignment, with field
/// order and offsets recorded by the enclosing struct. Field internals and
/// scalar type identities are not encoded. An incompatible change preserving
/// this metadata requires a new interface identity. Dependency versions may
/// differ only while their actual layouts and value semantics remain compatible.
/// External fields must implement `ComValue` and `ComInlineValue`, explicitly
/// defining raw representations and validation. C layout alone never bypasses
/// checked conversion. Generated method slots use the converted wire type.
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

/// Defines a generated tagged payload enum with an explicit COM ABI representation.
///
/// The input Rust enum remains available for local Rust code. The IDL generator
/// supplies both `tag` and the precomputed `assigned_signature`; complete
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

#[cfg(test)]
mod persistence_codegen;

#[cfg(test)]
mod navigation_tests;

/// Implements a concrete IDL class and optionally delegates its single event stream.
///
/// `#[com_class(Textbox, event = events)]` generates ordinary COM storage and
/// forwards the class stream's `attach` method to the named `TextboxEventPort`
/// field. Initialize that field with `Default::default()`, emit with `post`,
/// and retire it with `close`. The field remains ordinary exclusively owned
/// Rust state, visible in every constructor; the macro adds no hidden queue.
#[proc_macro_attribute]
pub fn com_class(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as class::ClassArgs);
    let input = parse_macro_input!(item as ItemStruct);
    match class::expand(args, input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Generates recursive raw fields and typed backing storage for an owned IDL value.
/// This uses the same field codec as copied structs and enum payloads.
#[proc_macro_attribute]
pub fn com_value(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as StructArgs);
    let input = parse_macro_input!(item as syn::DeriveInput);
    match value::expand_owned(args, input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
