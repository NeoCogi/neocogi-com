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

/// One maintained specification owns the pre-release v1 baseline and its examples.
#[test]
fn com_abi_v1_has_one_specification_and_matching_discovery_versions() {
    assert_eq!(neocogi_com_core::NC_COM_CORE_ABI_VERSION, 1);
    assert_eq!(neocogi_com_core::NC_COM_RUNTIME_ABI_VERSION, 1);
    assert_eq!(neocogi_com_core::NC_PLUGIN_DESCRIPTOR_ABI_VERSION, 1);
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
    let mut specifications = fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name().is_some_and(|name| name.to_string_lossy().starts_with("abi-")) && path.extension().is_some_and(|extension| extension == "md")
        })
        .collect::<Vec<_>>();
    specifications.sort();
    assert_eq!(specifications, [directory.join("abi-v1.md")], "keep one COM ABI document");
    let document = include_str!("../../../docs/abi-v1.md");
    assert!(document.starts_with("# COM ABI v1\n"));
}

/// Current guidance consumes the actual compiler encoding and Cargo name policy.
/// The numbers describe separate contracts: runtime/descriptor discovery stays v1.
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
    assert_eq!(neocogi_com_core::NC_COM_RUNTIME_ABI_VERSION, 1);
    assert_eq!(neocogi_com_core::NC_PLUGIN_DESCRIPTOR_ABI_VERSION, 1);
    for keyword in ["class", "factory"] {
        let invalid = format!("#[version(1)] {keyword} Removed {{}}");
        assert!(neocogi_idl::parse_module(Path::new("example/removed.ncidl"), &invalid).is_err());
    }
}

/// The guides' ownership spelling, generated bindings and plugin bootstrap must work together.
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
        "[package]\nname='documented-com-bootstrap'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[features]\ndynamic-plugin=[]\n[dependencies]\nneocogi-com={{path={:?},features=['runtime']}}\nneocogi-com-host={{path={:?}}}\n",
        root.join("crates/com"), root.join("crates/host"),
    )).unwrap();
    // The consumer's locally generated bindings are included as a module here;
    // every authored line below comes directly from its marked public block.
    let mut implementation = format!("mod example_contracts {{\n{generated}\n}}\n");
    let language = include_str!("../../../docs/contracts/IDL_MODULES.md");
    for (label, module) in [("generic-stream", "stream"), ("generic-values", "values")] {
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
    implementation.push_str("#[test] fn documented_bootstrap() { static_example(unsafe { neocogi_com::Runtime::__from_host_api(neocogi_com_host::initialize_process()) }.unwrap()).unwrap(); }\n");
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
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("test documented_bootstrap ... ok"),
        "bootstrap must execute, not just compile"
    );
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
