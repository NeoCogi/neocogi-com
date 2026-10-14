//! Public member admission must track real handle APIs, not private local names.

use neocogi_com_core::{NC_COM_HANDLE_METHOD_NAMES, is_reserved_com_method};
use quote::quote;

/// Audits actual handle impls, including trait methods, so adding a helper cannot
/// silently leave the admission inventory behind. Generic impls are parsed in
/// all source configurations; no runtime feature selection hides a public method.
#[test]
fn every_public_handle_receiver_is_reserved() {
    let mut audited = 0;
    for source in [include_str!("../../core/src/ptr.rs"), include_str!("../../core/src/boxed.rs")] {
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

/// Both ordinary and raw Rust spellings reject through the real macro parser.
#[test]
fn all_handle_operations_reject_at_macro_admission() {
    for name in NC_COM_HANDLE_METHOD_NAMES {
        for spelling in [name.to_string(), format!("r#{name}")] {
            let input: syn::ItemTrait = syn::parse_str(&format!("pub trait ICollision {{ fn {spelling}(&self) -> Result<u32, NcStatus>; }}")).unwrap();
            let args = syn::parse2(quote! {
                auto_guid = "nc.tests.handle_collision:v1",
                assigned_signature = b"00000000000000000000000000000001"
            })
            .unwrap();
            let error = crate::interface::expand_com_interface(args, input).unwrap_err().to_string();
            assert!(error.contains("conflicts with a COM handle operation"), "{spelling}: {error}");
        }
    }
}
