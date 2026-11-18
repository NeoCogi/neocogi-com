//! Unit-test children exercise real Cargo build admission without production controls.
use std::{error::Error, fs, io, path::PathBuf, process::Command};
use crate::{ModuleGraphError, ParseError, syntax::failure};
use super::{BuildError, Module, contract_links_name, test_support::snapshot};

/// Selects one private child; never read by an ordinary IDL dependency build.
const CASE: &str = "NEOCOGI_IDL_UNIT_BUILD_FAILURE";
/// Libtest name plus a marker prevent a zero-test child from satisfying acceptance.
const TEST: &str = "cargo::worker_failure_tests::bundled_worker_failure_preserves_artifacts_and_metadata";
/// Written only after the child has checked the typed failure and unchanged files.
const OBSERVED: &str = "observed bundled worker rejection";

/// The local module is parsed once before bundled admission and output writes.
#[test]
fn bundled_worker_failure_preserves_artifacts_and_metadata() {
    if let Ok(source) = std::env::var(CASE) {
        let output = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        let before = snapshot(&output);
        // The admitted local parse is moved into the resolver. Worker #2 must
        // therefore be the bundled parse for both file and directory sources.
        let _denial = failure::after(1);
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
            fs::write(source, "interface Plain {}").unwrap();
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

/// An incoming graph cannot replace the local raw record when its parse is reused.
#[test]
fn packaged_builder_rejects_a_conflicting_local_record_from_a_dependency() {
    use std::collections::BTreeMap;
    use std::path::Path;
    use super::{GRAPH_SCHEMA, GraphModule, GraphRecord, ModuleSources, dependency_graph_variable};

    const SELECT: &str = "NEOCOGI_IDL_LOCAL_RECORD_CONFLICT";
    const TEST: &str = "cargo::worker_failure_tests::packaged_builder_rejects_a_conflicting_local_record_from_a_dependency";
    const OBSERVED: &str = "observed conflicting local record";
    if std::env::var_os(SELECT).is_some() {
        let output = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        let before = snapshot(&output);
        let error = Module::new("idl", "proof/local.ncidl").build().unwrap_err();
        assert!(matches!(error, BuildError::Idl(message) if message.contains("conflicting sources or Rust locations for module \"proof.local\"")));
        assert_eq!(snapshot(&output), before);
        println!("{OBSERVED}");
        return;
    }

    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path();
    let dir = root.join("output");
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("neocogi-idl-graph.json"), "previous graph").unwrap();
    fs::create_dir_all(root.join("idl/proof")).unwrap();
    fs::write(root.join("idl/proof/local.ncidl"), "extern proof.upstream; interface Local {}").unwrap();
    let imported = root.join("dependency");
    fs::create_dir_all(imported.join("proof")).unwrap();
    let mut modules = BTreeMap::new();
    for (name, source) in [("upstream", "interface Upstream {}"), ("local", "interface Different {}")] {
        let relative = format!("proof/{name}.ncidl");
        fs::write(imported.join(&relative), source).unwrap();
        let sources = ModuleSources::load_file(&imported, Path::new(&relative)).unwrap();
        modules.insert(format!("proof.{name}"), GraphModule { sources, rust_path: None });
    }
    let graph = root.join("incoming.json");
    let record = GraphRecord {
        schema: GRAPH_SCHEMA,
        modules,
        source_roots: BTreeMap::new(),
    };
    fs::write(&graph, record.encode(&graph).unwrap()).unwrap();
    let result = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", TEST, "--nocapture", "--test-threads=1"])
        .env(SELECT, "1")
        .env("CARGO_MANIFEST_DIR", root)
        .env("OUT_DIR", &dir)
        .env("CARGO_MANIFEST_LINKS", contract_links_name("proof.local"))
        .env(dependency_graph_variable("proof.upstream"), graph)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(result.status.success(), "{stdout}\n{}", String::from_utf8_lossy(&result.stderr));
    assert!(stdout.contains(OBSERVED), "{stdout}");
    assert!(!stdout.contains("cargo::metadata="), "{stdout}");
}
