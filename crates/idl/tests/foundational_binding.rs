//! One binding phase owns generic/nongeneric lookup and supplying Rust projections.
use neocogi_idl::{DeclarationKind, Module, ResolvedModules, parse_module, resolve_modules};
use std::{fs, path::Path, process::Command};

/// Parses source with the actual logical module location used during binding.
fn module(path: &str, source: &str) -> Module {
    parse_module(Path::new(path), source).unwrap()
}

/// Own-module qualification has one meaning in fields, methods and bases.
#[test]
fn qualified_local_references_do_not_depend_on_unused_templates() {
    let source = r#"

        #[version(1)] struct Local { value: u32 }
        #[version(1)] struct Holder { value: proof::binding::Local }
        #[version(1)] interface IBase { fn read(&self) -> Result<proof::binding::Local>; }
        #[version(1)] interface Fact { fn value(&self) -> Result<proof::binding::Local>; }
        #[version(1)] interface Receiver: proof::binding::IBase {
            fn echo(&self, value: &proof::binding::Holder) -> Result<proof::binding::Local>;
        }
    "#;
    for unused in ["", "#[version(1)] struct Unused<T> { value: T }"] {
        let graph = resolve_modules(vec![module("proof/binding.ncidl", &format!("{source}\n{unused}"))]).unwrap();
        neocogi_idl::generate_rust(&graph, "proof.binding").unwrap();
    }
    let error = resolve_modules(vec![module(
        "proof/binding.ncidl",
        "extern proof.binding; #[version(1)] struct Local { value:u32 }",
    )])
    .unwrap_err();
    assert!(error.to_string().contains("cannot depend on itself"));
}

/// Independent diagnostics keep their classification, source and count across generic use.
#[test]
fn independent_name_and_storage_errors_have_one_source_aware_policy() {
    let source = "#[version(1)] struct Broken { first: Missing, second: Other }\n#[version(1)] struct Borrowed { text: &str }\n";
    let mut previous = None;
    for unused in ["", "#[version(1)] struct Unused<T> { value: T }"] {
        let error = resolve_modules(vec![module("proof/errors.ncidl", &format!("{source}{unused}"))]).unwrap_err();
        let neocogi_idl::ModuleGraphError::Resolution(error) = error else {
            panic!("expected semantic diagnostics")
        };
        assert_eq!(error.diagnostics.len(), 3);
        let rendered = error.to_string();
        assert!(rendered.contains("unknown IDL type \"Missing\""));
        assert!(rendered.contains("unknown IDL type \"Other\""));
        assert!(rendered.contains("requires an owned field"));
        let mut observed = error
            .diagnostics
            .iter()
            .map(|diagnostic| {
                assert!(!diagnostic.labels.is_empty());
                assert_eq!(diagnostic.labels[0].span.path(), "proof/errors.ncidl");
                (diagnostic.code.clone(), diagnostic.message.clone())
            })
            .collect::<Vec<_>>();
        observed.sort();
        if let Some(previous) = &previous {
            assert_eq!(previous, &observed);
        }
        previous = Some(observed);
    }
    for (reference, expected) in [("missing::module::Local", "EIDL071"), ("proof::other::Local", "EIDL072")] {
        for unused in ["", "#[version(1)] struct Unused<T> { value:T }"] {
            let source = format!("#[version(1)] struct Holder {{ value: {reference} }} {unused}");
            let error = resolve_modules(vec![
                module("proof/errors.ncidl", &source),
                module("proof/other.ncidl", "#[version(1)] struct Local { value:u32 }"),
            ])
            .unwrap_err();
            let neocogi_idl::ModuleGraphError::Resolution(error) = error else {
                panic!("expected semantic diagnostics")
            };
            assert_eq!(error.diagnostics[0].code, expected);
        }
    }
}

/// Callable IDs have the same admission rule for concrete and unused templates.
#[test]
fn nil_callable_ids_are_rejected_before_specialization_or_generation() {
    for parameters in ["", "<T>"] {
        let source = format!("#[id(1, \"00000000-0000-0000-0000-000000000000\")] interface Invalid{parameters} {{}}");
        let error = resolve_modules(vec![module("proof/nil.ncidl", &source)]).unwrap_err();
        let neocogi_idl::ModuleGraphError::Resolution(error) = error else {
            panic!("expected semantic diagnostics")
        };
        assert_eq!(error.diagnostics[0].code, "EIDL074");
        assert!(error.to_string().contains("a callable interface ID must not be nil"));
        assert!(error.to_string().contains("proof/nil.ncidl"));
    }
    // Nominal value identity remains a separate contract from callable IIDs.
    resolve_modules(vec![module(
        "proof/nil_value.ncidl",
        "#[id(1, \"00000000-0000-0000-0000-000000000000\")] struct Value { number:u32 }",
    )])
    .unwrap();
}

