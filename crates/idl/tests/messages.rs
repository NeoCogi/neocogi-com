//! Compile and execute generated message delivery through real COM entry points.

use std::{fs, path::Path, process::Command};
use neocogi_idl::{parse_module, resolve_modules};

/// Proves concrete numeric/string/object messages, decoding failures and ownership.
#[test]
fn concrete_messages_cross_com_and_acquire_consumer_ownership() {
    let source = r#"
        extern neocogi.com.events; use neocogi.com.events::EventBinding;
        #[version(1)] interface IToken { fn value(&self) -> Result<u32>; }
        #[version(1)] struct NumberChanged { value: f32, checked: bool }
        #[version(1)] struct TextChanged { text: String, cursor: u64 }
        #[version(1)] struct TextEnvelope { changed: TextChanged }
        #[version(1)] enum Completion { Accepted { token: ComPtr<IToken> }, Cancelled }
        #[version(1)] enum Notice { Ready, Done }
        #[version(1)] struct Batch { values: Vec<u32> }
        #[version(1)] struct BoolBatch { flags: Vec<bool> }
        #[version(1)] struct TextPair { left: String, right: String }
        #[version(1)] struct Tree { enabled: bool, children: Vec<Tree> }
        #[version(1)] struct Shapes { optional: Option<NumberChanged>, flags: [bool; 3] }
        #[version(1)] struct NestedBatch { values: Vec<NumberChanged> }
        #[version(1)] interface IValidate {
            fn tree(&mut self, value: &Tree) -> Result<()>;
            fn shapes(&mut self, value: &Shapes) -> Result<()>;
            fn batch(&mut self, value: &NestedBatch) -> Result<()>;
        }
        #[version(1)] factory IProducerFactory { fn create(&self, events: &EventBinding<Completion>) -> Result<ComPtr<IProducer>>; }
        #[version(1)] class IProducer { fn emit(&mut self) -> Result<()>; }
        #[version(1)] interface IMessageInput { fn accept(&mut self, value: &Completion) -> Result<()>; }
        #[version(1)] enum EditorEvent {
            /// Captured edit, before any completion from the same input.
            Changed(TextChanged),
            /// Captured completion through the same stream identity.
            Completed,
        }
        #[version(1)] factory EditorFactory { fn create(&self, events: &EventBinding<EditorEvent>) -> Result<ComPtr<Editor>>; }
        #[version(1)] class Editor {
            fn emit(&mut self) -> Result<()>;
        }
        #[version(1)] factory Editors { fn create(&self) -> Result<ComPtr<Editor>>; }
    "#;
    let consumer = parse_module(
        Path::new("example/consumer.ncidl"),
        r#"
        extern example.messages;
        use example.messages::Completion;
        #[version(1)] interface IForward { fn accept(&mut self, value: &Completion) -> Result<()>; }
    "#,
    )
    .unwrap();
    let graph = resolve_modules(vec![
        parse_module(Path::new("example/messages.ncidl"), source).unwrap(),
        consumer,
        parse_module(
            Path::new("neocogi/com/events.ncidl"),
            include_str!("../../api/idl/neocogi/com/events/events.ncidl"),
        )
        .unwrap(),
    ])
    .unwrap();
    let option = graph
        .module("example.messages")
        .unwrap()
        .source
        .declarations
        .iter()
        .find(|declaration| {
            declaration
                .specialization
                .as_ref()
                .is_some_and(|instance| instance.template.segments.last().is_some_and(|name| name == "Option"))
        })
        .unwrap();
    let generated = neocogi_idl::generate_rust(&graph, "example.messages").unwrap() + &format!("\npub use {} as OptionalNumber;\n", option.name);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        format!(
            "[package]\nname='message-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            root.join("crates/com/api"),
        ),
    )
    .unwrap();
    // The imported-output module has no local message declaration. Compile it too,
    // so allocation imports and cross-crate receiver paths are checked by rustc.
    let consumer = neocogi_idl::generate_rust(&graph, "example.consumer").unwrap();
    fs::write(project.path().join("src/consumer.rs"), consumer).unwrap();
    // Compile the same imported output through a nested Rust location. This
    // checks concrete receiver ABI paths as well as ordinary authored imports.
    let mut paths = neocogi_idl::generate::RustModulePaths::default();
    paths.insert("example.messages", "example_messages_contracts::relocated").unwrap();
    let relocated = neocogi_idl::generate::generate_rust_with_paths(&graph, "example.consumer", &paths).unwrap();
    fs::write(project.path().join("src/relocated_consumer.rs"), relocated).unwrap();

    fs::write(
        project.path().join("src/lib.rs"),
        generated
            + "\nextern crate self as example_messages_contracts;\npub mod consumer;\npub mod relocated { pub use super::*; }\npub mod relocated_consumer;\n"
            + include_str!("fixtures/messages.rs"),
    )
    .unwrap();
    let output = Command::new("cargo")
        .args(["test", "--offline", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target/message-proof"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("warning:"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Generated names are real declarations and cannot shadow authored contracts.
#[test]
fn removed_observer_syntax_is_rejected() {
    for invalid in [
        "#[version(1)] #[observer] interface Empty {}",
        "#[version(1)] #[observer] interface Many { first: TextChanged; second: TextChanged; }",
        "#[version(1)] #[observer] #[threading(shared)] interface Shared { changed: TextChanged; }",
        "#[version(1)] #[observer] interface Editor { changed: TextChanged; } #[version(1)] interface EditorObserver {}",
        "#[version(1)] interface EditorObserver {} #[version(1)] #[observer] interface Editor { changed: TextChanged; }",
    ] {
        let source = format!("#[version(1)] struct TextChanged {{ text: String }} {invalid}");
        assert!(parse_module(Path::new("example/observers.ncidl"), &source).is_err(), "{invalid}");
    }
}

/// Ownership-bearing messages cannot silently become copied fields or method values.
#[test]
fn owned_message_fields_do_not_weaken_plain_abi_rules() {
    for use_site in [
        "#[version(1)] interface IWrong { fn take(&self, value: TextChanged) -> Result<()>; }",
        "#[version(1)] interface IWrong { fn get(&self) -> Result<TextChanged>; }",
    ] {
        let source = format!("#[version(1)] struct TextChanged {{ text: String }} {use_site}");
        let module = parse_module(Path::new("example/messages.ncidl"), &source).unwrap();
        assert!(resolve_modules(vec![module]).is_err(), "{use_site}");
    }
}

/// Nested messages preserve finite inline layout; ordinary copied values still
/// reject them, and ownership cycles fail before Rust code generation.
#[test]
fn nested_message_layouts_reject_inline_cycles() {
    for source in [
        "#[version(1)] struct Loop { value: Loop }",
        "#[version(1)] struct First { next: Second } #[version(1)] struct Second { next: First }",
    ] {
        let error = resolve_modules(vec![parse_module(Path::new("example/messages.ncidl"), source).unwrap()])
            .unwrap_err()
            .to_string();
        assert!(error.contains("recursive copied-value layout"), "{error}");
    }
}

/// All ordinary aggregate fields participate in recursive conversion without annotations.
#[test]
fn nested_records_and_aggregate_sequences_resolve_uniformly() {
    let source = r#"
        #[version(1)] struct Inner { checked: bool }
        #[version(1)] struct Outer { nested: Inner, values: Vec<Inner>, option: Option<Inner> }
        #[version(1)] struct Tree { children: Vec<Tree> }
        #[version(1)] interface IUse { fn accept(&self, value: &Outer) -> Result<()>; }
    "#;
    let graph = resolve_modules(vec![parse_module(Path::new("example/values.ncidl"), source).unwrap()]).unwrap();
    assert!(graph.values.definitions["example.values.Inner"].copied);
    assert!(!graph.values.definitions["example.values.Outer"].copied);
    assert!(neocogi_idl::generate_rust(&graph, "example.values").unwrap().contains("com_value"));
}

/// Inline option recursion is just as infinite as an ordinary recursive field.
#[test]
fn optional_cycles_and_removed_message_annotation_are_rejected() {
    let source = "#[version(1)] struct StillInvalid { child: Option<StillInvalid> }";
    assert!(
        resolve_modules(vec![parse_module(Path::new("example/values.ncidl"), source).unwrap()])
            .unwrap_err()
            .to_string()
            .contains("recursive copied-value layout")
    );
    assert!(parse_module(Path::new("example/values.ncidl"), "#[version(1)] #[message] struct Gone { value: bool }").is_err());
}
