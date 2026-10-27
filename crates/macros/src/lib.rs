//! Implementation and persistence procedures for NeoCogi COM.
//!
//! IDL compiles callable and value contracts to complete Rust through the typed
//! backend. This crate adds concrete object allocation, typed plugin acquisition
//! and standalone positional NCB codecs to ordinary Rust application types.

use proc_macro::TokenStream;
use syn::{DeriveInput, ItemStruct, parse_macro_input};

/// Derives statically typed NCB decoding for a local Rust value.
///
/// Struct fields decode in declaration order; enums use declaration ordinals
/// and recursively typed payloads. Use `#[com_deserialize(default)]` on a named
/// struct to retain `Default` values for an omitted suffix. Truncated field bytes
/// remain errors. NCB has no alternate named-field or renamed-variant mode.
/// IDL records receive their codecs directly from the resolved declaration.
/// These derives apply to ordinary Rust persistence values.
#[proc_macro_derive(ComDeserialize, attributes(com_deserialize))]
pub fn derive_com_deserialize(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    match neocogi_idl::codegen::expand_com_deserialize_derive(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derives field-by-field serialization for owned initialization records and enums.
/// Uses the same counted fields and declaration ordinals as the NCB decoder.
#[proc_macro_derive(ComSerialize, attributes(com_deserialize))]
pub fn derive_com_serialize(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    match neocogi_idl::codegen::expand_com_serialize_derive(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
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
/// #[com_object(interfaces(IProcessor))]
/// struct Processor {
///     pending: bool,
/// }
/// ```
///
/// The generated allocation owns its reference count, shared borrow guard,
/// and original Rust value. Queryable projections use static tables, not fields. Construction remains local
/// to the owning implementation or is exposed through an IDL factory method.
/// `ownership = shared` (the default) emits `from_value` and local shared helpers.
/// `ownership = exclusive` emits `into_box` and local exclusive helpers instead.
/// This provider-local choice never changes an interface's C ABI or threading.
#[proc_macro_attribute]
pub fn com_object(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as neocogi_idl::codegen::ObjectArgs);
    let input = parse_macro_input!(item as ItemStruct);
    neocogi_idl::codegen::expand_com_object(args, input).into()
}

/// Describes one plugin using a safe typed acquisition function.
/// The function takes one supplied `Runtime` argument and explicitly returns
/// `Result<ComPtr<InterfaceAbi>, NcStatus>`. Its return interface supplies the
/// root identity, ABI signature and threading. `name` derives the plugin ID;
/// optional `version` defaults to the Cargo package version. Explicit
/// `dependencies = [PLUGIN_ID, ...]` are hard registration dependencies.
///
/// Generates `plugin_descriptor()` for static registration and, under the
/// provider's `dynamic-plugin` feature, `nc_get_plugin_descriptor_v1` for a DLL.
/// The provider owns initialized storage; acquisition transfers one owner and
/// may return a retained instance. No singleton or object-construction policy
/// is imposed on ordinary interface methods.
#[proc_macro_attribute]
pub fn plugin(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as neocogi_idl::codegen::PluginArgs);
    let input = parse_macro_input!(item as syn::ItemFn);
    match neocogi_idl::codegen::expand_plugin(args, input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Initializes the selected host implementation and supplies `Runtime` to main.
///
/// Apply to a safe synchronous `fn main(runtime: Runtime)` with the ordinary
/// Rust return type (`()`, `ExitCode` or a supported `Result`). The generated
/// entry calls the explicit `neocogi_com_host` dependency; facade features never select
/// that implementation. Services survive main's return for scheduled callbacks.
/// Reusable libraries and plugins receive access rather than using this macro.
/// `neocogi_com = path` and `neocogi_com_host = path` support renamed dependencies.
#[proc_macro_attribute]
pub fn com_main(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as neocogi_idl::codegen::MainArgs);
    let input = parse_macro_input!(item as syn::ItemFn);
    match neocogi_idl::codegen::expand_com_main(args, input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
