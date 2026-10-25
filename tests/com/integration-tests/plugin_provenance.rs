//! Real native-library publication, idempotency, conflicts and retained lifetime.
#![cfg(unix)]

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use neocogi_com::{NC_E_CLOSED, NC_OK, NcGuid, PluginOrigin, guid_from_qualified_name};

/// Carries the parent's distinct native files into the fatal-conflict subprocess.
const CONFLICT_PATHS: &str = "NC_COM_PROVENANCE_CONFLICT_PATHS";

/// Loads through the facade and raw v1 table, preserves the first path and one
/// ordered GUID entry, and invokes retained code after removing the original file.
#[test]
fn native_plugin_origin_is_atomic_idempotent_and_process_pinned() {
    let library = build_library();
    let first = library.with_file_name(format!("first-{}{}", std::process::id(), std::env::consts::DLL_SUFFIX));
    let second = library.with_file_name(format!("second-{}{}", std::process::id(), std::env::consts::DLL_SUFFIX));
    std::fs::copy(&library, &first).unwrap();
    std::fs::copy(&library, &second).unwrap();

    // Distinct copied mappings expose different descriptor addresses despite
    // identical metadata. The fatal load must fail at identity publication.
    let child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "conflicting_provider_child", "--nocapture"])
        .env(CONFLICT_PATHS, std::env::join_paths([&first, &second]).unwrap())
        .output()
        .unwrap();
    assert_eq!(child.status.signal(), Some(6), "{}", String::from_utf8_lossy(&child.stderr));
    let stderr = String::from_utf8_lossy(&child.stderr);
    assert!(stderr.contains("Registration"), "{stderr}");
    assert!(stderr.contains("NC_E_INVALID_ARG"), "{stderr}");
    std::fs::remove_file(second).unwrap();

    let runtime = crate::host_runtime();
    let id = guid_from_qualified_name("nc.tests.provenance");
    // Unregistered inspection borrows a validated descriptor, returns visitor
    // results/errors synchronously, and permits reentrant metadata reads.
    assert_eq!(
        unsafe {
            runtime.with_plugin_library_descriptor(&first, |description| {
                assert!(runtime.plugins()?.is_empty());
                assert_eq!(runtime.plugin_info(description.plugin_id)?, None);
                Ok(description.plugin_id)
            })
        }
        .unwrap(),
        id
    );
    assert_eq!(
        unsafe { runtime.with_plugin_library_descriptor(&first, |_| Err::<(), _>(NC_E_CLOSED)) },
        Err(NC_E_CLOSED)
    );
    assert!(runtime.plugins().unwrap().is_empty());
    // SAFETY: This path selects only the independently built trusted fixture.
    assert_eq!(unsafe { runtime.load_plugin_library(&first) }, id);
    assert_eq!(unsafe { runtime.load_plugin_library(&first) }, id);
    let alias = first.parent().unwrap().join(".").join(first.file_name().unwrap());
    let bytes = alias.to_str().unwrap().as_bytes();
    let mut raw_id = NcGuid::nil();
    // SAFETY: The v1 table is linked from this checkout. Path bytes and output
    // storage remain live for the synchronous ABI call.
    let status = unsafe { ((*nc_com::initialize_process().runtime).load_plugin_library_utf8)(bytes.as_ptr(), bytes.len(), &mut raw_id) };
    assert_eq!(status, NC_OK);
    assert_eq!(raw_id, id);
    let info = runtime.plugin_info(id).unwrap().unwrap();
    assert_eq!(runtime.plugins().unwrap().as_slice(), core::slice::from_ref(&info));
    assert_eq!(info.qualified_name, "nc.tests.provenance");
    assert_eq!(info.origin, PluginOrigin::Dynamic { path: first.clone() });

    std::fs::remove_file(&first).unwrap();
    assert_eq!(runtime.plugin_info(id).unwrap().unwrap(), info);
    // Reaching the distinctive bootstrap result proves dispatch never reopened
    // the missing path and the defining native mapping remains callable.
    assert_eq!(runtime.acquire_registered_plugin_root(id).unwrap_err(), NC_E_CLOSED);
}

/// Isolates the mandatory-load abort from the parent test runner.
#[test]
fn conflicting_provider_child() {
    let Some(paths) = std::env::var_os(CONFLICT_PATHS) else {
        return;
    };
    let runtime = crate::host_runtime();
    for path in std::env::split_paths(&paths) {
        // SAFETY: The parent created these trusted independent native fixtures.
        unsafe { runtime.load_plugin_library(path) };
    }
    panic!("a second provider claiming the same GUID must abort");
}

/// Builds only ABI-core and this DSO, keeping all artifacts under the workspace
/// target directory so the repository's cargo-clean policy removes them.
fn build_library() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target = manifest.join("../../target/plugin-provenance-fixture");
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["build", "--offline", "--locked", "--manifest-path"])
        .arg(manifest.join("integration-tests/fixtures/plugin-provenance/Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    target.join("debug").join(format!(
        "{}nc_plugin_provenance_fixture{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ))
}

// This final test executable owns its isolated process runtime.
extern crate nc_com as _;

/// Initializes this final test executable's selected host and admits its static tables.
fn host_runtime() -> neocogi_com::Runtime {
    // SAFETY: the explicit implementation owns its complete process-lived graph.
    unsafe { neocogi_com::Runtime::__from_host_api(nc_com::initialize_process()) }.expect("test host API is compatible")
}
