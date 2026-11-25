//! Mandatory plugin loads terminate the process through either linked runtime mode.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::io::Write;
use std::os::unix::{ffi::OsStringExt, process::ExitStatusExt};
use neocogi_com_sdk::{NcDebugMessage, NcDebugMessengerCreateInfo, NC_DEBUG_SEVERITY_ERROR, NC_DEBUG_TYPE_PLUGIN, NC_E_LOAD_FAILED};

/// Selects one child operation without recursively running the parent test.
const FAILURE_CASE: &str = "NC_COM_PLUGIN_LOAD_FAILURE_CASE";
/// Supplies a deliberately invalid library built by the parent process.
const LIBRARY_PATH: &str = "NC_COM_PLUGIN_LOAD_FAILURE_LIBRARY";
/// Distinguishes returning from a required load from actual process termination.
const RETURNED_EXIT_CODE: i32 = 97;

/// Checks local validation, C argument validation, and actual native loading errors.
#[test]
fn mandatory_plugin_load_failures_abort() {
    for case in ["missing", "empty", "interior_nul", "invalid_utf8"] {
        assert_load_aborts(case, None, false);
    }
    for case in ["missing_export", "rejected_entrypoint", "null_descriptor"] {
        let library = build_library(case);
        assert_load_aborts(case, Some(&library), false);
    }
}

/// Refused console output still delivers native events and cannot unwind past abort.
#[cfg(target_os = "linux")]
#[test]
fn mandatory_plugin_load_failures_abort_when_stderr_refuses_output() {
    for case in ["missing", "empty", "invalid_utf8"] {
        assert_load_aborts(case, None, true);
    }
}

/// Calls the public facade in a separate process so abort cannot kill the test runner.
fn assert_load_aborts(case: &str, library: Option<&Path>, refuse_stderr: bool) {
    let mut command = Command::new(std::env::current_exe().expect("test executable path"));
    command.args(["--exact", "mandatory_load_child", "--nocapture"]).env(FAILURE_CASE, case);
    if let Some(library) = library {
        command.env(LIBRARY_PATH, library);
    }
    if refuse_stderr {
        command.stderr(Stdio::from(std::fs::OpenOptions::new().write(true).open("/dev/full").unwrap()));
    }
    let output = command.output().expect("run mandatory-load subprocess");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(output.status.code(), Some(RETURNED_EXIT_CODE), "{case} returned from load: {stderr}");
    // Check SIGABRT specifically: a segmentation fault, Rust test panic, or a
    // missing shared runtime must not masquerade as the intended fatal policy.
    assert_eq!(output.status.signal(), Some(6), "{case} did not abort: {:?}: {stderr}", output.status);
    let detail = match case {
        "missing" => "OpenLibrary",
        "missing_export" => "FindSymbol",
        "rejected_entrypoint" => "entrypoint returned an error",
        "null_descriptor" => "entrypoint returned null",
        "invalid_utf8" => "path is not valid UTF-8",
        "empty" | "interior_nul" => "NC_E_INVALID_ARG",
        _ => panic!("unknown parent failure case"),
    };
    if !refuse_stderr {
        assert!(stderr.contains("fatal neocogi-com-sdk plugin load failure:"), "{case}: {stderr}");
        assert!(stderr.contains(detail), "{case} failed at the wrong boundary: {stderr}");
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let native = matches!(case, "missing" | "missing_export" | "rejected_entrypoint" | "null_descriptor");
    assert_eq!(stdout.matches("fatal-event|").count(), usize::from(native), "{case}: {stdout}");
    if case == "missing" {
        let event = format!("fatal-event|{NC_DEBUG_SEVERITY_ERROR}|{NC_DEBUG_TYPE_PLUGIN}|{}|true|", NC_E_LOAD_FAILED.code());
        assert!(stdout.contains(&event) && stdout.contains(detail), "{case}: {stdout}");
    }
}

/// Writes the event through an independent stream and reenters catalog inspection.
///
/// # Safety
/// The host lends a readable message and its UTF-8 bytes for this callback.
unsafe extern "C" fn observe_failure(severity: u32, category: u32, message: *const NcDebugMessage, _: *mut core::ffi::c_void) {
    // SAFETY: The installed debug callback receives one live borrowed message.
    let message = unsafe { &*message };
    // SAFETY: Runtime diagnostics lend readable UTF-8 text for the full call.
    let bytes = unsafe { std::slice::from_raw_parts(message.message.data, message.message.len) };
    let text = std::str::from_utf8(bytes).unwrap();
    let empty = host_runtime().plugins().is_ok_and(|plugins| plugins.is_empty());
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "fatal-event|{severity}|{category}|{}|{empty}|{text}", message.status.code());
    let _ = stdout.flush();
}

