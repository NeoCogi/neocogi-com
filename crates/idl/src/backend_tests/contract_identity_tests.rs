//! Independently emitted contracts negotiate names and exact signatures without versions.

use std::{fs, path::Path, process::Command};
use super::{name_ownership_tests::run_consumers_with_files, test_consumer::Consumer};
use crate::{generate::RustModulePaths, ResolvedModules};

/// Resolves a value dependency and its callable consumer with stable logical names.
fn graph(value: &str, interface: &str) -> ResolvedModules {
    crate::resolve_modules(vec![
        crate::parse_module(Path::new("proof/units.ncidl"), value).unwrap(),
        crate::parse_module(
            Path::new("proof/clock.ncidl"),
            &format!("extern proof.units; use proof.units::Duration; {interface}"),
        )
        .unwrap(),
    ])
    .unwrap()
}

/// Each peer owns different Rust definitions while retaining its resolved contract bytes.
fn peer(name: &str, graph: &ResolvedModules) -> String {
    let mut paths = RustModulePaths::default();
    for module in ["units", "clock"] {
        paths
            .insert(&format!("proof.{module}"), &format!("identity_consumer::{name}::{module}"))
            .unwrap();
    }
    let mut source = format!("pub mod {name} {{\n");
    for module in ["units", "clock"] {
        let generated = crate::generate::generate_rust_with_paths(graph, &format!("proof.{module}"), &paths).unwrap();
        source.push_str(&format!("pub mod {module} {{\n{generated}\n}}\n"));
    }
    source.push_str("}\n");
    source
}

/// Deprecated integers, explicit overrides and unannotated peers make real calls.
/// Transitive same-layout edits reject by SHA; distinct semantic names reject by IID.
#[test]
fn versionless_peers_negotiate_exact_transitive_contracts() {
    let value = "struct Duration { ticks: u32 }";
    let interface = "interface Clock { fn read(&self) -> Result<Duration>; }";
    let plain = graph(value, interface);
    let iid = plain.module("proof.clock").unwrap().declaration("Clock").unwrap().id;
    let legacy = graph(&format!("#[version(19)] {value}"), &format!("#[version(44)] #[id(72, \"{iid}\")] {interface}"));
    let explicit = graph(value, &format!("#[id(\"{iid}\")] {interface}"));
    let changed = graph("struct Duration { milliseconds: u32 }", interface);
    let renamed = graph(value, "interface ClockMilliseconds { fn read(&self) -> Result<Duration>; }");
    let mut source = "extern crate self as identity_consumer;\n".to_owned();
    for (name, graph) in [
        ("plain", plain),
        ("legacy", legacy),
        ("explicit", explicit),
        ("changed", changed),
        ("renamed", renamed),
    ] {
        source.push_str(&peer(name, &graph));
    }
    source.push_str(
        r#"
        use neocogi_com::{ComInterface, NcStatus, NC_E_ABI_VERSION, NC_E_NOINTERFACE};
        #[neocogi_com::com_object(plain::clock::Clock)]
        struct Clock;
        impl plain::clock::Clock for Clock {
            fn read(&self) -> Result<plain::units::Duration, NcStatus> {
                Ok(plain::units::Duration { ticks: 41 })
            }
        }
        #[neocogi_com::com_object(changed::clock::Clock)]
        struct ChangedClock;
        impl changed::clock::Clock for ChangedClock {
            fn read(&self) -> Result<changed::units::Duration, NcStatus> {
                Ok(changed::units::Duration { milliseconds: 42 })
            }
        }
        #[neocogi_com::com_object(renamed::clock::ClockMilliseconds)]
        struct MillisecondClock;
        impl renamed::clock::ClockMilliseconds for MillisecondClock {
            fn read(&self) -> Result<renamed::units::Duration, NcStatus> {
                Ok(renamed::units::Duration { ticks: 43 })
            }
        }
        #[test]
        fn exact_identity_calls() {
            let root = Clock::from_value(Clock);
            assert_eq!(root.query::<plain::clock::ClockAbi>().unwrap().read().unwrap().ticks, 41);
            assert_eq!(root.query::<legacy::clock::ClockAbi>().unwrap().read().unwrap().ticks, 41);
            assert_eq!(root.query::<explicit::clock::ClockAbi>().unwrap().read().unwrap().ticks, 41);
            assert_eq!(plain::clock::ClockAbi::IID, changed::clock::ClockAbi::IID);
            assert_ne!(plain::clock::ClockAbi::ABI_SIGNATURE, changed::clock::ClockAbi::ABI_SIGNATURE);
            assert!(matches!(root.query::<changed::clock::ClockAbi>(), Err(NC_E_ABI_VERSION)));
            assert!(matches!(root.query::<renamed::clock::ClockMillisecondsAbi>(), Err(NC_E_NOINTERFACE)));
            let changed = ChangedClock::from_value(ChangedClock);
            assert_eq!(changed.query::<changed::clock::ClockAbi>().unwrap().read().unwrap().milliseconds, 42);
            assert!(matches!(changed.query::<plain::clock::ClockAbi>(), Err(NC_E_ABI_VERSION)));
            let renamed = MillisecondClock::from_value(MillisecondClock);
            assert_eq!(renamed.query::<renamed::clock::ClockMillisecondsAbi>().unwrap().read().unwrap().ticks, 43);
            assert!(matches!(renamed.query::<plain::clock::ClockAbi>(), Err(NC_E_NOINTERFACE)));
        }
    "#,
    );
    run_consumers_with_files(
        Consumer::new("versionless-identity-consumer"),
        [("independent contracts".to_owned(), source)],
        &[],
        &["exact_identity_calls"],
    );
}

