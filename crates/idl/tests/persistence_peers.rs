//! Fresh IDL peers must negotiate the checked-in allocation-free persistence contracts.
use std::{fs, path::Path, process::Command};

/// Exercises all three contracts in both directions, rather than comparing only digest text.
#[test]
fn current_generated_persistence_peers_call_builtin_providers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let source = fs::read_to_string(root.join("crates/com/core/idl/neocogi/com/persistence/objects.ncidl")).unwrap();
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
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        format!(
            "[package]\nname='persistence-peers-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            root.join("crates/com/api"),
        ),
    )
    .unwrap();
    fs::write(project.path().join("src/peer.rs"), generated).unwrap();
    fs::write(project.path().join("src/lib.rs"), include_str!("fixtures/persistence_peers.rs")).unwrap();
    let output = Command::new("cargo")
        .args(["test", "--offline", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target/persistence-peers-proof"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("2 passed"),
        "both generated/built-in call directions must execute"
    );
}
