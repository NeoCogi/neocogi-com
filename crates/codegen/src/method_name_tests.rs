//! Public member admission must track real handle APIs, not private local names.

use neocogi_com_core::NC_COM_HANDLE_METHOD_NAMES;
use quote::quote;

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
