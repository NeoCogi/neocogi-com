//! Independent native clients prove static/dynamic parity and one nested catalog.
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
fn deploy(source: &Path, directory: &Path, name: Option<&str>) -> PathBuf {
    let destination = directory.join(name.map(std::ffi::OsStr::new).unwrap_or_else(|| source.file_name().unwrap()));
    fs::copy(source, &destination).unwrap();
    destination
}

/// Executes a final application with loader settings selected by the test only.
fn run(app: &Path, args: &[&std::ffi::OsStr], failure: Option<&str>) -> String {
    let mut command = Command::new(app);
    command.args(args).env_remove("NC_TEST_HOST_FAILURE");
    if let Some(failure) = failure {
        command.env("NC_TEST_HOST_FAILURE", failure);
    }
    let output = command.output().expect("run independent host application");
    assert!(
        output.status.success(),
        "application failed: {}\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// A dynamic application links no owner, yet A -> B reaches one host across DSOs.
/// The static composition exercises precisely the same providers and assertions.
#[test]
fn shared_and_static_hosts_keep_nested_providers_on_one_registry() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = root.join("tests/com/integration-tests/fixtures/shared-host/Cargo.toml");
    let target = std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|| root.join("target"));
    let deployment = tempfile::Builder::new().prefix("shared host Ω ").tempdir().unwrap();
    let tree = cargo(
        &manifest,
        &target,
        &[
            "tree",
            "-p",
            "shared-host-app",
            "--no-default-features",
            "--edges",
            "normal,build",
            "--prefix",
            "none",
        ],
    );
    assert!(!tree.contains("neocogi-com-host"), "dynamic application embeds a host owner: {tree}");
    let shim = cargo(
        &root.join("Cargo.toml"),
        &target,
        &["build", "-p", "neocogi-com-host-dylib", "--message-format=json"],
    );
    let host = deploy(&artifact(&shim, "neocogi_com_host_dylib"), deployment.path(), None);
    let built = cargo(&manifest, &target, &["build", "--workspace", "--message-format=json"]);
    let dynamic_app = deploy(
        &artifact(&built, "shared-host-app"),
        deployment.path(),
        Some(&format!("dynamic{}", std::env::consts::EXE_SUFFIX)),
    );
    let a = deploy(&artifact(&built, "shared_host_a"), deployment.path(), None);
    let b = deploy(&artifact(&built, "shared_host_b"), deployment.path(), None);
    let bad = deploy(&artifact(&built, "shared_host_bad"), deployment.path(), None);
    let dynamic = run(&dynamic_app, &["run".as_ref(), host.as_os_str(), b.as_os_str(), a.as_os_str()], None);
    assert_eq!(dynamic.trim(), "one host: nested value=42; plugins=2");
    // Missing image and wrong export have no static owner available as fallback.
    for (path, kind, mode) in [
        (deployment.path().join("missing-host"), "OpenLibrary", None),
        (b.clone(), "FindSymbol", None),
        (bad.clone(), "Entrypoint", Some("status")),
        (bad.clone(), "InvalidApi", Some("null")),
        (bad, "InvalidApi", Some("version")),
    ] {
        let output = run(&dynamic_app, &["error".as_ref(), path.as_os_str(), kind.as_ref()], mode);
        assert_eq!(output.trim(), format!("host error: {kind}"));
    }
    let built = cargo(
        &manifest,
        &target,
        &["build", "-p", "shared-host-app", "--features", "static-host", "--message-format=json"],
    );
    let static_app = deploy(
        &artifact(&built, "shared-host-app"),
        deployment.path(),
        Some(&format!("static{}", std::env::consts::EXE_SUFFIX)),
    );
    let b = deployment
        .path()
        .join(format!("{}shared_host_b{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX));
    let linked = run(&static_app, &["run".as_ref(), host.as_os_str(), b.as_os_str(), a.as_os_str()], None);
    assert_eq!(linked, dynamic);
}
