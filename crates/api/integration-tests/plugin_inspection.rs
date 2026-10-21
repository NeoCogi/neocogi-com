//! Real owned inspection, C callback and registration agree on readable native metadata.
#![cfg(any(unix, windows))]

use std::{
    path::{Path, PathBuf},
    process::Command,
};
use neocogi_com::*;
use neocogi_com_core::NcAcquirePluginFactory;

#[path = "fixtures/plugin-inspection/src/descriptors.rs"]
mod descriptors;

/// Builds one immutable native case; Cargo tracks option_env and rebuilds when this child env changes.
/// Every binary is copied before the next build, so pinned cases never share mutable storage.
fn build_case(case: usize, directory: &Path) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("../../../target"));
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["build", "--offline", "--locked", "--manifest-path"])
        .arg(root.join("integration-tests/fixtures/plugin-inspection/Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)
        .env("NC_COM_INSPECTION_CASE", case.to_string())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let name = format!("{}nc_plugin_inspection_fixture{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX);
    let path = directory.join(format!("case{case}-{name}"));
    std::fs::copy(target.join("debug").join(name), &path).unwrap();
    path
}

/// Independent executable starts with an empty global catalog; DSO fixtures contain no runtime.
#[test]
fn inspection_and_registration_share_admission_without_sharing_publication() {
    let directory = std::env::temp_dir().join(format!("nc-inspection-cases-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let runtime = Runtime::global();
    assert!(runtime.plugins().unwrap().is_empty());
    for (case, expected) in [
        NC_OK,
        NC_OK,
        NC_E_INVALID_ARG,
        NC_E_INVALID_ARG,
        NC_E_INVALID_ARG,
        NC_E_POINTER,
        NC_E_INVALID_ARG,
        NC_E_ABI_VERSION,
        NC_E_INVALID_ARG,
        NC_E_INVALID_ARG,
    ]
    .into_iter()
    .enumerate()
    {
        let path = build_case(case, &directory);
        let before = runtime.plugins().unwrap();
        // SAFETY: the independently built fixture supplies initialized immutable metadata and code.
        let owned = unsafe { nc_com::inspect_plugin_library(&path) };
        // SAFETY: the callback is synchronous; the descriptor is admitted and its mapping pinned.
        let callback = unsafe {
            runtime.with_plugin_library_descriptor(&path, |description| {
                assert_eq!(runtime.plugins()?, before);
                assert_eq!(description.plugin_id, descriptors::CASES[case].plugin_id);
                let signature = description.root_factory.abi_signature;
                // SAFETY: this admitted descriptor supplies the complete immutable digest.
                assert_eq!(signature.as_digest().unwrap(), &descriptors::SIGNATURE);
                Ok((
                    description.acquire.unwrap(),
                    description.dependencies()?.iter().map(|dependency| dependency.plugin_id).collect::<Vec<_>>(),
                ))
            })
        };
        assert_eq!(runtime.plugins().unwrap(), before, "inspection must not publish case {case}");
        // SAFETY: the host copy and all reachable fields are immutable statics with the same schema.
        assert_eq!(unsafe { validate_plugin_descriptor(&descriptors::CASES[case]) }, expected);
        if expected.is_ok() {
            let owned = owned.unwrap();
            let (acquire, dependencies): (NcAcquirePluginFactory, _) = callback.unwrap();
            assert_eq!(owned.description_abi_version, NC_PLUGIN_DESCRIPTOR_ABI_VERSION);
            assert_eq!(owned.core_abi_version, NC_COM_CORE_ABI_VERSION);
            assert_eq!(owned.plugin_id, descriptors::CASES[case].plugin_id);
            assert_eq!(owned.qualified_name, "nc.tests.inspection.fixture");
            assert_eq!(owned.plugin_version, "2.3.4");
            assert_eq!(owned.factory.root_factory_signature, descriptors::SIGNATURE);
            assert_eq!(owned.factory.root_factory_id, descriptors::CASES[case].root_factory.iid);
            assert_eq!(
                owned.factory.root_factory_threading,
                if case == 0 {
                    NcAbiThreadingMode::ThreadLocal
                } else {
                    NcAbiThreadingMode::ThreadSafe
                }
            );
            assert_eq!(owned.dependencies, dependencies);
            if case == 1 {
                assert_eq!(dependencies, descriptors::DEPENDENCIES.map(|dependency| dependency.plugin_id));
            } else {
                assert!(dependencies.is_empty());
            }
            // On Unix unlink the original before invoking its copied function pointer.
            // Windows pins an open mapping against file deletion; process exit releases it.
            #[cfg(unix)]
            std::fs::remove_file(&path).unwrap();
            let mut out = ComRaw::null();
            // SAFETY: every native open is process-pinned; even after unlink its code is live.
            // CLOSED on the first explicit call proves metadata operations did not bootstrap it.
            assert_eq!(unsafe { acquire(&mut out) }, NC_E_CLOSED);
            assert!(out.is_null());
        } else {
            assert_eq!(owned.unwrap_err().status(), Some(expected), "owned case {case}");
            assert_eq!(callback.unwrap_err(), expected, "callback case {case}");
        }
        // SAFETY: host metadata is process-lived. Only registration checks whether dependencies exist.
        let registration = unsafe { runtime.register_plugin_descriptor(&descriptors::CASES[case]) };
        assert_eq!(
            registration,
            if case == 1 {
                Err(NC_E_NOT_FOUND)
            } else if expected.is_ok() {
                Ok(())
            } else {
                Err(expected)
            }
        );
        assert_eq!(descriptors::acquisition_count(), 0, "registration must not invoke its host factory");
    }
    assert_eq!(runtime.plugins().unwrap().len(), 1);
    // Successful owned metadata already outlives its source file; malformed mappings stay pinned too.
    #[cfg(unix)]
    std::fs::remove_dir_all(directory).unwrap();
}

// This final executable owns exactly one process runtime; fixture DSOs use core only.
extern crate nc_com as _;
