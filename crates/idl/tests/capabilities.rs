//! Requested operations must be validated on closed recursive value definitions.
use std::{fs, path::Path, process::Command};
use neocogi_idl::{parse_module, resolve_modules};

/// The original finding and its nested/generic forms must fail in IDL, not rustc.
#[test]
fn unsupported_deserialization_reports_the_missing_field_operation() {
    let prefix = "#[version(1)] interface IToken {} ";
    for (source, reason) in [
        (
            "#[version(1)] #[deserialize] struct Snapshot { token: ComPtr<IToken> }",
            "object reconstruction contract",
        ),
        (
            "#[version(1)] #[deserialize] struct Snapshot { tokens: Vec<Option<ComPtr<IToken>>> }",
            "object reconstruction contract",
        ),
        (
            "#[version(1)] #[deserialize] struct Snapshot { tokens: [ComPtr<IToken>; 2] }",
            "object reconstruction contract",
        ),
        (
            "#[version(1)] #[deserialize] struct Data<T> { value: T } #[version(1)] struct Snapshot { data: Data<ComPtr<IToken>> }",
            "object reconstruction contract",
        ),
        (
            "#[version(1)] #[deserialize] struct Node<T> { children: Vec<Node<T>>, value: T } #[version(1)] struct Snapshot { data: Node<ComPtr<IToken>> }",
            "object reconstruction contract",
        ),
        (
            "#[version(1)] struct Child { count: u32 } #[version(1)] #[deserialize] struct Snapshot { child: Option<Child> }",
            "does not declare a generated decoder",
        ),
        ("#[version(1)] #[deserialize] struct Snapshot { id: Guid }", "Guid has no format-neutral"),
        (
            "#[version(1)] #[deserialize] struct Snapshot { status: Status }",
            "Status has no format-neutral",
        ),
        (
            "#[version(1)] enum Mode { First, Second } #[version(1)] #[deserialize] struct Snapshot { mode: Mode }",
            "referenced records/enums",
        ),
        (
            "#[version(1)] bitenum Flags: u8 { A = 1 } #[version(1)] #[deserialize] struct Snapshot { flags: Flags }",
            "referenced records/enums",
        ),
    ] {
        let module = parse_module(Path::new("proof/operations.ncidl"), &format!("{prefix}{source}")).unwrap();
        let error = resolve_modules(vec![module]).expect_err("requested decoder must be rejected");
        let diagnostic = error.to_string();
        assert!(diagnostic.contains("EIDL064"), "{source}: {diagnostic}");
        assert!(diagnostic.contains("operations.ncidl"), "the field's source must be identified: {diagnostic}");
        assert!(diagnostic.contains(reason), "{source}: {diagnostic}");
    }
}

/// An unannotated owning value remains valid for live COM transport.
#[test]
fn transport_and_local_deserialization_are_separate_capabilities() {
    let source = r#"
        #[version(1)] interface IToken {}
        #[version(1)] struct Snapshot { token: ComPtr<IToken> }
        #[version(1)] interface IInput { fn accept(&mut self, value: &Snapshot) -> Result<()>; }
        #[version(1)] #[deserialize] struct Unused<T> { value: T }
    "#;
    let module = parse_module(Path::new("proof/operations.ncidl"), source).unwrap();
    let graph = resolve_modules(vec![module]).unwrap();
    let snapshot = &graph.values.definitions["proof.operations.Snapshot"];
    assert!(!snapshot.copied);
    assert_eq!(snapshot.deserialization, neocogi_idl::Deserialization::NotRequested);
    neocogi_idl::generate_rust(&graph, "proof.operations").unwrap();
}

