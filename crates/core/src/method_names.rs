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
/// helpers must extend this inventory. Standard-prelude blanket methods are part
/// of this receiver surface even though their impls live outside this crate.
/// Arbitrarily imported extension traits can require qualified client calls.
/// Recheck the prelude surface on compiler/edition changes. This is build-time
/// policy, not wire metadata.
pub const NC_COM_HANDLE_METHOD_NAMES: &[&str] = &[
    "query_interface",
    "add_ref",
    "release",
    "clone",
    "clone_from",
    // Standard-prelude blanket operations also precede the Deref client.
    "into",       // core::convert::Into
    "try_into",   // core::convert::TryInto
    "to_owned",   // alloc::borrow::ToOwned
    "clone_into", // alloc::borrow::ToOwned
    "as_raw",
    "into_raw",
    "as_com_ref",
    "query",
    "query_ref",
    "query_weak",
    "downgrade",
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

#[cfg(test)]
mod tests {
    //! Source audits stay inside the independently packaged handle owner.
    use super::is_reserved_com_method;
    use std::string::ToString;

    /// Audits actual handle impls, including trait methods, so adding a helper cannot
    /// silently leave the admission inventory behind. Blanket prelude impls are
    /// checked by executable compiler consumers, not discovered from local source.
    /// Generic impls are parsed in
    /// all source configurations; no runtime feature selection hides a public method.
    #[test]
    fn every_public_handle_receiver_is_reserved() {
        let mut audited = 0;
        for source in [include_str!("ptr.rs"), include_str!("boxed.rs")] {
            for item in syn::parse_file(source).unwrap().items {
                let syn::Item::Impl(item) = item else { continue };
                let syn::Type::Path(ty) = &*item.self_ty else { continue };
                let name = ty.path.segments.last().unwrap().ident.to_string();
                if !matches!(name.as_str(), "ComPtr" | "ComRef" | "ComBox" | "WeakComBox") {
                    continue;
                }
                for member in item.items {
                    let syn::ImplItem::Fn(method) = member else { continue };
                    if method.sig.receiver().is_none() {
                        continue;
                    }
                    if item.trait_.is_none() && !matches!(method.vis, syn::Visibility::Public(_)) {
                        continue;
                    }
                    let name = method.sig.ident.to_string();
                    assert!(is_reserved_com_method(&name), "public {name} must enter the shared inventory");
                    audited += 1;
                }
            }
        }
        assert!(audited > 30, "must inspect the actual owned/borrowed/exclusive handle implementations");
        // These default/helper trait methods are not all explicit impl items above.
        for name in ["clone_from", "deref", "deref_mut"] {
            assert!(is_reserved_com_method(name));
        }
    }
}
