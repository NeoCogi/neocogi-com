//! Mandatory plugin loads terminate the process through either linked runtime mode.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::os::unix::{ffi::OsStringExt, process::ExitStatusExt};

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
        assert_load_aborts(case, None);
    }
    for case in ["missing_export", "rejected_entrypoint", "null_descriptor"] {
        let library = build_library(case);
        assert_load_aborts(case, Some(&library));
    }
}

/// Calls the public facade in a separate process so abort cannot kill the test runner.
fn assert_load_aborts(case: &str, library: Option<&Path>) {
    let mut command = Command::new(std::env::current_exe().expect("test executable path"));
    command.args(["--exact", "mandatory_load_child", "--nocapture"]).env(FAILURE_CASE, case);
    if let Some(library) = library {
        command.env(LIBRARY_PATH, library);
    }
    let output = command.output().expect("run mandatory-load subprocess");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(output.status.code(), Some(RETURNED_EXIT_CODE), "{case} returned from load: {stderr}");
    // Check SIGABRT specifically: a segmentation fault, Rust test panic, or a
    // missing shared runtime must not masquerade as the intended fatal policy.
    assert_eq!(output.status.signal(), Some(6), "{case} did not abort: {:?}: {stderr}", output.status);
    assert!(stderr.contains("fatal neocogi-com plugin load failure:"), "{case}: {stderr}");
    let detail = match case {
        "missing" => "OpenLibrary",
        "missing_export" => "FindSymbol",
        "rejected_entrypoint" => "entrypoint returned an error",
        "null_descriptor" => "entrypoint returned null",
        "invalid_utf8" => "path is not valid UTF-8",
        "empty" | "interior_nul" => "NC_E_INVALID_ARG",
        _ => panic!("unknown parent failure case"),
    };
    assert!(stderr.contains(detail), "{case} failed at the wrong boundary: {stderr}");
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
    let runtime = neocogi_com::Runtime::default();
    // SAFETY: paths either cannot identify a library or select the parent's
    // trusted fixture. All callable exports use the declared plugin C ABI.
    let _loaded = unsafe { runtime.load_plugin_library(path) };
    std::process::exit(RETURNED_EXIT_CODE);
}

/// Builds a dependency-free DSO with the real descriptor export ABI, or no export.
fn build_library(case: &str) -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    // Keep all fixture artifacts beneath the workspace target for cargo clean.
    let target = manifest.join("../../../target/plugin-load-failure-fixtures");
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
