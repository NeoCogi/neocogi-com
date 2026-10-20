//! IDL ownership contracts compile into real factory and container ABI transfers.
use std::path::Path;
use neocogi_idl::{parse_module, resolve_modules};

/// Resolves one module so independent ownership spellings share logical names.
fn graph(source: &str) -> Result<neocogi_idl::ResolvedModules, neocogi_idl::ResolutionError> {
    resolve_modules(vec![parse_module(Path::new("proof/ownership.ncidl"), source).unwrap()])
}

/// Ownership is part of the canonical signature, not the nominal declaration identity.
#[test]
fn ownership_modes_have_distinct_revision_signatures() {
    let make = |owner: &str| {
        graph(&format!(
            "#[version(1)] interface IValue {{}} #[version(1)] interface Factory {{ fn create(&self) -> Result<{owner}<IValue>>; }}"
        ))
        .unwrap()
    };
    let shared = make("ComPtr");
    let exclusive = make("ComBox");
    let weak = make("WeakComBox");
    let signature = |g: &neocogi_idl::ResolvedModules| g.module("proof.ownership").unwrap().abi_fingerprint;
    assert_ne!(signature(&shared), signature(&exclusive));
    assert_ne!(signature(&exclusive), signature(&weak));
    assert_ne!(signature(&shared), signature(&weak));
}

/// Exclusive values cannot enter repeatable field codecs or target scalar data.
#[test]
fn invalid_exclusive_positions_fail_in_the_resolver() {
    for body in [
        "#[version(1)] struct Bad { owner: ComBox<IValue> }",
        "#[version(1)] struct Bad { owner: Vec<ComBox<IValue>> }",
        "#[version(1)] enum Bad { Owner(ComBox<IValue>) }",
        "#[version(1)] interface Bad { fn take(&self, owner: ComBox<u32>) -> Result<()>; }",
        "#[version(1)] interface Bad { fn get(&self) -> Result<ComBox<ComBox<IValue>>>; }",
    ] {
        assert!(graph(&format!("#[version(1)] interface IValue {{}} {body}")).is_err(), "accepted {body}");
    }
}

/// Generated factories and containers exercise transfer slots, cached outputs,
/// concrete destruction, and the shared inspector's ownership descriptions.
#[test]
fn generated_factories_transfer_one_owner_through_real_abi_calls() {
    let graph = graph(
        r#"
        #[version(1)] interface IValue { fn value(&self) -> Result<u32>; }
        #[version(1)] interface IContainer {
            fn insert(&mut self, child: ComBox<IValue>) -> Result<()>;
            fn child(&self) -> Result<WeakComBox<IValue>>;
            fn remove(&mut self) -> Result<()>;
        }
        #[version(1)] interface Factory { fn create(&self) -> Result<ComBox<IValue>>; }
    "#,
    )
    .unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("src")).unwrap();
    std::fs::write(
        project.path().join("Cargo.toml"),
        format!(
            "[package]\nname='exclusive-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            root.join("crates/com/api")
        ),
    )
    .unwrap();
    std::fs::write(
        project.path().join("src/lib.rs"),
        neocogi_idl::generate_rust(&graph, "proof.ownership").unwrap() + include_str!("fixtures/exclusive_ownership.rs"),
    )
    .unwrap();
    let result = std::process::Command::new("cargo")
        .args(["test", "--offline", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            std::env::var_os("CARGO_TARGET_DIR").unwrap_or_else(|| root.join("target").into_os_string()),
        )
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
