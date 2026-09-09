use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn contract_crate_workflow_example_passes() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../workflow-examples/contract-crate/host/Cargo.toml");
    let host_target_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../target/workflow-example-contract-host");
    let plugin_target_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../target/workflow-example-contract-plugin");
    let runtime_link_dir = runtime_link_dir();
    let dylib_path_var = dylib_path_var();
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    remove_stale_runtime_library(&host_target_dir);

    let output = Command::new(cargo)
        .arg("test")
        .arg("--offline")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--target-dir")
        .arg(&host_target_dir)
        .env("NC_COM_CONTRACT_WORKFLOW_PLUGIN_TARGET_DIR", &plugin_target_dir)
        .env("RUSTFLAGS", rustflags_with_runtime_link_dir(&runtime_link_dir))
        .env(dylib_path_var, dylib_path_with_runtime_link_dir(dylib_path_var, &runtime_link_dir))
        .output()
        .expect("failed to run contract workflow example host");

    assert!(
        output.status.success(),
        "contract workflow example host failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn runtime_link_dir() -> PathBuf {
    env::var_os("NC_COM_RUNTIME_LINK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../target/debug"))
}

fn rustflags_with_runtime_link_dir(runtime_link_dir: &Path) -> String {
    let mut flags = env::var("RUSTFLAGS").unwrap_or_default();
    if !flags.is_empty() {
        flags.push(' ');
    }
    flags.push_str("-Lnative=");
    flags.push_str(&runtime_link_dir.display().to_string());
    flags.push_str(" -Lnative=");
    flags.push_str(&runtime_link_dir.join("deps").display().to_string());
    flags
}

fn dylib_path_var() -> &'static str {
    if cfg!(windows) {
        "PATH"
    } else if cfg!(target_os = "macos") {
        "DYLD_LIBRARY_PATH"
    } else {
        "LD_LIBRARY_PATH"
    }
}

fn dylib_path_with_runtime_link_dir(var: &str, runtime_link_dir: &Path) -> OsString {
    let mut paths = vec![runtime_link_dir.to_path_buf(), runtime_link_dir.join("deps")];
    if let Some(existing) = env::var_os(var) {
        paths.extend(env::split_paths(&existing));
    }
    env::join_paths(paths).expect("dynamic library search path contains invalid entry")
}

fn remove_stale_runtime_library(target_dir: &Path) {
    let filename = format!("{}nc_com_shared_runtime{}", env::consts::DLL_PREFIX, env::consts::DLL_SUFFIX);
    for dir in [target_dir.join("debug"), target_dir.join("debug/deps")] {
        let path = dir.join(&filename);
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("failed to remove stale runtime library {}: {error}", path.display()),
        }
    }
}
