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
        #[version(1)] #[message] enum Completion { Accepted { token: IToken }, Cancelled }
        #[version(1)] #[message] enum Notice { Ready, Done }
        #[version(1)] #[message] struct Batch { values: &[u32] }
        #[version(1)] interface IProducer { completed: Completion; fn emit(&mut self) -> Result<()>; }
    "#;
    let consumer = parse_module(
        Path::new("example/consumer.ncidl"),
        r#"
        extern example.messages;
        use example.messages::Completion;
        #[version(1)] interface IForward { completed: Completion; }
    "#,
    )
    .unwrap();
    let graph = resolve_modules(vec![parse_module(Path::new("example/messages.ncidl"), source).unwrap(), consumer]).unwrap();
    let generated = neocogi_idl::generate_rust(&graph, "example.messages").unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(project.path().join("Cargo.toml"), format!(
        "[package]\nname='message-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\nneocogi-com-events={{path={:?}}}\n",
        root.join("crates/com/api"), root.join("crates/com/events"),
    )).unwrap();
    // The imported-output module has no local message declaration. Compile it too,
    // so allocation imports and cross-crate receiver paths are checked by rustc.
    let consumer = neocogi_idl::generate_rust(&graph, "example.consumer").unwrap();
    fs::write(project.path().join("src/consumer.rs"), consumer).unwrap();
    fs::write(
        project.path().join("src/lib.rs"),
        generated + "\nextern crate self as example_messages_contracts;\npub mod consumer;\n" + include_str!("fixtures/messages.rs"),
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