/// Builds the same genuine provider/consumer graph with optional unrelated projections.
fn projection_graph(bystander: bool) -> ResolvedModules {
    let mut modules = vec![
        module(
            "demo/ext.ncidl",
            "#[version(1)] struct Inner<T> { value:T } #[version(1)] struct Boxed<T> { nested:Inner<T> }",
        ),
        module(
            "demo/mid.ncidl",
            r#"
            extern demo.ext;
            #[version(1)] interface IBase { fn get(&self) -> Result<demo::ext::Boxed<u32>>; }
            #[version(1)] interface Fact { fn value(&self) -> Result<demo::ext::Boxed<u32>>; }
        "#,
        ),
        module(
            "demo/leaf.ncidl",
            "extern demo.mid; #[version(1)] interface IChild: demo::mid::IBase + demo::mid::Fact {}",
        ),
    ];
    if bystander {
        modules.push(module(
            "aaa/other.ncidl",
            "extern demo.ext; #[version(1)] interface Other { fn get(&self) -> Result<demo::ext::Boxed<u32>>; }",
        ));
    }
    resolve_modules(modules).unwrap()
}

/// Metadata provenance and inherited calls work with only the genuine dependencies.
#[test]
fn inherited_value_metadata_keeps_the_supplying_projection_without_bystander_dependencies() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let graph = projection_graph(true);
    let without = projection_graph(false);
    assert_eq!(
        graph.module("demo.leaf").unwrap().abi_fingerprint,
        without.module("demo.leaf").unwrap().abi_fingerprint
    );
    let mut paths = neocogi_idl::generate::RustModulePaths::default();
    for name in ["demo.ext", "demo.mid", "demo.leaf"] {
        paths.insert(name, &format!("binding_peers::{}", name.split('.').next_back().unwrap())).unwrap();
    }
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        format!(
            "[package]\nname='binding-peers'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            root.join("crates/api")
        ),
    )
    .unwrap();
    for module in ["ext", "mid", "leaf"] {
        let generated = neocogi_idl::generate::generate_rust_with_paths(&graph, &format!("demo.{module}"), &paths).unwrap();
        assert!(!generated.contains("aaa_other_contracts"));
        fs::write(project.path().join(format!("src/{module}.rs")), generated).unwrap();
    }
    let name = |template: &str| {
        graph
            .module("demo.mid")
            .unwrap()
            .source
            .declarations
            .iter()
            .find(|declaration| {
                declaration
                    .specialization
                    .as_ref()
                    .is_some_and(|origin| origin.template.segments.last().is_some_and(|name| name == template))
            })
            .unwrap()
            .name
            .clone()
    };
    let boxed = name("Boxed");
    let inner = name("Inner");
    fs::write(
        project.path().join("src/lib.rs"),
        format!(
            r#"
        extern crate self as binding_peers;
        pub mod ext; pub mod mid; pub mod leaf;
        #[neocogi_com::com_object(leaf::IChild)] struct Provider;
        impl leaf::IChild for Provider {{}}
        impl mid::Fact for Provider {{
            fn value(&self) -> Result<mid::{boxed},neocogi_com::NcStatus> {{ Ok(mid::{boxed} {{ nested:mid::{inner} {{ value:91 }} }}) }}
        }}
        impl mid::IBase for Provider {{
            fn get(&self) -> Result<mid::{boxed}, neocogi_com::NcStatus> {{ Ok(mid::{boxed} {{ nested:mid::{inner} {{ value:73 }} }}) }}
        }}
        #[neocogi_com::com_object(interfaces(mid::Fact), ownership=exclusive)] struct Snapshot;
        impl mid::Fact for Snapshot {{
            fn value(&self) -> Result<mid::{boxed},neocogi_com::NcStatus> {{ Ok(mid::{boxed} {{ nested:mid::{inner} {{ value:91 }} }}) }}
        }}
        #[test] fn supplying_metadata_and_real_calls() {{
            let values = leaf::IChildAbi::IDL_INTERFACE.values;
            assert_eq!(values.len(),2);
            for value in values {{ assert!(value.rust_type.starts_with("binding_peers::mid::"),"{{}}",value.rust_type); }}
            assert_eq!(Provider::from_value(Provider).query::<leaf::IChildAbi>().unwrap().get().unwrap().nested.value,73);
            assert_eq!(Snapshot::into_box(Snapshot).query::<mid::FactAbi>().unwrap().value().unwrap().nested.value,91);
            assert_eq!(leaf::IChildAbi::IDL_INTERFACE.methods.len(),2);
        }}
    "#
        ),
    )
    .unwrap();
    let output = Command::new("cargo")
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
    assert!(String::from_utf8_lossy(&output.stdout).contains("supplying_metadata_and_real_calls ... ok"));
    assert!(matches!(
        graph.module("demo.leaf").unwrap().source.declarations[0].kind,
        DeclarationKind::Interface(_)
    ));
}
