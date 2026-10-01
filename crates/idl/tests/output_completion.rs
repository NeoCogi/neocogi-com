//! The IDL path must compile and execute the same ownership proof as Rust macros.
use std::{fs, path::Path, process::Command};

/// Runs the shared real-ABI fixture against actual generated IDL declarations.
#[test]
fn generated_consuming_outputs_prepare_before_acceptance() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let graph = neocogi_idl::resolve_modules(vec![
        neocogi_idl::parse_module(Path::new("proof/completion.ncidl"), include_str!("fixtures/output_completion.ncidl")).unwrap(),
    ])
    .unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        format!(
            "[package]\nname='output-completion-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            root.join("crates/com/api"),
        ),
    )
    .unwrap();
    let generated = neocogi_idl::generate_rust(&graph, "proof.completion").unwrap();
    let support = root.join("crates/com/api/integration-tests/support");
    fs::write(project.path().join("src/lib.rs"), format!(
        "{generated}\n#[cfg(test)] use neocogi_com::*;\n#[cfg(test)] type Excessive = [[bool; 0]; MAX_VALUE_COUNT];\n#[cfg(test)] #[path={:?}] mod allocation_probe;\n#[cfg(test)] #[path={:?}] mod generated_calls;\n",
        support.join("allocation_probe.rs"), support.join("output_completion_fixture.rs"),
    )).unwrap();
    let result = Command::new("cargo")
        .args(["test", "--offline", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target/message-proof"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
