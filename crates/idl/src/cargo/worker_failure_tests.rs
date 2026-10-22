//! Unit-test children exercise real Cargo build admission without production controls.
use std::{
    collections::BTreeMap,
    error::Error,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};
use crate::{ModuleGraphError, ParseError, syntax::failure};
use super::{BuildError, Module, contract_links_name};

/// Selects one private child; never read by an ordinary IDL dependency build.
const CASE: &str = "NEOCOGI_IDL_UNIT_BUILD_FAILURE";
/// Libtest name plus a marker prevent a zero-test child from satisfying acceptance.
const TEST: &str = "cargo::worker_failure_tests::bundled_worker_failure_preserves_artifacts_and_metadata";
/// Written only after the child has checked the typed failure and unchanged files.
const OBSERVED: &str = "observed bundled worker rejection";

/// Captures every file, including unrelated sentinels, rather than only the .rs output.
fn snapshot(directory: &Path) -> BTreeMap<OsString, Vec<u8>> {
    fs::read_dir(directory)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (entry.file_name(), fs::read(entry.path()).unwrap())
        })
        .collect()
}

/// Identification and record parsing succeed; bundled admission returns before all writes.
#[test]
fn bundled_worker_failure_preserves_artifacts_and_metadata() {
    if let Ok(source) = std::env::var(CASE) {
        let output = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        let before = snapshot(&output);
        // Minimal one-module build: identification parse, record reparse, then
        // bundled parse (#3). Wrong-phase denial must fail the exact variant check.
        let _denial = failure::after(2);
        let builder = match source.as_str() {
            "proof/build.ncidl" => Module::new("idl", source),
            "proof/build" => Module::directory("idl", source),
            _ => panic!("unknown unit-test source case"),
        };
        let error = builder.build().unwrap_err();
        let BuildError::Resolve(ModuleGraphError::BundledSource(ParseError::Worker(cause))) = &error else {
            panic!("failure occurred outside bundled admission: {error:?}")
        };
        assert_eq!(cause.kind(), io::ErrorKind::WouldBlock);
        let graph = error.source().expect("build keeps graph cause");
        let parse = graph.source().expect("graph keeps parser cause");
        assert!(parse.source().unwrap().to_string().contains("test denied syntax worker start"));
        assert!(error.to_string().contains("bundled neocogi.lang"));
        assert_eq!(snapshot(&output), before);
        println!("{OBSERVED}");
        return;
    }

    for directory in [false, true] {
        for seeded in [false, true] {
            let fixture = tempfile::tempdir().unwrap();
            let output = fixture.path().join("output");
            fs::create_dir(&output).unwrap();
            let relative = if directory { "proof/build" } else { "proof/build.ncidl" };
            let source = fixture.path().join("idl").join(relative);
            let source = if directory { source.join("part.ncidl") } else { source };
            fs::create_dir_all(source.parent().unwrap()).unwrap();
            fs::write(source, "#[version(1)] interface Plain {}").unwrap();
            if seeded {
                for name in [
                    "proof_build.rs",
                    "proof_build.rs.map",
                    "proof_build.include.rs",
                    "neocogi-idl-graph.json",
                    "unrelated",
                ] {
                    fs::write(output.join(name), format!("previous bytes for {name}")).unwrap();
                }
            }
            let before = snapshot(&output);
            let result = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", TEST, "--nocapture", "--test-threads=1"])
                .env(CASE, relative)
                .env("CARGO_MANIFEST_DIR", fixture.path())
                .env("OUT_DIR", &output)
                .env("CARGO_MANIFEST_LINKS", contract_links_name("proof.build"))
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&result.stdout);
            assert!(result.status.success(), "{stdout}\n{}", String::from_utf8_lossy(&result.stderr));
            assert!(stdout.contains(OBSERVED), "child must execute its admission assertions: {stdout}");
            assert!(!stdout.contains("cargo::metadata="), "failed admission advertised success: {stdout}");
            assert_eq!(snapshot(&output), before);
        }
    }
}
