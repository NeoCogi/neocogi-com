//! Enum persistence uses the ordinary variant protocol for copied and owned data.
use std::{fs, path::Path, process::Command};
use neocogi_idl::{parse_module, resolve_modules};

/// Compiles all payload shapes, recursion, and conditional generic capabilities.
#[test]
fn ordinary_enum_decoders_compile_and_reconstruct_every_payload_shape() {
    let source = r#"
        #[version(1)]  enum Mode: u8 { First = 3, Second = 9 }
        #[version(1)]  enum Packet { Empty, Flag(bool), Pair(u8, u16), Values { flags: [bool; 3] } }
        #[version(1)]  enum Owned { Empty, Text(String), Pair(String, String), Values { values: Vec<u32> } }
        #[version(1)]  enum Tree { Leaf(u32), Nodes(Vec<Tree>) }
        #[version(1)]  enum Maybe<T> { None, Some(T) }
        #[version(1)]  struct Number { value: Maybe<u32> }
        // These former public companions are now ordinary authored names.
        #[version(1)] struct PacketAbi { value: Packet }
        #[version(1)] struct PacketTag { value: u32 }
        #[version(1)] struct PacketPayload { value: u32 }
        #[version(1)] struct PacketFlagPayload { value: u32 }
        #[version(1)] interface IToken {}
        #[version(1)] struct Live { value: Maybe<ComPtr<IToken>> }
        #[version(1)] bitenum Flags: u16 { ONE = 1, TWO = 2 }
        #[version(1)] struct Scalars { id: Guid, status: Status, flags: Flags, mode: Mode }
        #[version(1)] struct Position { x: i32, y: i32 }
    "#;
    let graph = resolve_modules(vec![parse_module(Path::new("proof/enums.ncidl"), source).unwrap()]).unwrap();
    assert_eq!(graph.values.definitions["proof.enums.Number"].persistence, neocogi_idl::Persistence::Supported);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        format!(
            "[package]\nname='enum-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            root.join("crates/com")
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
    for capability in ["ComSerialize", "ComDeserialize"] {
        fs::write(
            project.path().join("src/main.rs"),
            format!("mod contract; fn requires<T: neocogi_com::{capability}>() {{}} fn main() {{ requires::<contract::Live>(); }}"),
        )
        .unwrap();
        let rejected = Command::new("cargo")
            .args(["check", "--offline", "--bin", "enum-proof", "--manifest-path"])
            .arg(project.path().join("Cargo.toml"))
            .env(
                "CARGO_TARGET_DIR",
                std::env::var_os("CARGO_TARGET_DIR").unwrap_or_else(|| root.join("target").into_os_string()),
            )
            .output()
            .unwrap();
        let errors = String::from_utf8_lossy(&rejected.stderr);
        assert!(!rejected.status.success(), "live references must not acquire {capability}");
        assert!(errors.contains(capability) && errors.contains("Live"), "{errors}");
    }
}

/// Decoder opt-ins are retired, and record format choices never apply to enums.
#[test]
fn rejects_obsolete_opt_ins_and_inapplicable_format_policies() {
    for attribute in [
        "#[deserialize]",
        "#[deserialize(if_supported)]",
        "#[persistence(default)]",
        "#[persistence(sequence)]",
    ] {
        let source = format!("#[version(1)] {attribute} enum Wrong {{ Value(u32) }}");
        assert!(parse_module(Path::new("proof/enums.ncidl"), &source).is_err());
    }
    for policy in ["sequence", "if_supported", "default, sequence", ""] {
        let source = format!("#[version(1)] #[persistence({policy})] struct Wrong {{ value: u32 }}");
        assert!(parse_module(Path::new("proof/enums.ncidl"), &source).is_err());
    }
}
