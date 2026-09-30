//! Accepted authored names must survive compilation and real COM conversion.
use std::{fs, path::Path, process::Command};

/// Compiles the actual IDL output, including names that used to capture codec locals.
#[test]
fn authored_names_preserve_values_across_generated_com() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        format!(
            "[package]\nname='identifier-hygiene-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            root.join("crates/com/api")
        ),
    )
    .unwrap();
    for (module, source) in [
        ("values", include_str!("fixtures/identifier_hygiene.ncidl")),
        ("shadow", include_str!("fixtures/identifier_shadow.ncidl")),
    ] {
        let graph = neocogi_idl::resolve_modules(vec![neocogi_idl::parse_module(Path::new(&format!("proof/{module}.ncidl")), source).unwrap()]).unwrap();
        let generated = neocogi_idl::generate_rust(&graph, &format!("proof.{module}")).unwrap();
        fs::write(project.path().join(format!("src/{module}.rs")), generated).unwrap();
    }
    fs::write(project.path().join("src/lib.rs"), include_str!("fixtures/identifier_hygiene.rs")).unwrap();
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
