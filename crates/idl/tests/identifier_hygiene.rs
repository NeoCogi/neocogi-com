//! Accepted authored names must survive compilation and real COM conversion.
use std::{fs, path::Path, process::Command};

/// Every callable role rejects handle helpers at authored IDL locations. Include
/// inherited exposure and a closed generic, not just a simple top-level trait.
#[test]
fn public_handle_names_reject_before_rust_generation() {
    for name in neocogi_com_core::NC_COM_HANDLE_METHOD_NAMES {
        for source in [
            format!("#[version(1)] interface ICollision {{ fn {name}(&self) -> Result<u32>; }}"),
            format!("#[version(1)] interface Collision {{ fn {name}(&self) -> Result<u32>; }}"),
            format!("#[version(1)] interface Collision {{ fn {name}(&self) -> Result<u32>; }}"),
            format!("#[version(1)] interface IBase {{ fn {name}(&self) -> Result<u32>; }} #[version(1)] interface IDerived: IBase {{}}"),
            format!("#[version(1)] interface ICollision<T> {{ fn {name}(&self) -> Result<T>; }} #[version(1)] interface Closed: ICollision<u32> {{}}"),
            format!("#[version(1)] interface IUnused<T> {{ fn {name}(&self) -> Result<T>; }}"),
        ] {
            let module = neocogi_idl::parse_module(Path::new("proof/collision.ncidl"), &source).unwrap();
            let error = neocogi_idl::resolve_modules(vec![module]).unwrap_err();
            let neocogi_idl::ModuleGraphError::Resolution(error) = error else {
                panic!("expected semantic diagnostics")
            };
            assert!(error.diagnostics.iter().any(|diagnostic| diagnostic.code == "EIDL100"), "{source}: {error}");
            assert!(error.to_string().contains(name), "{source}: {error}");
        }
        // Raw Rust method identifiers are not part of the IDL snake_case grammar;
        // they already reject before resolution. There is no parallel handwritten
        // Rust declaration frontend with a different admission rule.
        let raw = format!("#[version(1)] interface ICollision {{ fn r#{name}(&self) -> Result<u32>; }}");
        assert!(neocogi_idl::parse_module(Path::new("proof/raw_collision.ncidl"), &raw).is_err());
    }
    let source = "#[version(1)] interface ICapability { fn downgrade(&self) -> Result<WeakComPtr<ICapability>>; }";
    let module = neocogi_idl::parse_module(Path::new("proof/exact_return.ncidl"), source).unwrap();
    let neocogi_idl::ModuleGraphError::Resolution(error) = neocogi_idl::resolve_modules(vec![module]).unwrap_err() else {
        panic!("expected semantic diagnostics")
    };
    assert!(error.diagnostics.iter().any(|d| d.code == "EIDL100"));
}

/// Compiles the actual IDL output, including names that used to capture codec locals.
#[test]
fn authored_names_preserve_values_across_generated_com() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        format!(
            "[package]\nname='identifier-hygiene-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            root.join("crates/com/api")
        ),
    )
    .unwrap();
    for (module, source) in [
        ("values", include_str!("../src/backend_tests/fixtures/identifier_hygiene.ncidl")),
        ("shadow", include_str!("../src/backend_tests/fixtures/identifier_shadow.ncidl")),
        ("arguments", include_str!("../src/backend_tests/fixtures/identifier_arguments.ncidl")),
        ("support", include_str!("../src/backend_tests/fixtures/identifier_support.ncidl")),
    ] {
        let graph = neocogi_idl::resolve_modules(vec![neocogi_idl::parse_module(Path::new(&format!("proof/{module}.ncidl")), source).unwrap()]).unwrap();
        let generated = neocogi_idl::generate_rust(&graph, &format!("proof.{module}")).unwrap();
        fs::write(project.path().join(format!("src/{module}.rs")), generated).unwrap();
    }
    fs::write(
        project.path().join("src/lib.rs"),
        include_str!("../src/backend_tests/fixtures/identifier_hygiene.rs"),
    )
    .unwrap();
    let result = Command::new("cargo")
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
