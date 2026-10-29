//! Real native admission separates the published core from retired callbacks.
#![cfg(any(unix, windows))]

use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};
use neocogi_com::{NcStatus, NC_E_ABI_VERSION, NC_E_OUT_OF_RANGE};

/// Explicit subprocess operation; absent during the ordinary child-harness case.
const MODE: &str = "NC_GENERATION_CHILD_MODE";
/// Trusted fixture path chosen by the parent from Cargo's reported artifact.
const LIBRARY: &str = "NC_GENERATION_LIBRARY";
/// Detects any invocation of the retired callback; current bootstrap writes too.
const MARKER: &str = "NC_GENERATION_BOOTSTRAP_MARKER";
/// A required load returning would violate the mandatory failure contract.
const RETURNED: i32 = 97;

/// Stage a distinct provider mapping before rebuilding the other feature mode.
fn provider(current: bool) -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = root.join("tests/com/integration-tests/fixtures/legacy-bootstrap/Cargo.toml");
    let target = std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|| root.join("target"));
    let mut command = Command::new(env!("CARGO"));
    command
        .args(["build", "--offline", "--locked", "--message-format=json", "--manifest-path"])
        .arg(&manifest)
        .arg("--target-dir")
        .arg(target);
    if current {
        command.args(["--features", "current,dynamic-plugin"]);
    }
    let output = command.output().expect("build generation fixture");
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let filename = format!("{}nc_legacy_bootstrap_fixture{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX);
    let expected = manifest.canonicalize().unwrap();
    let artifact = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|record| {
            record["reason"] == "compiler-artifact"
                && record["target"]["name"] == "nc_legacy_bootstrap_fixture"
                && record["target"]["kind"] == serde_json::json!(["cdylib"])
                && record["profile"]["test"] == false
                && record["manifest_path"]
                    .as_str()
                    .is_some_and(|path| Path::new(path).canonicalize().ok().as_deref() == Some(expected.as_path()))
        })
        .flat_map(|record| {
            record["filenames"]
                .as_array()
                .unwrap()
                .iter()
                .map(|path| PathBuf::from(path.as_str().unwrap()))
                .collect::<Vec<_>>()
        })
        .find(|path| path.file_name().is_some_and(|name| name == filename.as_str()))
        .expect("Cargo reports provider library");
    let stage = tempfile::tempdir().expect("isolated provider stage");
    std::fs::copy(artifact, stage.path().join(filename)).expect("copy exact native provider");
    stage
}

/// Invoke each admission operation in its own process, with no global env edits.
fn child(mode: &str, stage: &Path, marker: &Path) -> Output {
    let filename = format!("{}nc_legacy_bootstrap_fixture{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX);
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "generation_child", "--nocapture"])
        .env(MODE, mode)
        .env(LIBRARY, stage.join(filename))
        .env(MARKER, marker)
        .output()
        .expect("run generation subprocess")
}

/// The old signature cannot pass merely by advertising a matching factory IID.
#[test]
fn retired_bootstrap_is_rejected_before_its_callback() {
    let legacy = provider(false);
    let current = provider(true);
    let marker = legacy.path().join("bootstrap.marker");
    let inspection = child("inspect", legacy.path(), &marker);
    assert!(inspection.status.success(), "{}", String::from_utf8_lossy(&inspection.stderr));
    assert!(!marker.exists(), "inspection invoked the retired callback");
    let load = child("load", legacy.path(), &marker);
    let detail = String::from_utf8_lossy(&load.stderr);
    assert!(
        !load.status.success() && load.status.code() != Some(RETURNED),
        "required load returned: {detail}"
    );
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(load.status.signal(), Some(6), "wrong failure: {detail}");
    }
    assert!(
        detail.contains("fatal neocogi-com plugin load failure:") && detail.contains("NC_E_ABI_VERSION"),
        "wrong rejection: {detail}"
    );
    assert!(!marker.exists(), "mandatory loading invoked the retired callback");
    let accepted = child("current", current.path(), &marker);
    assert!(accepted.status.success(), "{}", String::from_utf8_lossy(&accepted.stderr));
    assert_eq!(std::fs::read(marker).unwrap(), b"current", "current callback must actually execute");
}

/// Child entry: current control makes real calls; rejected legacy code never runs.
#[test]
fn generation_child() {
    let Ok(mode) = std::env::var(MODE) else { return };
    let path = PathBuf::from(std::env::var_os(LIBRARY).expect("provider path"));
    if mode == "inspect" {
        // SAFETY: Parent built immutable legacy metadata with a readable prefix.
        let error = unsafe { neocogi_com_host::inspect_plugin_library(path) }.expect_err("legacy core must be rejected");
        assert_eq!(error.status(), Some(NC_E_ABI_VERSION));
        assert_eq!(error.kind(), neocogi_com_host::PluginLoadErrorKind::Metadata);
        return;
    }
    // SAFETY: This executable explicitly owns the immutable process host graph.
    let runtime = unsafe { neocogi_com::Runtime::__from_host_api(neocogi_com_host::initialize_process()) }.unwrap();
    // SAFETY: Parent supplies its own trusted native fixture. Old metadata is
    // admitted only by its prefix; current metadata has complete static backing.
    let id = unsafe { runtime.load_plugin_library(path) };
    match mode.as_str() {
        "load" => std::process::exit(RETURNED),
        "current" => {
            let root = runtime.acquire_plugin_root::<neocogi_com::persistence::IObjectChildrenAbi>(id).unwrap();
            assert_eq!(root.count().unwrap(), 0);
            let error: NcStatus = match root.get(0) {
                Err(error) => error,
                Ok(_) => panic!("empty fixture returned a child"),
            };
            assert_eq!(error, NC_E_OUT_OF_RANGE);
        }
        _ => panic!("unknown generation child mode"),
    }
}
