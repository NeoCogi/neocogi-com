//! Compile the actual public guide blocks, including their bootstrap and COM calls.

use std::{fs, path::Path, process::Command};

/// Extracts one explicitly marked Markdown example, rejecting missing or duplicate blocks.
/// Labels keep prose movement independent of tests without duplicating authored code.
fn example<'a>(document: &'a str, label: &str) -> &'a str {
    let marker = format!("<!-- checked-example: {label} -->");
    assert_eq!(document.matches(&marker).count(), 1, "one {label} example must remain documented");
    let after = document.split_once(&marker).unwrap().1.trim_start();
    assert!(after.starts_with("```"), "{label} marker must immediately precede its code block");
    let (_, body) = after.split_once('\n').unwrap();
    body.split_once("\n```").expect("example has a closing fence").0
}

/// Documentation and the compiler use the same native ABI authority.
#[test]
fn current_abi_specification_matches_the_runtime_version() {
    let document = include_str!("../../../docs/abi.md");
    let version = neocogi_com::NC_COM_ABI_VERSION;
    assert!(document.starts_with(&format!("# COM ABI {version} (WIP)\n")));
    let row = format!("| Native ABI | NC_COM_ABI_VERSION == {version};");
    assert_eq!(document.lines().filter(|line| line.starts_with("| Native ABI | ")).count(), 1);
    assert!(document.lines().any(|line| line.starts_with(&row)));
    let getter = format!("| Native getter | `{}`;", neocogi_com::NC_GET_PLUGIN_DESCRIPTOR_SYMBOL);
    assert!(document.lines().any(|line| line.starts_with(&getter)));

    let migration = include_str!("../../../docs/guides/ABI_MIGRATION.md");
    let native_abi = format!("| Native ABI | v3/core 21 | {version} |");
    assert!(migration.lines().any(|line| line == native_abi));
}

/// Current guidance consumes the actual compiler encoding and Cargo name policy.
#[test]
fn authoring_guides_match_current_encoding_and_module_mangling() {
    let domain = std::str::from_utf8(neocogi_idl::MODULE_ABI_ENCODING_DOMAIN).unwrap().trim_end_matches('\0');
    let language = include_str!("../../../docs/contracts/IDL_MODULES.md");
    assert!(
        language.contains(&format!("`{domain}`")),
        "owning language reference must name the current encoding"
    );
    let setup = include_str!("../README.md");
    let links = setup.lines().find_map(|line| line.strip_prefix("links = ")).unwrap().trim_matches('"');
    assert_eq!(links, neocogi_idl::cargo::contract_links_name("example.widgets"));
    for keyword in ["class", "factory"] {
        let invalid = format!("{keyword} Removed {{}}");
        assert!(neocogi_idl::parse_module(Path::new("example/removed.ncidl"), &invalid).is_err());
    }
}

