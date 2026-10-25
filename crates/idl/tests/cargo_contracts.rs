//! Real Cargo metadata exchange must distinguish every accepted logical module name.

use std::{fs, path::Path, process::Command};
use neocogi_idl::cargo::contract_links_name;

/// Encoded Cargo keys preserve separators and case even after Cargo uppercases them.
#[test]
fn contract_owner_names_round_trip_without_normalization_collisions() {
    let names = ["a_b.c", "a.b_c", "Case.same", "case.same", "_a.b2", "a._b2"];
    let mut keys = std::collections::BTreeSet::new();
    for module in names {
        let key = contract_links_name(module);
        assert!(keys.insert(key.to_ascii_uppercase()), "distinct DEP_* owner for {module}");
        let bytes = key
            .strip_prefix("nc_contract_")
            .unwrap()
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(String::from_utf8(bytes).unwrap(), module);
    }
}

/// Four ordinary contract packages compose through their build scripts, including names
/// that formerly collided before Cargo ran, plus case-only DEP_* names. Generated
/// copied-value codecs and a real COM call exercise the selected source graphs.
#[test]
fn colliding_display_names_build_and_call_together() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    let packages = [
        ("left", "a_b.c", "Left", 1),
        ("right", "a.b_c", "Right", 2),
        ("upper", "Case.same", "Upper", 4),
        ("lower", "case.same", "Lower", 8),
        ("host", "proof.host", "Host", 0),
    ];
    fs::write(
        project.path().join("Cargo.toml"),
        "[workspace]\nresolver='2'\nmembers=['left','right','upper','lower','host']\n",
    )
    .unwrap();
    for (package, module, name, _) in packages {
        let path = project.path().join(package);
        fs::create_dir_all(path.join("src")).unwrap();
        let source_path = format!("{}.ncidl", module.replace('.', "/"));
        let source = path.join("idl").join(&source_path);
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        let mut manifest = format!(
            "[package]\nname='{package}'\nversion='0.0.0'\nedition='2024'\nlinks={:?}\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            contract_links_name(module),
            root.join("crates/api"),
        );
        if package == "host" {
            for (dependency, _, _, _) in &packages[..4] {
                manifest.push_str(&format!("{dependency}={{path='../{dependency}'}}\n"));
            }
        }
        manifest.push_str(&format!("[build-dependencies]\nneocogi-idl={{path={:?}}}\n", root.join("crates/idl")));
        fs::write(path.join("Cargo.toml"), manifest).unwrap();
        fs::write(
            path.join("build.rs"),
            format!("fn main() {{ neocogi_idl::cargo::Module::new(\"idl\", {source_path:?}).rust_path({package:?}).build().unwrap(); }}\n"),
        )
        .unwrap();
        let mut library = format!("include!(concat!(env!(\"OUT_DIR\"), \"/{}.include.rs\"));\n", module.replace('.', "_"));
        let idl = if package == "host" {
            library.push_str(
                r#"
use neocogi_com::{com_object, NcStatus};
#[com_object(Host)]
struct Probe;
impl Host for Probe {
    fn sum(&self, value: Combined) -> Result<u32, NcStatus> {
        Ok(value.left.value + value.right.value + value.upper.value + value.lower.value)
    }
}
#[test]
fn selected_contracts_execute() {
    let value = Combined {
        left: Left { value: 1 }, right: Right { value: 2 },
        upper: Upper { value: 4 }, lower: Lower { value: 8 },
    };
    assert_eq!(Probe::from_value(Probe).query::<HostAbi>().unwrap().sum(value), Ok(15));
    let entry = include_str!(concat!(env!("OUT_DIR"), "/proof_host.include.rs"));
    assert!(entry.contains("IDL module `proof.host`"));
    assert!(entry.contains("Demangle:"));
}
"#,
            );
            let mut text = String::new();
            for (_, dependency, imported, _) in &packages[..4] {
                text.push_str(&format!("extern {dependency};\nuse {dependency}::{imported};\n"));
            }
            text.push_str("#[version(1)] struct Combined { left: Left, right: Right, upper: Upper, lower: Lower }\n#[version(1)] interface Host { fn sum(&self, value: Combined) -> Result<u32>; }\n");
            text
        } else {
            format!("#[version(1)] struct {name} {{ value: u32 }}\n")
        };
        fs::write(source, idl).unwrap();
        fs::write(path.join("src/lib.rs"), library).unwrap();
    }
    let output = Command::new("cargo")
        .args(["test", "--offline", "-p", "host", "--manifest-path"])
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
    assert!(String::from_utf8_lossy(&output.stdout).contains("test selected_contracts_execute ... ok"));
}

