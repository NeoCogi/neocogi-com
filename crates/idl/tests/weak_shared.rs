//! Shared weak ownership stays explicit through resolution, generation and real ABI calls.
use neocogi_idl::{generate_rust, parse_module, resolve_modules};
use std::{fs, path::Path, process::Command};

/// Resolves independently authored variants at the same nominal module identity.
fn graph(source: &str) -> Result<neocogi_idl::ResolvedModules, neocogi_idl::ResolutionError> {
    resolve_modules(vec![parse_module(Path::new("proof/weak.ncidl"), source).unwrap()])
}

/// Every ownership mode has a distinct signature, including through generic arguments.
#[test]
fn weak_shared_identity_and_capabilities_are_distinct() {
    let mut fingerprints = std::collections::BTreeSet::new();
    for wrapper in ["ComPtr", "WeakComPtr", "ComBox", "WeakComBox"] {
        let g = graph(&format!("#[version(1)] interface IValue {{}} #[version(1)] interface Source {{ fn get(&self) -> Result<{wrapper}<IValue>>; }}")).unwrap();
        assert!(fingerprints.insert(g.module("proof.weak").unwrap().abi_fingerprint));
    }
    let g = graph(r#"
        #[version(1)] interface IValue {}
        #[version(1)] struct Handle<T> { value: T }
        #[version(1)] struct References {
            strong: Handle<ComPtr<IValue>>,
            weak: Handle<WeakComPtr<IValue>>,
            nested: Option<Vec<WeakComPtr<INcString>>>,
        }
    "#,
    ).unwrap();
    let fields = &g.values.definitions["proof.weak.References"].fields;
    assert_ne!(fields[0], fields[1], "specialization identity retains ownership");
    let value = &g.values.definitions["proof.weak.References"];
    assert!(!value.copied);
    assert_eq!(value.persistence, neocogi_idl::Persistence::Unsupported);
    let source = generate_rust(&g, "proof.weak").unwrap();
    assert!(source.contains("IdlValueType::WeakObject"));
    assert!(source.contains("WeakComPtr<::neocogi_com::INcStringAbi>"));
}

/// Weak handles require an interface target and borrowed aggregate input ownership.
#[test]
fn invalid_weak_positions_and_targets_are_rejected() {
    for body in [
        "#[version(1)] struct Bad { value: WeakComPtr<u32> }",
        "#[version(1)] struct Bad { value: WeakComPtr<ComPtr<IValue>> }",
        "#[version(1)] struct Bad { value: WeakComPtr<Unknown> }",
        "#[version(1)] interface Bad { fn take(&self, value: WeakComPtr<IValue>) -> Result<()>; }",
        "#[version(1)] interface Bad { fn take(&self, value: &[WeakComPtr<IValue>]) -> Result<()>; }",
        "#[version(1)] interface Bad { fn get(&self) -> Result<Vec<WeakComPtr<IValue>>>; }",
        "#[version(1)] struct WeakComPtr { value: u32 }",
        "#[version(1)] struct Bad { value: WeakLocalCom<IValue> }",
        "#[version(1)] #[persistence(default)] struct Bad { value: WeakComPtr<IValue> }",
        "#[version(1)] struct Record { value: WeakComPtr<IValue> } #[version(1)] interface Bad { fn take(&self, value: Record) -> Result<()>; }",
    ] {
        assert!(graph(&format!("#[version(1)] interface IValue {{}} {body}")).is_err(), "accepted {body}");
    }
    for ty in ["WeakComPtr<>", "WeakComPtr<IValue, IValue>"] {
        assert!(parse_module(Path::new("proof/weak.ncidl"), &format!("#[version(1)] struct Bad {{ value: {ty} }}")).is_err());
    }
}

/// Both shared threading modes and expired tokens execute through generated code.
#[test]
fn generated_weak_values_and_results_preserve_lifetime_and_validation() {
    let g = graph(r#"
        #[version(1)] interface Counter {
            fn value(&self) -> Result<u32>;
            fn set_value(&mut self, value: u32) -> Result<()>;
        }
        #[version(1)] #[threading(shared)] interface Shared { fn value(&self) -> Result<u32>; }
        #[version(1)] struct References {
            one: WeakComPtr<Counter>,
            many: Vec<WeakComPtr<Counter>>,
            fixed: [WeakComPtr<Counter>; 2],
        }
        #[version(1)] enum WeakEvent { Observed(WeakComPtr<Counter>), Empty }
        #[version(1)] interface Store {
            fn get(&self) -> Result<WeakComPtr<Counter>>;
            fn capture(&mut self, values: &References) -> Result<()>;
            fn shared(&self) -> Result<WeakComPtr<Shared>>;
            fn event(&mut self, value: &WeakEvent) -> Result<()>;
        }
    "#,
    ).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(project.path().join("Cargo.toml"), format!(
        "[package]\nname='weak-shared-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
        root.join("crates/com/api"),
    ),
    ).unwrap();
    fs::write(project.path().join("src/lib.rs"), generate_rust(&g, "proof.weak").unwrap() + include_str!("fixtures/weak_shared.rs"),
    ).unwrap();
    let result = Command::new("cargo").args(["test", "--offline", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target/weak-shared-proof"))
        .output().unwrap();
    assert!(result.status.success(), "{}\n{}", String::from_utf8_lossy(&result.stdout), String::from_utf8_lossy(&result.stderr));
    assert!(String::from_utf8_lossy(&result.stdout).contains("4 passed"), "generated ABI tests must execute");
}