/// Guide blocks must agree on ownership, enum wire types and executable bootstrap.
#[test]
fn readme_factory_contract_and_bootstrap_execute_as_documented() {
    let document = include_str!("../../../docs/COM.md");
    let source = neocogi_idl::parse_module(Path::new("example/contracts.ncidl"), example(document, "idl")).unwrap();
    let graph = neocogi_idl::resolve_modules(vec![source]).unwrap();
    let generated = neocogi_idl::generate_rust(&graph, "example.contracts").unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(project.path().join("Cargo.toml"), format!(
        "[package]\nname='documented-com-bootstrap'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[features]\ndynamic-plugin=[]\n[dependencies]\nneocogi-com-sdk={{path={:?},features=['runtime']}}\nneocogi-com-host={{path={:?}}}\n",
        root.join("crates/com"), root.join("crates/host"),
    )).unwrap();
    // The consumer's locally generated bindings are included as a module here;
    // every authored line below comes directly from its marked public block.
    let mut implementation = format!("mod example_contracts {{\n{generated}\n}}\n");
    let language = include_str!("../../../docs/contracts/IDL_MODULES.md");
    for (label, module) in [("generic-stream", "stream"), ("generic-values", "values"), ("enum-representation", "enums")] {
        let source = neocogi_idl::parse_module(Path::new(&format!("example/{module}.ncidl")), example(language, label)).unwrap();
        let graph = neocogi_idl::resolve_modules(vec![source]).unwrap();
        let generated = neocogi_idl::generate_rust(&graph, &format!("example.{module}")).unwrap();
        implementation.push_str(&format!("mod {module} {{\n{generated}\n}}\n"));
    }
    // The cross-module language example uses explicit Rust locations, just as
    // a Cargo build does. No generated paths or declarations are rewritten.
    let sources = [("language-render", "render"), ("language-widgets", "widgets")]
        .into_iter()
        .map(|(label, module)| neocogi_idl::parse_module(Path::new(&format!("example/{module}.ncidl")), example(language, label)).unwrap())
        .collect();
    let graph = neocogi_idl::resolve_modules(sources).unwrap();
    let mut paths = neocogi_idl::generate::RustModulePaths::default();
    paths.insert("example.render", "documentation_samples::render").unwrap();
    implementation.push_str("extern crate self as documentation_samples;\n");
    for module in ["render", "widgets"] {
        let generated = neocogi_idl::generate::generate_rust_with_paths(&graph, &format!("example.{module}"), &paths).unwrap();
        implementation.push_str(&format!("mod {module} {{\n{generated}\n}}\n"));
    }
    for label in ["implementation", "plugin", "static-host"] {
        implementation.push_str(example(document, label));
        implementation.push('\n');
    }
    // Compile the guide's actual wire checks against the generated enum module.
    implementation.push_str(example(language, "enum-wire-check"));
    implementation.push('\n');
    implementation.push_str("#[test] fn documented_bootstrap() { static_example(neocogi_com_host::initialize_runtime()).unwrap(); }\n");
    fs::write(project.path().join("src/lib.rs"), implementation).unwrap();
    let output = Command::new("cargo")
        .args(["test", "--offline", "--features", "dynamic-plugin", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            std::env::var_os("CARGO_TARGET_DIR").unwrap_or_else(|| root.join("target").into_os_string()),
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let execution = String::from_utf8(output.stdout).unwrap();
    for expected in ["test documented_bootstrap ... ok", "test documented_enum_wire_contract ... ok"] {
        assert!(
            execution.lines().any(|line| line == expected),
            "documented example must execute: {expected}\n{execution}"
        );
    }
}

/// Scan every active normative guide, rather than only the page edited for a migration.
/// Dated review/evidence trees retain their recorded source scope.
#[test]
fn active_guides_do_not_reintroduce_retired_protocol_prescriptions() {
    fn markdown(root: &Path, files: &mut Vec<std::path::PathBuf>) {
        for entry in fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                markdown(&path, files);
            } else if path.extension().is_some_and(|extension| extension == "md") {
                files.push(path);
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    for relative in ["docs/contracts", "docs/guides", "docs/architecture"] {
        markdown(&root.join(relative), &mut files);
    }
    for relative in ["docs/COM.md", "crates/com/README.md", "crates/macros/README.md", "crates/idl/README.md"] {
        files.push(root.join(relative));
    }
    files.sort();
    let retired = [
        "`neocogi-com-codegen`",
        "Writable buffers are initialized in/out storage",
        "A bare struct or enum parameter is copied by value",
        "`String` is an owned immutable COM string",
        "`#[com_struct(packed)]`",
        "The generator\nuses the common `com_interface` macro internally",
        "Record labels emitted by `ComSerialize`",
        "`Message::IDL_MESSAGE`",
        "common receiver and cached weak origin",
        "QueryInterface's canonical IUnknown projection identifies a live COM object",
    ];
    for path in files {
        let source = fs::read_to_string(&path).unwrap();
        for prescription in retired {
            assert!(!source.contains(prescription), "{} retains retired guidance: {prescription}", path.display());
        }
    }
}

/// The public guide composes real policy prefixes; private/retired helpers cannot
/// be imported by an independent consumer merely because callbacks use the C ABI.
#[test]
fn provider_lifetime_prefix_guide_executes_with_private_helpers() {
    let guide = include_str!("../../../docs/guides/ABI_MIGRATION.md");
    let contract = neocogi_idl::parse_module(Path::new("proof/lifetime.ncidl"), "interface Probe { fn value(&self) -> Result<u32>; }").unwrap();
    let graph = neocogi_idl::resolve_modules(vec![contract]).unwrap();
    let bindings = neocogi_idl::generate_rust(&graph, "proof.lifetime").unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(project.path().join("Cargo.toml"), format!(
        "[package]\nname='documented-lifetime-prefix'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com-sdk={{path={:?}}}\n[[bin]]\nname='private-support'\npath='src/private.rs'\n",
        root.join("crates/com"),
    )).unwrap();
    let implementation = r#"
        static DROPS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        #[neocogi_com_sdk::com_object(Probe)] struct Shared;
        #[neocogi_com_sdk::com_object(Probe, ownership = exclusive)] struct Exclusive;
        impl Probe for Shared { fn value(&self) -> Result<u32, neocogi_com_sdk::NcStatus> { Ok(1) } }
        impl Probe for Exclusive { fn value(&self) -> Result<u32, neocogi_com_sdk::NcStatus> { Ok(2) } }
        impl Drop for Shared { fn drop(&mut self) { DROPS.fetch_add(1, std::sync::atomic::Ordering::SeqCst); } }
        impl Drop for Exclusive { fn drop(&mut self) { DROPS.fetch_add(1, std::sync::atomic::Ordering::SeqCst); } }
        const SHARED: IUnknownVTable = interface_prefix::<Shared>(&<ProbeAbi as neocogi_com_sdk::ComInterface>::INTERFACE_ID);
        const EXCLUSIVE: IUnknownVTable = interface_prefix::<Exclusive>(&<ProbeAbi as neocogi_com_sdk::ComInterface>::INTERFACE_ID);
        #[test] fn selected_prefixes_release_the_matching_policy() {
            use neocogi_com_sdk::{ComRaw, IUnknown, NC_E_INVALID_ARG, NC_OK};
            let shared = Shared::from_value(Shared);
            let exclusive = Exclusive::into_box(Exclusive);
            assert_eq!(shared.query::<ProbeAbi>().unwrap().value(), Ok(1));
            assert_eq!(exclusive.query_ref::<ProbeAbi>().unwrap().value(), Ok(2));
            let mut out = ComRaw::<IUnknown>::null();
            unsafe {
                assert_eq!((SHARED.add_ref)(shared.as_raw().object), 2);
                assert_eq!((SHARED.release)(shared.as_raw().object), 1);
                assert_eq!((SHARED.query_interface)(shared.as_raw().object, &<ProbeAbi as neocogi_com_sdk::ComInterface>::INTERFACE_ID, &mut out), NC_OK);
                assert_eq!(neocogi_com_sdk::raw::validate_interface(out.cast::<ProbeAbi>()), Ok(()));
                assert_eq!(neocogi_com_sdk::raw::release(out), 1);
                assert_eq!((EXCLUSIVE.add_ref)(exclusive.as_raw().object), 0);
                assert_eq!((EXCLUSIVE.query_interface)(exclusive.as_raw().object, &<ProbeAbi as neocogi_com_sdk::ComInterface>::INTERFACE_ID, &mut out), NC_E_INVALID_ARG);
                assert!(!out.is_present());
                let raw = exclusive.into_raw();
                assert_eq!((EXCLUSIVE.release)(raw.object), 0);
            }
            drop(shared);
            assert_eq!(DROPS.load(std::sync::atomic::Ordering::SeqCst), 2);
        }
    "#;
    fs::write(
        project.path().join("src/lib.rs"),
        format!("#![allow(dead_code)]\n{bindings}\n{}\n{implementation}", example(guide, "lifetime-prefix")),
    )
    .unwrap();
    // Each deliberately named import must fail for its own expected visibility
    // reason; an unrelated compile error cannot stand in for a missing rejection.
    let rejected = [
        ("weak_retain", "E0432"),
        ("weak_release", "E0432"),
        ("weak_upgrade", "E0432"),
        ("admit_interface", "E0432"),
        ("release_rejected_weak", "E0432"),
        ("nc_shared_release", "E0603"),
        ("nc_exclusive_release", "E0603"),
        ("nc_strong_release", "E0432"),
        ("CANONICAL_UNKNOWN_VTABLE", "E0432"),
    ];
    let private = rejected
        .iter()
        .map(|(name, _)| format!("use neocogi_com_sdk::lifetime::{name};\n"))
        .collect::<String>()
        + "fn main() {}\n";
    fs::write(project.path().join("src/private.rs"), private).unwrap();
    let target = std::env::var_os("CARGO_TARGET_DIR").unwrap_or_else(|| root.join("target").into_os_string());
    let output = Command::new("cargo")
        .current_dir(project.path())
        .args(["test", "--lib", "--offline"])
        .env("CARGO_TARGET_DIR", &target)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("test selected_prefixes_release_the_matching_policy ... ok"));
    let output = Command::new("cargo")
        .current_dir(project.path())
        .args(["check", "--bin", "private-support", "--offline", "--message-format=json"])
        .env("CARGO_TARGET_DIR", &target)
        .output()
        .unwrap();
    assert!(!output.status.success(), "private lifetime support unexpectedly became public");
    let errors = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|record| record["reason"] == "compiler-message" && record["message"]["level"] == "error")
        .map(|record| record["message"].clone())
        .collect::<Vec<_>>();
    assert_eq!(
        errors.len(),
        rejected.len(),
        "unexpected diagnostics: {errors:?}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for (name, code) in rejected {
        assert!(
            errors
                .iter()
                .any(|error| error["code"]["code"] == code && error["message"].as_str().unwrap().contains(name)),
            "missing {code} for {name}: {errors:?}"
        );
    }
}
