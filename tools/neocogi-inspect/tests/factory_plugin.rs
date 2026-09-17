//! Inspects a relocated factory plugin without any contract-library search path.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Builds the real generated factory fixture using ordinary Cargo.
fn build_factory_plugin() -> PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = workspace.join("target/factory-plugin-inspector-fixture");
    let output = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
        .args(["rustc", "--offline", "--locked", "--manifest-path"])
        .arg(workspace.join("examples/com-plugin/Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)
        .args(["--lib", "--crate-type", "cdylib", "--features", "dynamic-plugin"])
        .output()
        .expect("failed to build factory fixture");
    assert_success("factory plugin build", &output);
    target.join("debug").join(format!(
        "{}neocogi_example_com_plugin{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ))
}

/// Reports child diagnostics before failing a build or inspection assertion.
fn assert_success(subject: &str, output: &Output) {
    assert!(
        output.status.success(),
        "{subject} failed\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Relocation proves that inspection does not depend on generated contract DSOs.
#[test]
fn inspects_a_standalone_plugin_and_its_exact_factory_signature() {
    let original = build_factory_plugin();
    let staging = tempfile::tempdir().expect("isolated plugin directory");
    let plugin = staging.path().join(original.file_name().unwrap());
    std::fs::copy(original, &plugin).expect("relocate plugin without build outputs");
    let output = Command::new(env!("CARGO_BIN_EXE_neocogi-inspect"))
        .arg("dump")
        .arg(plugin)
        .args(["--format", "json"])
        .env_remove("LD_LIBRARY_PATH")
        .env_remove("DYLD_LIBRARY_PATH")
        .output()
        .expect("failed to inspect relocated plugin");
    assert_success("standalone plugin inspection", &output);
    let artifact: serde_json::Value = serde_json::from_slice(&output.stdout).expect("inspection JSON");
    assert_eq!(artifact["schema_version"], 9);
    assert_eq!(artifact["description_abi_version"], 2);
    assert_eq!(artifact["plugin"]["name"], "nc.examples.factory_plugin_component");
    let factory = &artifact["factory"];
    assert_eq!(factory["root_factory_signature"].as_str().unwrap().len(), 64);
    assert!(factory.get("modules").is_none());
    assert!(factory.get("direct_contracts").is_none());
}
