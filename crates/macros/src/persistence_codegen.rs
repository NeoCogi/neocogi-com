//! Reproducible built-in persistence bindings without a core/build-tool cycle.
//!
//! Core cannot depend on its own IDL compiler or procedural macros. This test
//! runs the ordinary IDL and interface generators and checks both committed
//! halves. Set `NC_UPDATE_PERSISTENCE_BINDINGS=1` to deliberately regenerate them.

use std::path::Path;

use quote::quote;
use syn::Item;

use crate::{abi_method::parse_abi_trait_item, args::InterfaceArgs, interface::expand_interface_parts};

/// Checks that the built-in contract retains exactly the original IDL identity.
#[test]
fn persistence_bindings_match_idl() {
    let consumer = crate::test_consumer::Consumer::new("persistence-binding-proof");
    let source = std::fs::read_to_string(consumer.core.join("idl/neocogi/com/persistence/objects.ncidl")).unwrap();
    let module = neocogi_idl::parse_module_parts(
        Path::new("neocogi/com/persistence"),
        &[neocogi_idl::ModuleSource {
            path: Path::new("neocogi/com/persistence/objects.ncidl"),
            text: &source,
        }],
    )
    .unwrap();
    let graph = neocogi_idl::resolve_modules(vec![module]).unwrap();
    let generated = neocogi_idl::generate_rust(&graph, "neocogi.com.persistence").unwrap();
    let mut abi = proc_macro2::TokenStream::new();
    let mut provider = proc_macro2::TokenStream::new();
    for item in syn::parse_file(&generated).unwrap().items {
        match item {
            Item::Trait(mut definition) => {
                // Identity and signature still come from the IDL compiler; the
                // split only changes which crate owns the emitted Rust items.
                let index = definition
                    .attrs
                    .iter()
                    .position(|attr| attr.path().segments.last().unwrap().ident == "com_interface")
                    .unwrap();
                let args: InterfaceArgs = definition.attrs.remove(index).parse_args().unwrap();
                let nc = syn::parse_quote!(::neocogi_com);
                let methods = definition.items.iter().map(|item| parse_abi_trait_item(item, &nc).unwrap()).collect();
                let parts = expand_interface_parts(args, definition, methods).unwrap();
                abi.extend(parts.abi);
                provider.extend(parts.provider);
            }
            // Authoring descriptions depend on the facade's design module.
            // Core persistence exposes its physical ABI only; retaining these
            // convenience constants here would create a core -> facade cycle.
            Item::Impl(definition)
                if definition
                    .items
                    .iter()
                    .any(|item| matches!(item, syn::ImplItem::Const(value) if value.ident == "IDL_INTERFACE")) => {}
            // Module identity/fingerprint constants belong to the contract.
            item => abi.extend(quote! { #item }),
        }
    }
    // The ABI half has no facade-owned object support. Resolving its paths to
    // core makes that separation a compiler-checked property, even without alloc.
    let abi = abi.to_string().replace(":: neocogi_com", "crate");
    check_generated(&consumer.core.join("src/persistence/interfaces.rs"), &abi);
    check_generated(&consumer.facade.join("src/persistence/providers.rs"), &format!("use super::*; {}", provider));
}

/// Formats one generated half and either verifies it or explicitly refreshes it.
fn check_generated(path: &Path, tokens: &str) {
    let file = syn::parse_file(tokens).unwrap();
    let expected = format!(
        "// Generated from core/idl/neocogi/com/persistence/objects.ncidl.\n\
         // Regenerate: NC_UPDATE_PERSISTENCE_BINDINGS=1 cargo test -p neocogi-com-macros persistence_bindings_match_idl\n\n{}",
        prettyplease::unparse(&file),
    );
    if std::env::var_os("NC_UPDATE_PERSISTENCE_BINDINGS").as_deref() == Some(std::ffi::OsStr::new("1")) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, &expected).unwrap();
    }
    assert_eq!(std::fs::read_to_string(path).unwrap(), expected, "stale bindings: {}", path.display());
}