/// Real Cargo packages carry free value/interface types through two provider hops
/// and a diamond. Consumers need only the immediate provider, including when free
/// types are qualified in templates and no IDL import re-exports them by name.
#[test]
fn imported_generics_use_typed_routes_through_direct_providers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    let sources = [
        (
            "leaf",
            "proof.leaf",
            "leaf::r#type",
            vec![],
            r#"
            #[version(1)] struct Fixed { count: u32 }
            #[version(1)] interface Service { fn read(&self) -> Result<Fixed>; }
        "#,
        ),
        (
            "left",
            "proof.left",
            "left::bindings",
            vec!["leaf"],
            r#"
            extern proof.leaf;
            #[version(1)] struct Wrapper<T> { fixed: proof::leaf::Fixed, value: T }
            #[version(1)] interface Reader<T> {
                fn read(&self, source: &proof::leaf::Service, extra: T) -> Result<proof::leaf::Fixed>;
            }
        "#,
        ),
        (
            "right",
            "proof.right",
            "right::bindings",
            vec!["leaf"],
            r#"
            extern proof.leaf;
            #[version(1)] struct Other<T> { fixed: proof::leaf::Fixed, value: T }
        "#,
        ),
        (
            "bridge",
            "proof.bridge",
            "bridge",
            vec!["left", "right"],
            r#"
            extern proof.left;
            extern proof.right;
            #[version(1)] struct Combined<T> {
                left: proof::left::Wrapper<T>, right: proof::right::Other<T>
            }
            #[version(1)] interface Root<T>: proof::left::Reader<T> {}
        "#,
        ),
        (
            "consumer",
            "proof.consumer",
            "consumer",
            vec!["bridge"],
            r#"
            extern proof.bridge;
            #[version(1)] struct Input { value: proof::bridge::Combined<u32> }
            #[version(1)] interface Client: proof::bridge::Root<u32> {
                fn sum(&self, input: Input) -> Result<u32>;
            }
        "#,
        ),
    ];
    let parsed = sources
        .iter()
        .map(|(_, module, _, _, text)| neocogi_idl::parse_module(Path::new(&format!("{}.ncidl", module.replace('.', "/"))), text).unwrap())
        .collect();
    let graph = neocogi_idl::resolve_modules(parsed).unwrap();
    let module = graph.module("proof.consumer").unwrap();
    // Select public closed declarations by their retained logical template;
    // generated UUID spellings are deliberately not duplicated by this test.
    let closed = |template: &str| {
        module
            .source
            .declarations
            .iter()
            .find(|declaration| {
                declaration
                    .specialization
                    .as_ref()
                    .is_some_and(|instance| instance.template.segments.join(".") == template)
            })
            .unwrap()
            .name
            .clone()
    };
    let (wrapper, other, combined, reader, provider_root) = (
        closed("proof.left.Wrapper"),
        closed("proof.right.Other"),
        closed("proof.bridge.Combined"),
        closed("proof.left.Reader"),
        closed("proof.bridge.Root"),
    );
    fs::write(
        project.path().join("Cargo.toml"),
        "[workspace]\nresolver='2'\nmembers=['leaf','left','right','bridge','consumer']\n",
    )
    .unwrap();
    for (package, module, rust_path, dependencies, text) in &sources {
        let path = project.path().join(package);
        let source_path = format!("{}.ncidl", module.replace('.', "/"));
        let source = path.join("idl").join(&source_path);
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::create_dir_all(path.join("src")).unwrap();
        let mut manifest = format!(
            "[package]\nname='{package}'\nversion='0.0.0'\nedition='2024'\nlinks={:?}\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            contract_links_name(module),
            root.join("crates/api"),
        );
        for dependency in dependencies {
            manifest.push_str(&format!("{dependency}={{path='../{dependency}'}}\n"));
        }
        manifest.push_str(&format!("[build-dependencies]\nneocogi-idl={{path={:?}}}\n", root.join("crates/idl")));
        fs::write(path.join("Cargo.toml"), manifest).unwrap();
        fs::write(
            path.join("build.rs"),
            format!("fn main() {{ neocogi_idl::cargo::Module::new(\"idl\", {source_path:?}).rust_path({rust_path:?}).build().unwrap(); }}\n"),
        )
        .unwrap();
        fs::write(source, text).unwrap();
        let entry = format!("include!(concat!(env!(\"OUT_DIR\"), \"/{}.include.rs\"));", module.replace('.', "_"));
        let library = match *package {
            "leaf" => format!("pub mod r#type {{ {entry} }}"),
            // These convenience exports let the test implement the public
            // service without spelling compiler-private routing identifiers.
            // Generated consumers still use their compiler-selected routes.
            "left" => format!("pub mod bindings {{ {entry} }} pub use leaf::r#type::{{Fixed, Service, ServiceAbi, ServiceVtblFor}};"),
            "right" => format!("pub mod bindings {{ {entry} }}"),
            "bridge" => format!("{entry} pub use left::bindings::*; pub use right::bindings::*; pub use left::{{Fixed, Service, ServiceAbi, ServiceVtblFor}};"),
            "consumer" => format!(
                r#"
{entry}
use bridge::{{Fixed, Service, ServiceAbi, ServiceVtblFor}};
use neocogi_com::{{com_object, ComRef, NcStatus}};
#[com_object(Service)] struct Source;
impl Service for Source {{ fn read(&self) -> Result<Fixed, NcStatus> {{ Ok(Fixed {{ count: 40 }}) }} }}
#[com_object(Client)] struct Controller;
impl {reader} for Controller {{
    fn read(&self, source: ComRef<'_, ServiceAbi>, extra: u32) -> Result<Fixed, NcStatus> {{
        Ok(Fixed {{ count: source.read()?.count + extra }})
    }}
}}
impl {provider_root} for Controller {{}}
impl Client for Controller {{
    fn sum(&self, input: Input) -> Result<u32, NcStatus> {{
        Ok(input.value.left.fixed.count + input.value.left.value + input.value.right.fixed.count + input.value.right.value)
    }}
}}
#[test] fn transitive_values_and_interfaces_call() {{
    let source = Source::from_value(Source).query::<ServiceAbi>().unwrap();
    let client = Controller::from_value(Controller).query::<ClientAbi>().unwrap();
    assert_eq!(client.read(source.as_com_ref(), 2).unwrap(), Fixed {{ count: 42 }});
    assert_eq!(client.sum(Input {{ value: {combined} {{
        left: {wrapper} {{ fixed: Fixed {{ count: 10 }}, value: 3 }},
        right: {other} {{ fixed: Fixed {{ count: 20 }}, value: 9 }},
    }} }}), Ok(42));
    assert_eq!(Fixed::IDL_TYPE.rust_type, "leaf::r#type::Fixed");
    let descriptions = ClientAbi::IDL_INTERFACE.values;
    assert!(descriptions.iter().any(|value| value.rust_type == "leaf::r#type::Fixed"));
}}
"#
            ),
            _ => unreachable!("fixture package list is exhaustive"),
        };
        fs::write(path.join("src/lib.rs"), library).unwrap();
    }
    let output = Command::new("cargo")
        .args(["test", "--offline", "-p", "consumer"])
        .current_dir(project.path())
        .env("CARGO_TARGET_DIR", project.path().join("target"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("test transitive_values_and_interfaces_call ... ok"));
}
