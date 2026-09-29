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
    for path in [
        "",
        "crate::events",
        "self::events",
        "super::events",
        "api::Thing<u32>",
        "api::type",
        "api::gen",
        "api ::events",
        "api::Évent",
        "api::::Event",
    ] {
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

/// The final transported spelling includes the declaration, not just its module prefix.
#[test]
fn public_metadata_paths_charge_the_complete_spelling() {
    let graph = resolve_modules(vec![
        parse_module(Path::new("example/value.ncidl"), "#[version(1)] struct Value { value: u32 }").unwrap(),
    ])
    .unwrap();
    let maximum = neocogi_com_core::NC_DESCRIPTOR_RUST_PATH_MAX_LEN;
    let mut paths = RustModulePaths::default();
    paths.insert("example.value", &"a".repeat(maximum - "::Value".len())).unwrap();
    generate_rust_with_paths(&graph, "example.value", &paths).unwrap();
    let mut oversized = RustModulePaths::default();
    oversized.insert("example.value", &"a".repeat(maximum)).unwrap();
    assert!(
        generate_rust_with_paths(&graph, "example.value", &oversized)
            .unwrap_err()
            .to_string()
            .contains("invalid public Rust path")
    );
}

/// Builds an IDL owner, discovers its COM catalog, then compiles a downstream form.
/// The fixture lives in a temporary workspace so its raw module and Cargo dependency
/// names are real Rust resolution boundaries, independent of test-crate imports.
#[test]
fn raw_identifier_catalog_paths_compile_in_exported_forms() {
    use std::{fs, process::Command};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    let directory = project.path();
    fs::create_dir_all(directory.join("contracts/src/bin")).unwrap();
    fs::create_dir_all(directory.join("contracts/idl/example")).unwrap();
    fs::create_dir_all(directory.join("app/src")).unwrap();
    fs::write(directory.join("Cargo.toml"), "[workspace]\nmembers=['contracts','app']\nresolver='3'\n").unwrap();
    let dependencies = format!(
        "[dependencies]\nneocogi-com={{path={:?},features=['static-runtime']}}\nneocogi-ui={{path={:?}}}\nneocogi-ui-document={{path={:?}}}\n",
        root.join("crates/com/api"),
        root.join("crates/ui/api"),
        root.join("crates/ui/document"),
    );
    fs::write(directory.join("contracts/Cargo.toml"), format!(
        "[package]\nname='path-contracts'\nversion='0.0.0'\nedition='2024'\nlinks='nc_contract_example_paths'\n{dependencies}neocogi-ui-core-contracts={{path={:?}}}\nserde_json='1'\n[build-dependencies]\nneocogi-idl={{path={:?}}}\n",
        root.join("crates/ui/core-contracts"), root.join("crates/com/idl"),
    )).unwrap();
    fs::write(
        directory.join("contracts/build.rs"),
        r#"
        /// Publishes generated declarations under a Rust raw-identifier module.
        fn main() {
            neocogi_idl::cargo::Module::new("idl", "example/paths.ncidl")
                .rust_path("path_contracts::r#type").build().unwrap();
        }
    "#,
    )
    .unwrap();
    fs::write(
        directory.join("contracts/idl/example/paths.ncidl"),
        r#"
        extern neocogi.com.events;
        extern neocogi.ui.core;
        use neocogi.com.events::EventBinding;
        use neocogi.ui.core::IWidget;
        /// Nested payload exposes the real generated value type in form dispatch.
        #[version(1)] struct CounterChanged {
            /// Count received by the application's required handler.
            value: u32
        }
        /// One complete typed stream, with nested and unit variants.
        #[version(1)] enum CounterEvent {
            /// Carries the new count.
            Changed(CounterChanged),
            /// Restores the application count to zero.
            Reset
        }
        /// Concrete projection whose Rust module location uses a raw identifier.
        #[version(1)] factory CounterFactory { fn create(&self, events: &EventBinding<Counter, CounterEvent>) -> Result<ComPtr<Counter>>; }
        #[version(1)] class Counter: IWidget {}
    "#,
    )
    .unwrap();
    fs::write(
        directory.join("contracts/src/lib.rs"),
        r#"
        /// Real public module named with a Rust keyword, separate from its IDL identity.
        pub mod r#type { include!(concat!(env!("OUT_DIR"), "/example_paths.include.rs")); }
    "#,
    )
    .unwrap();
    fs::write(directory.join("contracts/src/bin/catalog.rs"), include_str!("fixtures/rust_paths/catalog.rs")).unwrap();
    fs::write(
        directory.join("app/Cargo.toml"),
        format!("[package]\nname='path-app'\nversion='0.0.0'\nedition='2024'\n{dependencies}path-contracts={{path='../contracts'}}\n",),
    )
    .unwrap();
    fs::write(directory.join("app/src/lib.rs"), include_str!("fixtures/rust_paths/application.rs")).unwrap();
    for arguments in [
        vec!["run", "--offline", "-p", "path-contracts", "--bin", "catalog", "--", "app/src"],
        vec!["test", "--offline", "-p", "path-app"],
    ] {
        let output = Command::new("cargo")
            .args(&arguments)
            .current_dir(directory)
            .env("CARGO_TARGET_DIR", root.join("target/message-proof"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{arguments:?}\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
