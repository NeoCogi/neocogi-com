//! Native Windows inspection rejects a NUL path before entering a DLL getter.
#![cfg(windows)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use nc_com::{inspect_plugin_library, PluginLoadErrorKind};

/// Uses fresh processes, so a previously pinned mapping cannot mask truncation.
#[test]
fn embedded_nul_path_is_rejected_before_descriptor_getter() {
    let temporary = tempfile::tempdir().unwrap();
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target = temporary.path().join("target");
    let build = Command::new(env!("CARGO"))
        .args(["build", "--offline", "--locked", "--manifest-path"])
        .arg(manifest.join("integration-tests/fixtures/plugin-provenance/Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)
        .output()
        .unwrap();
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let directory = temporary.path().join("可信插件");
    fs::create_dir(&directory).unwrap();
    let library = directory.join("路径.dll");
    fs::copy(target.join("debug/nc_plugin_provenance_fixture.dll"), &library).unwrap();
    let library = library.canonicalize().unwrap();
    let marker = temporary.path().join("getter.marker");
    for mode in ["valid", "nul", "missing"] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "windows_path_child", "--nocapture"])
            .env("NC_COM_WINDOWS_PATH_CASE", mode)
            .env("NC_COM_WINDOWS_PATH_LIBRARY", &library)
            .env("NC_COM_WINDOWS_PATH_MARKER", &marker)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{mode}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("test windows_path_child ... ok"));
        if mode == "valid" {
            assert_eq!(fs::read(&marker).unwrap(), b"descriptor getter entered");
            fs::remove_file(&marker).unwrap();
        } else {
            assert!(!marker.exists(), "{mode} entered the descriptor getter");
        }
    }
}

/// Receives only NUL-free environment data; creates the malformed OsString here.
/// Each invocation starts before this process has ever opened the trusted DLL.
#[test]
fn windows_path_child() {
    let Some(mode) = std::env::var_os("NC_COM_WINDOWS_PATH_CASE") else {
        return;
    };
    let library = PathBuf::from(std::env::var_os("NC_COM_WINDOWS_PATH_LIBRARY").unwrap());
    let path = match mode.to_str().unwrap() {
        "valid" => library,
        "nul" => {
            let mut path = library.into_os_string();
            path.push("\0suffix");
            PathBuf::from(path)
        }
        "missing" => library.with_file_name("missing.dll"),
        _ => panic!("unknown path fixture mode"),
    };
    // SAFETY: the parent independently built this trusted descriptor-only DLL.
    let result = unsafe { inspect_plugin_library(&path) };
    if mode == "valid" {
        let description = result.unwrap();
        assert_eq!(description.qualified_name, "nc.tests.provenance");
        assert_eq!(description.plugin_id, neocogi_com::guid_from_qualified_name("nc.tests.provenance"));
    } else {
        let error = result.unwrap_err();
        assert_eq!(error.kind(), PluginLoadErrorKind::OpenLibrary);
        assert_eq!(error.path(), path);
        if mode == "nul" {
            assert!(error.message().contains("interior NUL"), "{error}");
        }
    }
}
