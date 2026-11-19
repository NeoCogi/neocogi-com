//! Separately built native providers share the executable's linked host catalog.
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use serde_json::Value;

/// Runs an actual locked Cargo graph and retains its artifact messages.
fn cargo(manifest: &Path, target: &Path, args: &[&str]) -> String {
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(args)
        .args(["--offline", "--locked", "--manifest-path"])
        .arg(manifest)
        .env("CARGO_TARGET_DIR", target)
        .output()
        .expect("start native fixture Cargo");
    assert!(
        output.status.success(),
        "fixture Cargo failed: {}\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// Selects a Cargo-reported executable or native library, without guessing names.
fn artifact(messages: &str, name: &str) -> PathBuf {
    for message in messages.lines().filter_map(|line| serde_json::from_str::<Value>(line).ok()) {
        if message["reason"] != "compiler-artifact" || message["target"]["name"] != name {
            continue;
        }
        if let Some(executable) = message["executable"].as_str() {
            return executable.into();
        }
        if let Some(files) = message["filenames"].as_array() {
            if let Some(path) = files.iter().filter_map(Value::as_str).find(|path| path.ends_with(std::env::consts::DLL_SUFFIX)) {
                return path.into();
            }
        }
    }
    panic!("Cargo did not report artifact {name}");
}

/// Copies the final native role to a deployment directory separate from its build.
fn deploy(source: &Path, directory: &Path) -> PathBuf {
    let destination = directory.join(source.file_name().expect("artifact filename"));
    fs::copy(source, &destination).unwrap();
    destination
}

/// A loads B through the same host during acquisition and method dispatch.
#[test]
fn linked_host_keeps_nested_providers_on_one_registry() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = root.join("tests/com/integration-tests/fixtures/linked-host/Cargo.toml");
    let target = std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|| root.join("target"));
    let deployment = tempfile::Builder::new().prefix("linked host Ω ").tempdir().unwrap();

    let built = cargo(&manifest, &target, &["build", "--workspace", "--message-format=json"]);
    let app = deploy(&artifact(&built, "linked-host-app"), deployment.path());
    let a = deploy(&artifact(&built, "linked_host_a"), deployment.path());
    let b = deploy(&artifact(&built, "linked_host_b"), deployment.path());

    let output = Command::new(app).args([b, a]).output().expect("run independent host application");
    assert!(
        output.status.success(),
        "application failed: {}\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "one host: nested value=42; plugins=2");
}
