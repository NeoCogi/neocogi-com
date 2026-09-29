//! Real generated factory calls prove that event ownership starts at construction.
use neocogi_idl::{generate_rust, parse_module, resolve_modules};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Locates the checked-in generic COM contracts independently of the test cwd.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap()
}

/// Resolves one independent consumer against the exact shipped library templates.
fn generate(source: &str) -> Result<String, neocogi_idl::GenerateError> {
    let templates = fs::read_to_string(root().join("crates/com/api/idl/neocogi/com/events/events.ncidl")).unwrap();
    let graph = resolve_modules(vec![
        parse_module(Path::new("neocogi/com/events.ncidl"), &templates).unwrap(),
        parse_module(Path::new("proof/construction.ncidl"), source).unwrap(),
    ])
    .unwrap();
    generate_rust(&graph, "proof.construction")
}

/// Compiles generated values, methods and receiver metadata, then exercises
/// ownership through checked COM factory/producer calls rather than Rust shortcuts.
#[test]
fn constructor_owns_one_permanent_sink_and_publishes_its_reachable_types() {
    let source = r#"
        extern neocogi.com.events; use neocogi.com.events::EventBinding;
        #[version(1)] struct Changed { text: String, enabled: bool }
        #[version(1)] enum Event { Changed(Changed), Submitted }
        #[version(1)] interface Base {}
        #[version(1)] class Counter: Base {
            fn emit(&mut self, value: &Event) -> Result<()>;
            fn close(&mut self) -> Result<()>;
        }
        #[version(1)] #[threading(shared)] class SharedOrigin {}
        #[version(1)] factory SharedFactory {
            fn create(&self, events: &EventBinding<SharedOrigin, Event>) -> Result<ComPtr<SharedOrigin>>;
        }
        #[version(1)] factory Factory {
            fn create(&self, reject: bool, events: &EventBinding<Counter, Event>) -> Result<ComPtr<Counter>>;
        }
    "#;
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        format!(
            "[package]\nname='construction-events-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            root().join("crates/com/api")
        ),
    )
    .unwrap();
    fs::write(project.path().join("src/contract.rs"), generate(source).unwrap()).unwrap();
    fs::write(project.path().join("src/lib.rs"), include_str!("fixtures/construction_events.rs")).unwrap();
    let result = Command::new("cargo")
        .args(["test", "--offline", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root().join("target/message-proof"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    // An independently compiled caller cannot reopen either the public product
    // or its private producer helper. Runtime replacement rejection alone would
    // leave the unwanted attachment protocol available to future consumers.
    fs::write(
        project.path().join("src/lib.rs"),
        r#"
            pub mod contract;
            use contract::*;
            use neocogi_com::ComPtr;
            use neocogi_com::events::EventQueueInterface;
            pub fn cannot_attach(
                mut product: ComPtr<CounterAbi>,
                mut port: CounterEventPort,
                queue: ComPtr<CounterEventQueueAbi>,
            ) {
                product.attach(queue.as_com_ref());
                port.attach(queue.as_com_ref());
                CounterEventQueueAbi::receiver(|_, _| Ok(()));
            }
        "#,
    )
    .unwrap();
    let result = Command::new("cargo")
        .args(["check", "--offline", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root().join("target/message-proof"))
        .output()
        .unwrap();
    let diagnostics = String::from_utf8_lossy(&result.stderr);
    assert!(!result.status.success(), "late attachment unexpectedly compiled");
    assert_eq!(diagnostics.matches("no method named `attach`").count(), 2, "{diagnostics}");
    assert!(diagnostics.contains("named `receiver` found"), "{diagnostics}");
}

/// Two receiver arguments would weaken the one-port contract; two constructors
/// disagreeing about a product's event type would make generated forms ambiguous.
#[test]
fn constructor_event_contract_rejects_multiple_or_inconsistent_sinks() {
    let prefix = r#"
        extern neocogi.com.events; use neocogi.com.events::EventBinding;
        #[version(1)] enum A { A }
        #[version(1)] enum B { B }
        #[version(1)] class Product {}
    "#;
    let duplicate = format!(
        "{prefix} #[version(1)] factory Factory {{ fn create(&self, a: &EventBinding<Product, A>, b: &EventBinding<Product, A>) -> Result<ComPtr<Product>>; }}"
    );
    assert!(generate(&duplicate).unwrap_err().to_string().contains("multiple event bindings"));
    let inconsistent = format!(
        "{prefix} #[version(1)] factory Factory {{ fn first(&self, events: &EventBinding<Product, A>) -> Result<ComPtr<Product>>; fn second(&self, events: &EventBinding<Product, B>) -> Result<ComPtr<Product>>; }}"
    );
    assert!(generate(&inconsistent).unwrap_err().to_string().contains("disagree on their event type"));
}

/// A factory cannot claim an unrelated object as its product's event origin.
#[test]
fn constructor_rejects_a_different_origin_contract() {
    let source = r#"
        extern neocogi.com.events; use neocogi.com.events::EventBinding;
        #[version(1)] enum Event { Changed }
        #[version(1)] class Product {}
        #[version(1)] class Other {}
        #[version(1)] factory Factory {
            fn create(&self, events: &EventBinding<Other, Event>) -> Result<ComPtr<Product>>;
        }
    "#;
    assert!(generate(source).unwrap_err().to_string().contains("returned product as event origin"));
}
