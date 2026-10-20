//! Reproducible built-in persistence bindings without a core/build-tool cycle.
//!
//! Core cannot depend on its own IDL compiler or procedural macros. This test
//! runs the ordinary IDL and interface generators and checks both committed
//! halves. Set `NC_UPDATE_PERSISTENCE_BINDINGS=1` to deliberately regenerate them.

use std::path::Path;

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
    let generated = neocogi_idl::generate::generate_bootstrap(&graph, "neocogi.com.persistence").unwrap();
    // Core and facade consume the same typed plan as ordinary compiled peers.
    // Only the canonical support path changes at the explicit bootstrap boundary.
    let abi = generated
        .abi
        .parse::<proc_macro2::TokenStream>()
        .unwrap()
        .to_string()
        .replace(":: neocogi_com", "crate");
    let provider = generated.provider;
    check_generated(&consumer.core.join("src/persistence/interfaces.rs"), &abi);
    check_generated(&consumer.facade.join("src/persistence/providers.rs"), &format!("use super::*; {}", provider));
}

/// Formats one generated half and either verifies it or explicitly refreshes it.
fn check_generated(path: &Path, tokens: &str) {
    let file = syn::parse_file(tokens).unwrap();
    let expected = format!(
        "// Generated from core/idl/neocogi/com/persistence/objects.ncidl.\n\
         // Regenerate: NC_UPDATE_PERSISTENCE_BINDINGS=1 cargo test -p neocogi-com-codegen persistence_bindings_match_idl\n\n{}",
        prettyplease::unparse(&file),
    );
    if std::env::var_os("NC_UPDATE_PERSISTENCE_BINDINGS").as_deref() == Some(std::ffi::OsStr::new("1")) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, &expected).unwrap();
    }
    assert_eq!(std::fs::read_to_string(path).unwrap(), expected, "stale bindings: {}", path.display());
}
