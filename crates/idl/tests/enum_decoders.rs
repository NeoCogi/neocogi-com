//! Enum persistence uses the ordinary variant protocol for copied and owned data.
use std::{fs, path::Path, process::Command};
use neocogi_idl::{parse_module, resolve_modules};

/// Compiles all payload shapes, recursion, and conditional generic capabilities.
#[test]
fn ordinary_enum_decoders_compile_and_reconstruct_every_payload_shape() {
    let source = r#"
        #[version(1)] #[deserialize] enum Mode: u8 { First = 3, Second = 9 }
        #[version(1)] #[deserialize] enum Packet { Empty, Flag(bool), Pair(u8, u16), Values { flags: [bool; 3] } }
        #[version(1)] #[deserialize] enum Owned { Empty, Text(String), Pair(String, String), Values { values: Vec<u32> } }
        #[version(1)] #[deserialize] enum Tree { Leaf(u32), Nodes(Vec<Tree>) }
        #[version(1)] #[deserialize(if_supported)] enum Maybe<T> { None, Some(T) }
        #[version(1)] #[deserialize] struct Number { value: Maybe<u32> }
        // These former public companions are now ordinary authored names.
        #[version(1)] struct PacketAbi { value: Packet }
        #[version(1)] struct PacketTag { value: u32 }
        #[version(1)] struct PacketPayload { value: u32 }
        #[version(1)] struct PacketFlagPayload { value: u32 }
        #[version(1)] interface IToken {}
        #[version(1)] struct Live { value: Maybe<ComPtr<IToken>> }
    "#;
    let graph = resolve_modules(vec![parse_module(Path::new("proof/enums.ncidl"), source).unwrap()]).unwrap();
    assert_eq!(
        graph.values.definitions["proof.enums.Number"].deserialization,
        neocogi_idl::Deserialization::Supported
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        format!(
            "[package]\nname='enum-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            root.join("crates/com/api")
        ),
    )
    .unwrap();
    fs::write(
        project.path().join("src/contract.rs"),
        neocogi_idl::generate_rust(&graph, "proof.enums").unwrap(),
    )
    .unwrap();
    fs::write(project.path().join("src/lib.rs"), include_str!("fixtures/enum_decoders.rs")).unwrap();
    let result = Command::new("cargo")
        .args(["test", "--offline", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target/message-proof"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

/// Optional library capabilities must not turn a required caller decoder into a promise it cannot meet.
#[test]
fn required_decoders_reject_unsupported_conditional_dependencies() {
    for leaf in ["ComPtr<IUnknown>", "Guid"] {
        let source = format!(
            "#[version(1)] #[deserialize(if_supported)] enum Maybe<T> {{ None, Some(T) }} #[version(1)] #[deserialize] enum Required {{ Value(Maybe<{leaf}>) }}"
        );
        let error = resolve_modules(vec![parse_module(Path::new("proof/enums.ncidl"), &source).unwrap()])
            .unwrap_err()
            .to_string();
        assert!(error.contains("EIDL064"), "{error}");
    }
    for policy in ["default", "sequence", "default, sequence"] {
        let source = format!("#[version(1)] #[deserialize({policy})] enum Wrong {{ Value(u32) }}");
        assert!(parse_module(Path::new("proof/enums.ncidl"), &source).is_err());
    }
}
