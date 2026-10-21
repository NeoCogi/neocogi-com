//! The bundled Option is resolved, specialized, inspected and called as an enum.
use std::{fs, path::Path, process::Command};
use neocogi_idl::{AbiType, DeclarationKind, parse_module, resolve_modules};

/// Builds a source under a chosen module name without explicit standard imports.
fn graph(name: &str, source: &str) -> neocogi_idl::ResolvedModules {
    resolve_modules(vec![parse_module(Path::new(&format!("proof/{name}.ncidl")), source).unwrap()]).unwrap()
}

/// Prelude lookup must preserve normal lexical shadowing and stable specialization identity.
#[test]
fn option_is_an_ordinary_shadowable_library_enum() {
    let implicit = graph("first", "#[version(1)] struct Value { value: Option<u32> }");
    let explicit = graph(
        "second",
        "extern neocogi.lang; use neocogi.lang::Option; #[version(1)] struct Value { value: Option<u32> }",
    );
    let instance = |graph: &neocogi_idl::ResolvedModules, owner| {
        let module = graph.module(owner).unwrap();
        let declaration = module
            .source
            .declarations
            .iter()
            .find(|declaration| declaration.specialization.is_some())
            .unwrap();
        let DeclarationKind::Enum(value) = &declaration.kind else {
            panic!("ordinary enum expected")
        };
        assert_eq!(value.variants.iter().map(|variant| variant.name.as_str()).collect::<Vec<_>>(), ["None", "Some"]);
        assert!(graph.values.definitions[&module.declaration(&declaration.name).unwrap().qualified_name].copied);
        module.declaration(&declaration.name).unwrap().clone()
    };
    let first = instance(&implicit, "proof.first");
    let second = instance(&explicit, "proof.second");
    assert_eq!((first.id, first.abi_signature), (second.id, second.abi_signature));
    let local = graph(
        "local",
        "#[version(1)] enum Option<T> { Missing, Present(T) } #[version(1)] struct Value { value: Option<u32> }",
    );
    assert!(local.module("neocogi.lang").is_none());
    let named = graph(
        "named",
        "#[version(1)] struct Option { value: u32 } #[version(1)] struct Value { value: Option }",
    );
    assert!(named.module("neocogi.lang").is_none());
}

/// Generic payloads cannot smuggle borrows, ownership transfer, or infinite layouts into calls.
#[test]
fn ordinary_optional_payload_rules_apply_at_every_use_site() {
    for source in [
        "#[version(1)] struct Invalid { value: Option<&IUnknown> }",
        "#[version(1)] interface Invalid { fn use_it(&self, value: Option<&IUnknown>) -> Result<()>; }",
        "#[version(1)] interface Invalid { fn text(&self) -> Result<Option<String>>; }",
        "#[version(1)] struct Invalid { child: Option<Invalid> }",
        "#[version(1)] struct Invalid { value: Option<Slice<u8>> }",
    ] {
        assert!(
            resolve_modules(vec![parse_module(Path::new("proof/invalid.ncidl"), source).unwrap()]).is_err(),
            "{source}"
        );
    }
    graph("tree", "#[version(1)] struct Tree { children: Option<Vec<Tree>> }");
    let source = "#[version(1)] enum Substitute<T> { Value(T) }";
    assert!(resolve_modules(vec![parse_module(Path::new("neocogi/lang.ncidl"), source).unwrap()]).is_err());
}

/// Uses names recovered from declarations so the fixture never guesses specialization hashes.
#[test]
fn optional_copied_values_and_borrowed_owned_values_cross_com() {
    let source = include_str!("../examples/generic_values.ncidl");
    let resolved = graph("options", source);
    let module = resolved.module("proof.options").unwrap();
    let DeclarationKind::Struct(names) = &module.source.declaration("OptionalValues").unwrap().kind else {
        unreachable!()
    };
    let mut generated = String::from(r#"include!(concat!(env!("OUT_DIR"), "/proof_options.include.rs"));"#);
    for (field, alias) in [
        (&names.fields[0], "MaybeCount"),
        (&names.fields[1], "MaybeText"),
        (&names.fields[2], "MaybeToken"),
        (&names.fields[3], "MaybeFlag"),
    ] {
        let AbiType::Named(name) = &field.ty else { unreachable!() };
        generated += &format!(
            "\n/// Test-facing name for a resolved ordinary enum.\npub use {} as {alias};\n",
            name.rust_path()
        );
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(project.path().join("Cargo.toml"), format!("[package]\nname='option-proof'\nversion='0.0.0'\nedition='2024'\nlinks='nc_contract_70726f6f662e6f7074696f6e73'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n", root.join("crates/com/api"))).unwrap();
    let manifest = project.path().join("Cargo.toml");
    fs::write(
        &manifest,
        fs::read_to_string(&manifest).unwrap() + &format!("\n[build-dependencies]\nneocogi-idl={{path={:?}}}\n", root.join("crates/com/idl")),
    )
    .unwrap();
    fs::create_dir_all(project.path().join("idl/proof")).unwrap();
    fs::write(project.path().join("idl/proof/options.ncidl"), source).unwrap();
    fs::write(
        project.path().join("build.rs"),
        "fn main() { neocogi_idl::cargo::Module::new(\"idl\", \"proof/options.ncidl\").build().unwrap(); }",
    )
    .unwrap();
    fs::write(project.path().join("src/contract.rs"), generated).unwrap();
    fs::write(project.path().join("src/lib.rs"), include_str!("fixtures/standard_library.rs")).unwrap();
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
