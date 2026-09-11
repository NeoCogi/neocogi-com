//! End-to-end inspection of a factory plugin backed by a generated contract DSO.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Built plugin path and the directory containing its exact native contract.
struct FactoryPluginBuild {
    /// Factory plugin shared library passed to the inspector.
    plugin: PathBuf,
    /// Build-script output directory searched by the platform loader in this test.
    contract_directory: PathBuf,
}

/// Builds the repository's factory plugin example in an isolated target tree.
fn build_factory_plugin() -> FactoryPluginBuild {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = workspace.join("crates/com/api/Cargo.toml");
    let target = workspace.join("target/factory-plugin-inspector-fixture");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .arg("build")
        .arg("--offline")
        .arg("--locked")
        .arg("--manifest-path")
        .arg(manifest)
        .arg("--target-dir")
        .arg(&target)
        .arg("--example")
        .arg("factory_plugin_component")
        .output()
        .expect("failed to start factory plugin fixture build");
    assert_success("factory plugin fixture", &output);
    let plugin = target.join("debug").join("examples").join(format!(
        "{}factory_plugin_component{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX,
    ));
    let build_directory = target.join("debug/build");
    let contract_directory = std::fs::read_dir(build_directory)
        .expect("fixture build directory must exist")
        .map(|entry| entry.expect("build directory entry must be readable").path().join("out"))
        .find(|out| {
            std::fs::read_dir(out).is_ok_and(|entries| {
                entries.filter_map(Result::ok).any(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(&format!("{}nc_contract_neocogi_idl_example_", std::env::consts::DLL_PREFIX))
                })
            })
        })
        .expect("generated contract library directory must exist");
    FactoryPluginBuild { plugin, contract_directory }
}

/// Requires a successful child process while preserving its diagnostics.
fn assert_success(subject: &str, output: &Output) {
    assert!(
        output.status.success(),
        "{subject} failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Confirms auto-detection and owned schema output for the complete IDL graph.
#[test]
fn dumps_a_factory_plugin_and_its_canonical_contract_module() {
    let build = build_factory_plugin();
    let mut command = Command::new(env!("CARGO_BIN_EXE_neocogi-inspect"));
    command.arg("dump").arg(build.plugin).arg("--format").arg("json");
    // Plain Cargo does not add an install-relative plugin runpath. The
    // deployment tool does; this focused inspector test supplies the same
    // build-script output directory through the platform's test environment.
    #[cfg(target_os = "linux")]
    command.env("LD_LIBRARY_PATH", &build.contract_directory);
    #[cfg(target_os = "macos")]
    command.env("DYLD_LIBRARY_PATH", &build.contract_directory);
    let output = command.output().expect("failed to start neocogi-inspect");
    assert_success("factory plugin inspection", &output);

    let artifact: serde_json::Value = serde_json::from_slice(&output.stdout).expect("inspection output must be JSON");
    assert_eq!(artifact["schema_version"], 6);
    assert_eq!(artifact["plugin"]["name"], "nc.examples.factory_plugin_component");
    let factory = &artifact["activation"]["factory"];
    assert_eq!(factory["modules"][0]["name"], "neocogi.idl_example");
    assert!(factory["modules"][0]["source"].as_str().unwrap().contains("factory ExampleFactory"));
    assert_eq!(factory["direct_contracts"][0]["abi_fingerprint"].as_str().unwrap().len(), 64);
}
