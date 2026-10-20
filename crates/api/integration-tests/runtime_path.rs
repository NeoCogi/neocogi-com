use std::path::{Path, PathBuf};
use std::process::Command;

fn run_fixture_cargo(cargo: &str, manifest: &Path, target_dir: &Path, operation: &str, args: &[&str], deny_rustdoc_warnings: bool) {
    let mut command = Command::new(cargo);
    command
        .arg(operation)
        .args(args)
        .arg("--offline")
        .arg("--locked")
        .arg("--manifest-path")
        .arg(manifest)
        .arg("--target-dir")
        .arg(target_dir);

    if deny_rustdoc_warnings {
        let rustdocflags = std::env::var("RUSTDOCFLAGS").unwrap_or_default();
        let rustdocflags = if rustdocflags.is_empty() {
            "-D warnings".to_string()
        } else {
            format!("{rustdocflags} -D warnings")
        };
        command.env("RUSTDOCFLAGS", rustdocflags);
    }

    let output = command.output().unwrap_or_else(|error| panic!("failed to run cargo {operation}: {error}"));

    assert!(
        output.status.success(),
        "runtime path fixture cargo {operation} failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn macro_output_and_rustdoc_support_an_explicit_runtime_facade() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration-tests/fixtures/runtime-path/Cargo.toml");
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../target"));
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    run_fixture_cargo(&cargo, &manifest, &target_dir, "check", &[], false);
    run_fixture_cargo(&cargo, &manifest, &target_dir, "doc", &["--no-deps"], true);
}
