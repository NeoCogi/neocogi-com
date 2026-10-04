//! Real generated factory calls prove that event ownership starts at construction.
use neocogi_idl::{parse_module, resolve_modules};
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
fn resolve(source: &str) -> Result<neocogi_idl::ResolvedModules, neocogi_idl::ResolutionError> {
    let templates = fs::read_to_string(root().join("crates/com/api/idl/neocogi/com/events/events.ncidl")).unwrap();
    resolve_modules(vec![
        parse_module(Path::new("neocogi/com/events.ncidl"), &templates).unwrap(),
        parse_module(Path::new("proof/construction.ncidl"), source).unwrap(),
    ])
}

/// Emits only after constructor contracts and their public names have resolved.
fn generate(source: &str) -> Result<String, neocogi_idl::GenerateError> {
    generate_rust(&resolve(source).unwrap(), "proof.construction")
}

/// Compiles generated values, methods and receiver metadata, then exercises
/// ownership through checked COM factory/producer calls rather than Rust shortcuts.
#[test]
fn constructor_owns_one_permanent_sink_and_publishes_its_reachable_types() {
    let source = r#"
        extern neocogi.com.events; use neocogi.com.events::{EventBinding, IEvent};
        // These local declarations must not capture support names in generated
        // event binding conversions, method results, or catalog metadata.
        #[version(1)] struct Option { marker: u32 }
        #[version(1)] struct Some { marker: u32 }
        #[version(1)] struct None { marker: u32 }
        #[version(1)] struct Changed { text: String, enabled: bool, raw_0: u8, budget: u32 }
        #[version(1)] interface IChangedEvent: IEvent {
            fn text(&self) -> Result<ComPtr<INcString>>;
            fn enabled(&self) -> Result<bool>;
            fn raw_0(&self) -> Result<u8>;
            fn budget(&self) -> Result<u32>;
        }
        #[version(1)] interface ISubmittedEvent: IEvent {}
        #[version(1)] interface Base {}
        #[version(1)] #[events(IChangedEvent, ISubmittedEvent)] class Counter: Base {
            fn emit(&mut self, value: &Changed) -> Result<()>;
            fn submit(&mut self) -> Result<()>;
            fn close(&mut self) -> Result<()>;
        }
        #[version(1)] #[threading(shared)] class SharedOrigin {}
        #[version(1)] factory SharedFactory {
            fn create(&self, events: &EventBinding) -> Result<ComPtr<SharedOrigin>>;
        }
        #[version(1)] factory Factory {
            fn create(&self, reject: bool, events: &EventBinding) -> Result<ComPtr<Counter>>;
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
            use neocogi_com::events::IEventReceiverAbi;
            pub fn cannot_attach(
                mut product: ComPtr<CounterAbi>,
                mut port: neocogi_com::events::EventPort,
                queue: ComPtr<IEventReceiverAbi>,
            ) {
                product.attach(queue.as_com_ref());
                port.attach(queue.as_com_ref());
                IEventReceiverAbi::receiver(|_, _| Ok(()));
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

/// Standalone fixtures consume the shipped common receiver from its owning crate.
fn generate_rust(graph: &neocogi_idl::ResolvedModules, module: &str) -> Result<String, neocogi_idl::GenerateError> {
    let mut paths = neocogi_idl::generate::RustModulePaths::default();
    if graph.module("neocogi.com.events").is_some() { paths.insert("neocogi.com.events", "neocogi_com::events").unwrap(); }
    neocogi_idl::generate::generate_rust_with_paths(graph, module, &paths)
}