/// Isolated entry for each fatal failure; the ordinary test run does nothing here.
#[test]
fn mandatory_load_child() {
    let Ok(case) = std::env::var(FAILURE_CASE) else {
        return;
    };
    let path = match case.as_str() {
        "missing" => {
            let path = std::env::temp_dir().join(format!("neocogi-missing-plugin-{}.so", std::process::id()));
            assert!(!path.exists(), "missing-path fixture must not execute an existing library");
            path
        }
        "empty" => PathBuf::new(),
        "interior_nul" => PathBuf::from("plugin\0suffix"),
        "invalid_utf8" => PathBuf::from(std::ffi::OsString::from_vec(vec![0xff])),
        "missing_export" | "rejected_entrypoint" | "null_descriptor" => PathBuf::from(std::env::var_os(LIBRARY_PATH).expect("fixture library path")),
        _ => panic!("unknown child failure case"),
    };
    let runtime = crate::host_runtime();
    let info = NcDebugMessengerCreateInfo {
        size: std::mem::size_of::<NcDebugMessengerCreateInfo>(),
        severity_mask: NC_DEBUG_SEVERITY_ERROR,
        value_type_mask: NC_DEBUG_TYPE_PLUGIN,
        callback: Some(observe_failure),
        user_data: core::ptr::null_mut(),
    };
    // SAFETY: The static callback ignores user data and its code lasts until abort.
    let _observer = unsafe { runtime.debug().create_messenger(&info) }.unwrap();
    // SAFETY: paths either cannot identify a library or select the parent's
    // trusted fixture. All callable exports use the declared plugin C ABI.
    let _result = std::panic::catch_unwind(|| unsafe { runtime.load_plugin_library(path) });
    std::process::exit(RETURNED_EXIT_CODE);
}

/// Builds a dependency-free DSO with the real descriptor export ABI, or no export.
fn build_library(case: &str) -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    // Keep all fixture artifacts beneath the workspace target for cargo clean.
    let target = manifest.join("../../target/plugin-load-failure-fixtures");
    std::fs::create_dir_all(&target).expect("create fixture output directory");
    let library = target.join(format!("{}{case}{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX));
    let output = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--edition=2024", "--crate-type=cdylib", "--cfg", case])
        .arg(manifest.join("integration-tests/fixtures/plugin-load-failure.rs"))
        .arg("-o")
        .arg(&library)
        .output()
        .expect("compile invalid plugin fixture");
    assert!(output.status.success(), "compile {case}: {}", String::from_utf8_lossy(&output.stderr));
    library
}

// This final test executable owns its isolated process runtime.
extern crate neocogi_com_host as _;

/// Initializes this final test executable's selected host and admits its static tables.
fn host_runtime() -> neocogi_com_sdk::RuntimeHandle {
    // SAFETY: the explicit implementation owns its complete process-lived graph.
    unsafe { neocogi_com_sdk::RuntimeHandle::__from_host_api(neocogi_com_host::initialize_process()) }.expect("test host API is compatible")
}