/// Both supported Cargo build routes surface source diagnostics and emit callable bindings.
/// Reusing one scratch consumer bounds disk usage; the build script changes between routes.
#[test]
fn cargo_build_apis_report_deprecated_versions_without_rejecting_calls() {
    let consumer = Consumer::new("identity-warning-consumer");
    let root = consumer.directory.path();
    let manifest = root.join("Cargo.toml");
    let mut source = fs::read_to_string(&manifest).unwrap();
    source.push_str(&format!(
        "[build-dependencies]\nneocogi-idl={{path={:?}}}\n",
        Path::new(env!("CARGO_MANIFEST_DIR"))
    ));
    // The owned-module route requires the canonical links name; SharedSources
    // ignores package ownership and uses its explicitly supplied Rust location.
    source = source.replace(
        "[package]\n",
        &format!("[package]\nlinks={:?}\n", crate::cargo::contract_links_name("proof.warnings")),
    );
    fs::write(manifest, source).unwrap();
    fs::create_dir_all(root.join("schemas/proof")).unwrap();
    fs::write(
        root.join("schemas/proof/warnings.ncidl"),
        r#"
        #[version(1)] struct Value { count: u32 }
        #[id(7, "9be73186-f123-47bc-8140-4a57825eeb21")]
        interface Reader { fn read(&self) -> Result<Value>; }
    "#,
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        r#"
        extern crate self as identity_warning_consumer;
        mod contract { include!(env!("BINDINGS")); }
        #[neocogi_com::com_object(contract::Reader)] struct Reader;
        impl contract::Reader for Reader {
            fn read(&self) -> Result<contract::Value, neocogi_com::NcStatus> {
                Ok(contract::Value { count: 17 })
            }
        }
        #[test] fn warning_contract_call() {
            let reader = Reader::from_value(Reader).query::<contract::ReaderAbi>().unwrap();
            assert_eq!(reader.read().unwrap().count, 17);
        }
    "#,
    )
    .unwrap();
    for build in [
        r#"neocogi_idl::cargo::Module::new("schemas", "proof/warnings.ncidl")
            .rust_path("identity_warning_consumer::contract").build().unwrap()"#,
        r#"neocogi_idl::cargo::SharedSources::new("schemas")
            .module("proof.warnings", "identity_warning_consumer::contract").build().unwrap().remove(0)"#,
    ] {
        fs::write(
            root.join("build.rs"),
            format!(
                r#"
            fn main() {{
                let output = {build};
                println!("cargo::rustc-env=BINDINGS={{}}", output.include.display());
            }}
        "#
            ),
        )
        .unwrap();
        let output = Command::new("cargo")
            .args(["test", "--offline", "--manifest-path"])
            .arg(root.join("Cargo.toml"))
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{build}:\n{stderr}");
        assert!(stderr.contains("warning[WIDL001]: declaration versions are deprecated and ignored"), "{stderr}");
        assert!(
            stderr.contains("warning[WIDL001]: the version argument of `id` is deprecated and ignored"),
            "{stderr}"
        );
        assert!(stderr.contains("proof/warnings.ncidl:"), "{stderr}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.lines().any(|line| line == "test warning_contract_call ... ok"), "{stdout}");
    }
}
