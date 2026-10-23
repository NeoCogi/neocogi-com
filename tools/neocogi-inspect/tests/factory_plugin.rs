//! Inspects a relocated factory plugin without any contract-library search path.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Reads the exact dynamic output reported by Cargo, independent of build-dir layout.
fn shared_artifact(output: &Output, target_name: &str) -> PathBuf {
    let filename = format!("{}{target_name}{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX);
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|record| record["reason"] == "compiler-artifact" && record["target"]["name"] == target_name)
        .flat_map(|record| {
            record["filenames"]
                .as_array()
                .unwrap()
                .iter()
                .map(|name| PathBuf::from(name.as_str().unwrap()))
                .collect::<Vec<_>>()
        })
        .find(|path| path.file_name().is_some_and(|name| name == filename.as_str()))
        .expect("Cargo reports the requested shared library")
}

/// Builds a pure provider that is inspectable without any initialized runtime.
fn build_factory_plugin() -> PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.join("target"));
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .args(["rustc", "--offline", "--locked", "--manifest-path"])
        .arg(workspace.join("examples/com-plugin/Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)
        .args(["--lib", "--crate-type", "cdylib", "--features", "dynamic-plugin", "--message-format=json"])
        .output()
        .expect("build runtime-free factory fixture");
    assert_success("factory plugin build", &output);
    shared_artifact(&output, "nc_example_com")
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
    assert_eq!(artifact["description_abi_version"], 1);
    assert_eq!(artifact["plugin"]["name"], "nc.examples.factory_plugin_component");
    let factory = &artifact["factory"];
    assert_eq!(factory["root_factory_signature"].as_str().unwrap().len(), 64);
    assert!(factory.get("modules").is_none());
    assert!(factory.get("direct_contracts").is_none());
}
