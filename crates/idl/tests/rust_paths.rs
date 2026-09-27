//! Rust package placement must not change logical contracts or wire signatures.

use std::path::Path;

use neocogi_idl::{parse_module, resolve_modules};
use neocogi_idl::generate::{RustModulePaths, generate_rust_with_paths};

/// Relocated contracts keep signatures while all imported type families move together.
#[test]
fn rust_locations_only_change_import_paths() {
    let owner = parse_module(
        Path::new("example/contract.ncidl"),
        r#"
        #[version(1)] struct Value { number: u32 }
        #[version(1)] interface IBase { fn value(&self) -> Result<Value>; }
        #[version(1)] struct Changed { value: u32 }
    "#,
    )
    .unwrap();
    let consumer = parse_module(
        Path::new("example/consumer.ncidl"),
        r#"
        extern example.contract;
        use example.contract::{Value, IBase, Changed};
        #[version(1)] interface IConsumer: IBase {
            fn changed(&mut self, value: &Changed) -> Result<()>;
            fn set(&mut self, value: Value, source: &IBase) -> Result<()>;
        }
    "#,
    )
    .unwrap();
    let graph = resolve_modules(vec![owner, consumer]).unwrap();
    let baseline = neocogi_idl::generate_rust(&graph, "example.consumer").unwrap();
    let mut paths = RustModulePaths::default();
    paths.insert("example.contract", "shared_api::contracts").unwrap();
    let relocated = generate_rust_with_paths(&graph, "example.consumer", &paths).unwrap();
    // Token comparison ignores wrapping introduced by longer paths; every other
    // emitted token, including assigned identities and signatures, must agree.
    let normalize = |text: &str| text.parse::<proc_macro2::TokenStream>().unwrap().to_string();
    assert_eq!(
        normalize(&baseline.replace("example_contract_contracts", "shared_api::contracts")),
        normalize(&relocated)
    );
    assert!(relocated.contains("shared_api::contracts::Changed"));
}

/// Invalid setup never creates ambiguous, relative, or generic import locations.
#[test]
fn rust_locations_reject_invalid_and_conflicting_owners() {
    let mut paths = RustModulePaths::default();
    for path in ["", "crate::events", "self::events", "super::events", "api::Thing<u32>"] {
        assert!(paths.insert("example.events", path).is_err(), "{path}");
    }
    paths.insert("example.events", "api::events").unwrap();
    paths.insert("example.events", "::api::events").unwrap();
    assert!(paths.insert("example.events", "other_api::events").is_err());
    let graph = resolve_modules(vec![
        parse_module(Path::new("example/value.ncidl"), "#[version(1)] struct Value { value: u32 }").unwrap(),
    ])
    .unwrap();
    assert!(generate_rust_with_paths(&graph, "example.value", &paths).is_err());
}
