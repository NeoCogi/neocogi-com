//! Compile-time admission policy for the Rust surface of COM interfaces.
//!
//! Handles lend generated clients through Deref. A handle's own receiver methods
//! take precedence, so public contract names must exclude those operations. IDL
//! and handwritten macro declarations consume this one inventory; private field
//! and parameter names impose no such restriction.

/// Names occupied by supported client-bearing COM handles and their helper traits.
///
/// This is the union for `ComPtr`, `ComRef`, `ComBox` and `WeakComBox`, plus native
/// lifecycle slots. It includes default `Clone::clone_from`; associated functions
/// without a receiver do not shadow client method lookup. New public receiver
/// helpers must extend this inventory. It is build-time policy, not wire metadata.
pub const NC_COM_HANDLE_METHOD_NAMES: &[&str] = &[
    "query_interface",
    "add_ref",
    "release",
    "clone",
    "clone_from",
    "as_raw",
    "into_raw",
    "as_com_ref",
    "query",
    "query_weak",
    "downgrade",
    "downgrade_box",
    "supports_interface_id",
    "supports_interface_bytes",
    "is_idle",
    "is_alive",
    "same_object",
    "try_add_ref",
    "into_unknown",
    "into_thread_local",
    "as_thread_local_ref",
    "fmt",
    "deref",
    "deref_mut",
    "drop",
];

/// Whether an authored interface method collides with a COM handle operation.
///
/// Rust raw identifiers have the same lookup identity as their unprefixed spelling.
/// Validation is case-sensitive, allocation-free, and independent of ABI versions.
/// It applies only to method names; private bindings remain unrestricted.
pub fn is_reserved_com_method(name: &str) -> bool {
    let name = name.strip_prefix("r#").unwrap_or(name);
    NC_COM_HANDLE_METHOD_NAMES.contains(&name)
}
