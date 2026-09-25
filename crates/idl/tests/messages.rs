//! Compile and execute generated message delivery through real COM entry points.

use std::{fs, path::Path, process::Command};
use neocogi_idl::{parse_module, resolve_modules};

/// Proves concrete numeric/string/object messages, decoding failures and ownership.
#[test]
fn concrete_messages_cross_com_and_acquire_consumer_ownership() {
    let source = r#"
        #[version(1)] interface IToken { fn value(&self) -> Result<u32>; }
        #[version(1)] #[message] struct NumberChanged { value: f32, checked: bool }
        #[version(1)] #[message] struct TextChanged { text: String, cursor: u64 }
        #[version(1)] #[message] struct TextEnvelope { changed: TextChanged }
        #[version(1)] #[message] enum Completion { Accepted { token: IToken }, Cancelled }
        #[version(1)] #[message] enum Notice { Ready, Done }
        #[version(1)] #[message] struct Batch { values: &[u32] }
        #[version(1)] #[message] struct BoolBatch { flags: &[bool] }
        #[version(1)] #[message] struct TextPair { left: String, right: String }
        #[version(1)] interface IProducer { completed: Completion; fn emit(&mut self) -> Result<()>; }
        #[version(1)] interface IMessageInput { fn accept(&mut self, value: &Completion) -> Result<()>; }
        #[version(1)] #[message] enum EditorEvent {
            /// Captured edit, before any completion from the same input.
            Changed(TextChanged),
            /// Captured completion through the same observer identity.
            Completed,
        }
        #[version(1)] #[observer] interface Editor {
            /// The sole output carries every editor event to its owner.
            event: EditorEvent;
            fn emit(&mut self) -> Result<()>;
        }
        #[version(1)] factory Editors { fn create(&self, observer: Option<&EditorObserver>) -> Result<Editor>; }
    "#;
    let consumer = parse_module(
        Path::new("example/consumer.ncidl"),
        r#"
        extern example.messages;
        use example.messages::Completion;
        #[version(1)] interface IForward { completed: Completion; fn accept(&mut self, value: &Completion) -> Result<()>; }
    "#,
    )
    .unwrap();
    let graph = resolve_modules(vec![parse_module(Path::new("example/messages.ncidl"), source).unwrap(), consumer]).unwrap();
    let generated = neocogi_idl::generate_rust(&graph, "example.messages").unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(project.path().join("Cargo.toml"), format!(
        "[package]\nname='message-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
        root.join("crates/com/api"),
    )).unwrap();
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
        generated + "\nextern crate self as example_messages_contracts;\npub mod consumer;\npub mod relocated { pub use super::*; }\npub mod relocated_consumer;\n" + include_str!("fixtures/messages.rs"),
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
fn observer_contracts_validate_names_and_policy() {
    for invalid in [
        "#[version(1)] #[observer] interface Empty {}",
        "#[version(1)] #[observer] interface Many { first: TextChanged; second: TextChanged; }",
        "#[version(1)] #[observer] #[threading(shared)] interface Shared { changed: TextChanged; }",
        "#[version(1)] #[observer] interface Editor { changed: TextChanged; } #[version(1)] interface EditorObserver {}",
        "#[version(1)] interface EditorObserver {} #[version(1)] #[observer] interface Editor { changed: TextChanged; }",
    ] {
        let source = format!("#[version(1)] #[message] struct TextChanged {{ text: String }} {invalid}");
        assert!(parse_module(Path::new("example/observers.ncidl"), &source).is_err(), "{invalid}");
    }
}

/// Ownership-bearing messages cannot silently become copied fields or method values.
#[test]
fn owned_message_fields_do_not_weaken_plain_abi_rules() {
    for use_site in [
        "#[version(1)] struct Copied { value: TextChanged }",
        "#[version(1)] interface IWrong { fn take(&self, value: TextChanged) -> Result<()>; }",
        "#[version(1)] interface IWrong { fn get(&self) -> Result<TextChanged>; }",
    ] {
        let source = format!("#[version(1)] #[message] struct TextChanged {{ text: String }} {use_site}");
        let module = parse_module(Path::new("example/messages.ncidl"), &source).unwrap();
        assert!(resolve_modules(vec![module]).is_err(), "{use_site}");
    }
}

/// Nested messages preserve finite inline layout; ordinary copied values still
/// reject them, and ownership cycles fail before Rust code generation.
#[test]
fn nested_message_layouts_reject_inline_cycles() {
    for source in [
        "#[version(1)] #[message] struct Loop { value: Loop }",
        "#[version(1)] #[message] struct First { next: Second } #[version(1)] #[message] struct Second { next: First }",
    ] {
        let error = resolve_modules(vec![parse_module(Path::new("example/messages.ncidl"), source).unwrap()]).unwrap_err().to_string();
        assert!(error.contains("recursive copied-value layout"), "{error}");
    }
}

/// A nested message cannot silently acquire ordinary copied-aggregate semantics,
/// even if its current innermost payload happens to be copyable.
#[test]
fn nested_messages_stay_out_of_plain_value_fields() {
    let source = r#"
        #[version(1)] #[message] struct Inner { value: u32 }
        #[version(1)] #[message] struct Outer { nested: Inner }
        #[version(1)] struct Copied { value: Outer }
    "#;
    let error = resolve_modules(vec![parse_module(Path::new("example/messages.ncidl"), source).unwrap()]).unwrap_err().to_string();
    assert!(error.contains("cannot embed a message"), "{error}");
}

/// A message slice requires an element codec before a foreign buffer can be
/// interpreted as initialized Rust values. Copied structs and enums have no
/// generated checked element codec, even when their current fields look simple.
#[test]
fn message_slices_reject_unvalidated_aggregate_elements() {
    for declaration in [
        "#[version(1)] struct Item { checked: bool }",
        "#[version(1)] enum Item { Ready, Waiting }",
    ] {
        let source = format!("{declaration} #[version(1)] #[message] struct Batch {{ values: &[Item] }}");
        let module = parse_module(Path::new("example/invalid_slice.ncidl"), &source).unwrap();
        let error = resolve_modules(vec![module]).unwrap_err().to_string();
        assert!(error.contains("scalar message-slice element"), "{error}");
    }
}
