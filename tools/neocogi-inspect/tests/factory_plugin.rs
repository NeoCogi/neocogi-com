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

/// Exercises each real output sink independently; earlier writes cannot satisfy later cases.
fn assert_native_output_paths(plugin: &Path, intended: &Path, neighbor: &Path) {
    assert_ne!(intended, neighbor);
    for command in ["dump", "plan"] {
        for option in ["--output", "-o"] {
            std::fs::write(intended, b"intended sentinel").unwrap();
            std::fs::write(neighbor, b"neighbor sentinel").unwrap();
            let output = Command::new(env!("CARGO_BIN_EXE_neocogi-inspect"))
                .arg(command)
                .arg(plugin)
                .args(["--format", "json", option])
                .arg(intended)
                .env_remove("LD_LIBRARY_PATH")
                .env_remove("DYLD_LIBRARY_PATH")
                .output()
                .expect("inspect to native output path");
            assert_success(command, &output);
            let artifact: serde_json::Value = serde_json::from_slice(&std::fs::read(intended).unwrap()).expect("command-specific output JSON");
            match command {
                "dump" => {
                    assert_eq!(artifact["schema_version"], 9);
                    assert_eq!(artifact["plugin"]["name"], "nc.examples.factory_plugin_component");
                }
                "plan" => {
                    assert_eq!(artifact["schema"], "neocogi-com.load-plan");
                    assert_eq!(artifact["plugins"].as_array().unwrap().len(), 1);
                    assert_eq!(artifact["load_order"].as_array().unwrap().len(), 1);
                    assert_eq!(artifact["load_order"][0]["name"], "nc.examples.factory_plugin_component");
                }
                _ => unreachable!(),
            }
            assert_eq!(std::fs::read(neighbor).unwrap(), b"neighbor sentinel");
        }
        let missing = intended.parent().unwrap().join("missing-plugin.so");
        assert!(!missing.exists());
        let mut inline_output = std::ffi::OsString::from("--output=");
        inline_output.push(intended);
        for option in [inline_output, std::ffi::OsString::from("--format=json")] {
            std::fs::write(intended, b"intended sentinel").unwrap();
            std::fs::write(neighbor, b"neighbor sentinel").unwrap();
            let output = Command::new(env!("CARGO_BIN_EXE_neocogi-inspect"))
                .arg(command)
                .arg(&missing)
                .arg(option)
                .output()
                .expect("reject inline option before inspection");
            assert_eq!(
                output.status.code(),
                Some(2),
                "usage failure, not operational load failure: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(std::fs::read(intended).unwrap(), b"intended sentinel");
            assert_eq!(std::fs::read(neighbor).unwrap(), b"neighbor sentinel");
        }
    }
}

/// Relocation and native output identity share one provider build and staging owner.
#[test]
fn inspects_a_standalone_plugin_and_its_exact_factory_signature() {
    let original = build_factory_plugin();
    let staging = tempfile::tempdir().expect("isolated plugin directory");
    let plugin = staging.path().join(original.file_name().unwrap());
    std::fs::copy(original, &plugin).expect("relocate plugin without build outputs");
    let output = Command::new(env!("CARGO_BIN_EXE_neocogi-inspect"))
        .arg("dump")
        .arg(&plugin)
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

    assert_native_output_paths(&plugin, &staging.path().join("report-é.json"), &staging.path().join("unicode-neighbor.json"));
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::ffi::OsStringExt;
        let intended = staging.path().join(std::ffi::OsString::from_vec(b"report-\xff.json".to_vec()));
        let neighbor = staging.path().join("report-\u{fffd}.json");
        assert_native_output_paths(&plugin, &intended, &neighbor);
    }
}