/// Proves Rust trait availability and recursive decoding for accepted closed policies.
#[test]
fn accepted_generic_deserializers_compile_and_reconstruct_values() {
    let source = r#"
        #[version(1)] #[deserialize] struct Node<T> { value: T, children: Vec<Node<T>> }
        #[version(1)] #[deserialize] struct Flags { enabled: bool }
        #[version(1)] #[deserialize(sequence)] struct Pair<T> { first: T, second: T }
        #[version(1)] #[deserialize] struct Snapshot {
            root: Node<String>, optional: Option<Flags>, mask: [bool; 3], pair: Pair<u32>, units: Vec<()>,
        }
        #[version(1)] #[deserialize(default)] struct Config { enabled: bool }
    "#;
    let module = parse_module(Path::new("proof/operations.ncidl"), source).unwrap();
    let graph = resolve_modules(vec![module]).unwrap();
    assert!(graph
        .values
        .definitions
        .values()
        .all(|value| value.deserialization == neocogi_idl::Deserialization::Supported));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        format!(
            "[package]\nname='operation-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            root.join("crates/com/api")
        ),
    )
    .unwrap();
    fs::write(
        project.path().join("src/contract.rs"),
        neocogi_idl::generate_rust(&graph, "proof.operations").unwrap(),
    )
    .unwrap();
    fs::write(
        project.path().join("src/lib.rs"),
        r#"
        pub mod contract;
        /// The explicit `default` policy uses the host's authored defaults.
        impl Default for contract::Config {
            fn default() -> Self { Self { enabled: true } }
        }
        #[derive(neocogi_com::ComSerialize)]
        /// A finite tree with the same schema as the generated closed Node<String>.
        struct Node { value: String, children: Vec<Node> }
        #[derive(neocogi_com::ComSerialize)]
        /// A named inner record used through an optional field.
        struct Flags { enabled: bool }
        #[derive(neocogi_com::ComSerialize)]
        /// Independent writer exercises generated record and sequence decoders.
        struct Snapshot { root: Node, optional: Option<Flags>, mask: [bool; 3], pair: [u32; 2], units: Vec<()> }
        #[derive(neocogi_com::ComSerialize)]
        /// Writes an empty named record through the actual format encoder.
        struct EmptyRecord {}
        #[test]
        fn reconstructs_the_complete_closed_value() {
            let source = Snapshot {
                root: Node { value: "parent".into(), children: vec![Node { value: "child".into(), children: vec![] }] },
                optional: Some(Flags { enabled: true }), mask: [false, true, false], pair: [7, 9], units: vec![(), ()],
            };
            let bytes = neocogi_com::binary::to_vec(&source).unwrap();
            let decoded: contract::Snapshot = neocogi_com::binary::from_slice(&bytes).unwrap();
            assert_eq!(decoded.root.children[0].value, "child");
            assert!(decoded.optional.unwrap().enabled);
            assert_eq!(decoded.mask, [false, true, false]);
            assert_eq!((decoded.pair.first, decoded.pair.second), (7, 9));
            assert_eq!(decoded.units.len(), 2);
            // An empty record invokes the explicitly supplied custom Default.
            let empty = neocogi_com::binary::to_vec(&EmptyRecord {}).unwrap();
            let defaults: contract::Config = neocogi_com::binary::from_slice(&empty).unwrap();
            assert!(defaults.enabled);
        }
    "#,
    )
    .unwrap();
    let output = Command::new("cargo")
        .args(["test", "--offline", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target/message-proof"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A generic policy failure identifies both the template field and its closed use.
#[test]
fn imported_decoder_failure_preserves_both_source_locations() {
    let template = parse_module(Path::new("proof/template.ncidl"), "#[version(1)] #[deserialize] struct Data<T> { value: T }").unwrap();
    let consumer = parse_module(
        Path::new("proof/consumer.ncidl"),
        "extern proof.template; use proof.template::Data; #[version(1)] struct Snapshot { data: Data<ComPtr<IUnknown>> }",
    )
    .unwrap();
    let diagnostic = resolve_modules(vec![consumer, template]).unwrap_err().to_string();
    assert!(diagnostic.contains("template.ncidl"), "{diagnostic}");
    assert!(diagnostic.contains("consumer.ncidl"), "{diagnostic}");
    assert!(diagnostic.contains("Data<ComPtr<IUnknown>>"), "{diagnostic}");
    assert!(diagnostic.contains("decoder required for this closed specialization"), "{diagnostic}");
}
