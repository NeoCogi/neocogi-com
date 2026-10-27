//! Unit payloads are checked data, independently of status-only method results.

use std::{fs, path::Path, process::Command};

/// Generates and executes real calls under the compiler running this test.
#[test]
fn unit_values_use_checked_storage_in_copied_calls() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let source = r#"
        #[version(1)] enum Completion { Done(()), Idle }
        #[version(1)] struct Mixed { before: u32, unit: (), enabled: bool, after: u32 }
        #[version(1)] struct Arrays { nested: [[(); 2]; 2], empty: [(); 0] }
        #[version(1)] interface IUnits {
            fn status(&self) -> Result<()>;
            fn completion(&mut self, value: Completion) -> Result<Completion>;
            fn optional(&self, value: Option<()>) -> Result<Option<()>>;
            fn mixed(&mut self, value: Mixed) -> Result<Mixed>;
            fn arrays(&self, value: Arrays) -> Result<Arrays>;
            fn direct(&self, value: [(); 2]) -> Result<[(); 2]>;
            fn buffers(&mut self, input: &[()], output: &mut [()]) -> Result<()>;
            fn finish(&self, output: &mut [Completion]) -> Result<ComPtr<IUnknown>>;
        }
    "#;
    let graph = neocogi_idl::resolve_modules(vec![neocogi_idl::parse_module(Path::new("proof/units.ncidl"), source).unwrap()]).unwrap();
    assert!(graph.values.definitions["proof.units.Completion"].copied);
    assert!(graph.values.definitions["proof.units.Mixed"].copied);
    let module = graph.module("proof.units").unwrap();
    let optional = &module.source.declarations.iter().find(|d| d.specialization.is_some()).unwrap().name;
    let mut generated = neocogi_idl::generate_rust(&graph, "proof.units").unwrap();
    generated.push_str(&format!("\npub use {optional} as Maybe;\n"));
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    // The generated runtime fixture deliberately uses alloc-only facade support.
    fs::write(project.path().join("Cargo.toml"), format!(
        "[package]\nname='unit-values-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?},default-features=false,features=['alloc','macros']}}\n",
        root.join("crates/com"),
    )).unwrap();
    fs::write(
        project.path().join("src/lib.rs"),
        format!("{generated}\n{}", include_str!("fixtures/unit_values.rs")),
    )
    .unwrap();
    let cargo = env!("CARGO");
    let version = Command::new(cargo).arg("--version").output().unwrap();
    assert!(version.status.success());
    println!("active unit-value compiler: {}", String::from_utf8_lossy(&version.stdout).trim());
    let output = Command::new(cargo)
        .args(["test", "--offline", "--manifest-path"])
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
    assert!(String::from_utf8_lossy(&output.stdout).contains("unit_calls_and_malformed_storage ... ok"));

    // Bare unit input remains deliberately excluded from the method language.
    let module = neocogi_idl::parse_module(
        Path::new("proof/bare.ncidl"),
        "#[version(1)] interface Bare { fn accept(&self, value: ()) -> Result<()>; }",
    )
    .unwrap();
    assert!(neocogi_idl::resolve_modules(vec![module]).unwrap_err().to_string().contains("EIDL061"));
}
