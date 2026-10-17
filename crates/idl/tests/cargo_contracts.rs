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
        let bytes = key.strip_prefix("nc_contract_").unwrap().as_bytes().chunks_exact(2)
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
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    let packages = [
        ("left", "a_b.c", "Left", 1),
        ("right", "a.b_c", "Right", 2),
        ("upper", "Case.same", "Upper", 4),
        ("lower", "case.same", "Lower", 8),
        ("host", "proof.host", "Host", 0),
    ];
    fs::write(project.path().join("Cargo.toml"), "[workspace]\nresolver='2'\nmembers=['left','right','upper','lower','host']\n").unwrap();
    for (package, module, name, _) in packages {
        let path = project.path().join(package);
        fs::create_dir_all(path.join("src")).unwrap();
        let source_path = format!("{}.ncidl", module.replace('.', "/"));
        let source = path.join("idl").join(&source_path);
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        let mut manifest = format!(
            "[package]\nname='{package}'\nversion='0.0.0'\nedition='2024'\nlinks={:?}\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            contract_links_name(module), root.join("crates/com/api"),
        );
        if package == "host" {
            for (dependency, _, _, _) in &packages[..4] {
                manifest.push_str(&format!("{dependency}={{path='../{dependency}'}}\n"));
            }
        }
        manifest.push_str(&format!("[build-dependencies]\nneocogi-idl={{path={:?}}}\n", root.join("crates/com/idl")));
        fs::write(path.join("Cargo.toml"), manifest).unwrap();
        fs::write(path.join("build.rs"), format!(
            "fn main() {{ neocogi_idl::cargo::Module::new(\"idl\", {source_path:?}).rust_path({package:?}).build().unwrap(); }}\n"
        )).unwrap();
        let mut library = format!("include!(concat!(env!(\"OUT_DIR\"), \"/{}.include.rs\"));\n", module.replace('.', "_"));
        let idl = if package == "host" {
            library.push_str(r#"
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
"#);
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
    let output = Command::new("cargo").args(["test", "--offline", "-p", "host", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target/cargo-contract-proof")).output().unwrap();
    assert!(output.status.success(), "{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("test selected_contracts_execute ... ok"));
}
