//! Native admission rejects older ABI records before calling acquisition.
#![cfg(any(unix, windows))]

use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};
use neocogi_com_sdk::{NcStatus, NC_E_ABI_VERSION, NC_E_OUT_OF_RANGE};

/// A real generated adapter makes entry into authored acquisition observable.
mod host_admission {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use neocogi_com_sdk::{ComPtr, IUnknown, NcStatus, RuntimeHandle, NC_E_CLOSED};

    /// Counts only calls that pass generated host admission.
    pub static CALLS: AtomicUsize = AtomicUsize::new(0);

    /// Fails normally after admission; no synthetic object or retained runtime is needed.
    #[neocogi_com_sdk::plugin(name = "nc.tests.v2_host_admission")]
    fn acquire(_runtime: RuntimeHandle) -> Result<ComPtr<IUnknown>, NcStatus> {
        CALLS.fetch_add(1, Ordering::SeqCst);
        Err(NC_E_CLOSED)
    }
}

/// Only these fields are readable when the host advertises an unsupported ABI.
/// No child pointers or callback storage accompany this prefix.
#[repr(C)]
struct RejectedHostPrefix {
    /// Advertised full record size; unsupported versions must still stop at the prefix.
    size: usize,
    /// ABI header words, including historical version pairs.
    words: [u32; 2],
}

/// Old host records are rejected before children or authored code.
#[test]
fn retired_host_prefixes_are_rejected_before_authored_acquisition() {
    use std::sync::atomic::Ordering;
    use neocogi_com_sdk::{ComRaw, IUnknown, IUnknownVTable, NcHostApi, NC_COM_ABI_MAGIC, NC_E_CLOSED};
    let acquire = host_admission::plugin_descriptor().acquire.unwrap();
    host_admission::CALLS.store(0, Ordering::SeqCst);
    for words in [
        [1, 18],
        [2, 19],
        [3, 21],
        [3, 22],
        [4, 21],
        [4, 22],
        [4, 23],
        [24, 0],
        [NC_COM_ABI_MAGIC, 4],
        [NC_COM_ABI_MAGIC, 24],
    ] {
        let prefix = RejectedHostPrefix {
            size: core::mem::size_of::<NcHostApi>(),
            words,
        };
        // Non-owning sentinels make clearing both raw output words observable.
        let mut output = ComRaw::<IUnknown> {
            object: core::ptr::NonNull::<u8>::dangling().as_ptr().cast(),
            vtable: core::ptr::NonNull::<IUnknownVTable>::dangling().as_ptr(),
        };
        // SAFETY: An unsupported host needs only its aligned readable ABI
        // prefix. Output is writable storage; its old pointer bits are never read.
        let status = unsafe { acquire(core::ptr::from_ref(&prefix).cast(), &mut output) };
        assert_eq!(status, NC_E_ABI_VERSION);
        assert!(output.object.is_null() && output.vtable.is_null());
        assert_eq!(host_admission::CALLS.load(Ordering::SeqCst), 0);
    }
    let mut output = ComRaw::null();
    // SAFETY: This test executable retains the selected complete process host.
    let status = unsafe { acquire(neocogi_com_host::initialize_process(), &mut output) };
    assert_eq!(status, NC_E_CLOSED);
    assert!(output.object.is_null() && output.vtable.is_null());
    assert_eq!(host_admission::CALLS.load(Ordering::SeqCst), 1);
}

/// Explicit subprocess operation; absent during the ordinary child-harness case.
const MODE: &str = "NC_GENERATION_CHILD_MODE";
/// Trusted fixture path chosen by the parent from Cargo's reported artifact.
const LIBRARY: &str = "NC_GENERATION_LIBRARY";
/// Detects any invocation of the retired callback; current bootstrap writes too.
const MARKER: &str = "NC_GENERATION_BOOTSTRAP_MARKER";
/// A required load returning would violate the mandatory failure contract.
const RETURNED: i32 = 97;

/// Stage a distinct provider mapping before rebuilding the other feature mode.
fn provider(features: Option<&str>) -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = root.join("tests/com/integration-tests/fixtures/legacy-bootstrap/Cargo.toml");
    let target = std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|| root.join("target"));
    let mut command = Command::new(env!("CARGO"));
    command
        .args(["build", "--offline", "--locked", "--message-format=json", "--manifest-path"])
        .arg(&manifest)
        .arg("--target-dir")
        .arg(target);
    if let Some(features) = features {
        command.args(["--features", features]);
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
    let current = provider(Some("current,dynamic-plugin"));
    for features in [None, Some("legacy-unified")] {
        let legacy = provider(features);
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
            detail.contains("fatal neocogi-com-sdk plugin load failure:") && detail.contains("NC_E_ABI_VERSION"),
            "wrong rejection: {detail}"
        );
        assert!(!marker.exists(), "mandatory loading invoked the retired callback");
    }
    let marker = current.path().join("bootstrap.marker");
    let accepted = child("current", current.path(), &marker);
    assert!(accepted.status.success(), "{}", String::from_utf8_lossy(&accepted.stderr));
    assert_eq!(std::fs::read(marker).unwrap(), b"current", "current callback must actually execute");
}

/// Native lookup never falls back to an ABI v1 getter.
#[test]
fn abi_v1_discovery_is_rejected_without_bootstrap() {
    let legacy = provider(Some("legacy-symbol"));
    let marker = legacy.path().join("v1-bootstrap.marker");
    let inspected = child("v1-symbol", legacy.path(), &marker);
    assert!(inspected.status.success(), "{}", String::from_utf8_lossy(&inspected.stderr));
    assert!(!marker.exists(), "ABI v1 bootstrap must never execute");
}

/// Published ABI-v2 discovery is never used as a fallback for the new family.
#[test]
fn abi_v2_discovery_is_rejected_without_bootstrap() {
    let legacy = provider(Some("legacy-v2-symbol"));
    let marker = legacy.path().join("v2-bootstrap.marker");
    let inspected = child("v2-symbol", legacy.path(), &marker);
    assert!(inspected.status.success(), "{}", String::from_utf8_lossy(&inspected.stderr));
    assert!(!marker.exists(), "ABI-v2 bootstrap must never execute");
}

/// Native lookup never falls back to an ABI v3 getter.
#[test]
fn abi_v3_discovery_is_rejected_without_bootstrap() {
    let legacy = provider(Some("legacy-v3-symbol"));
    let marker = legacy.path().join("v3-bootstrap.marker");
    let inspected = child("v3-symbol", legacy.path(), &marker);
    assert!(inspected.status.success(), "{}", String::from_utf8_lossy(&inspected.stderr));
    assert!(!marker.exists(), "the ABI-v3 acquisition callback must remain uncalled");
}

/// The former WIP ABI v4 getter remains a distinct, retired export.
#[test]
fn abi_v4_discovery_is_rejected_without_bootstrap() {
    let legacy = provider(Some("legacy-v4-symbol"));
    let marker = legacy.path().join("v4-bootstrap.marker");
    let inspected = child("v4-symbol", legacy.path(), &marker);
    assert!(inspected.status.success(), "{}", String::from_utf8_lossy(&inspected.stderr));
    assert!(!marker.exists(), "the ABI-v4 acquisition callback must remain uncalled");
}

/// Child entry: the current control makes real calls after admission.
#[test]
fn generation_child() {
    let Ok(mode) = std::env::var(MODE) else { return };
    let path = PathBuf::from(std::env::var_os(LIBRARY).expect("provider path"));
    if matches!(mode.as_str(), "v1-symbol" | "v2-symbol" | "v3-symbol" | "v4-symbol") {
        // SAFETY: Parent supplies a trusted fixture exposing only the old getter.
        let error = unsafe { neocogi_com_host::inspect_plugin_library(path) }.expect_err("retired getter must not be used");
        assert_eq!(error.kind(), neocogi_com_host::PluginLoadErrorKind::FindSymbol);
        return;
    }
    if mode == "inspect" {
        // SAFETY: Parent built immutable legacy metadata with a readable prefix.
        let error = unsafe { neocogi_com_host::inspect_plugin_library(path) }.expect_err("legacy ABI must be rejected");
        assert_eq!(error.status(), Some(NC_E_ABI_VERSION));
        assert_eq!(error.kind(), neocogi_com_host::PluginLoadErrorKind::Metadata);
        return;
    }
    // SAFETY: This executable explicitly owns the immutable process host graph.
    let runtime = unsafe { neocogi_com_sdk::RuntimeHandle::__from_host_api(neocogi_com_host::initialize_process()) }.unwrap();
    // SAFETY: Parent supplies its own trusted native fixture. Old metadata is
    // admitted only by its prefix; current metadata has complete static backing.
    let id = unsafe { runtime.load_plugin_library(path) };
    match mode.as_str() {
        "load" => std::process::exit(RETURNED),
        "current" => {
            let root = runtime.acquire_plugin_root::<neocogi_com_sdk::persistence::IObjectChildrenAbi>(id).unwrap();
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
