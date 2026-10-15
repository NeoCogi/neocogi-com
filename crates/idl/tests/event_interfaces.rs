//! Common IDL event transport preserves typed snapshots and both weak ownership kinds.
use neocogi_idl::{parse_module, resolve_modules};
use std::{fs, path::Path, process::Command};

/// Compile the real common receiver plus independently declared event interfaces.
#[test]
fn common_receiver_captures_typed_events_without_owning_their_origins() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let common = fs::read_to_string(root.join("crates/com/api/idl/neocogi/com/events/events.ncidl")).unwrap();
    let source = r#"
        extern neocogi.com.events;
        use neocogi.com.events::IEvent;
        #[version(1)] interface Widget { fn value(&self) -> Result<u32>; }
        #[version(1)] interface Service { fn value(&self) -> Result<u32>; }
        #[version(1)] interface IChanged: IEvent { fn value(&self) -> Result<u32>; }
        #[version(1)] interface ISubmitted: IEvent { fn text(&self) -> Result<ComPtr<INcString>>; }
        #[version(1)] struct Origins {
            one: WeakComBox<Widget>,
            many: Vec<WeakComBox<Widget>>,
            fixed: [WeakComBox<Widget>; 2],
        }
        #[version(1)] interface IOrigins {
            fn capture(&mut self, origins: &Origins) -> Result<()>;
            fn value(&self) -> Result<WeakComBox<Widget>>;
        }
    "#;
    let graph = resolve_modules(vec![
        parse_module(Path::new("neocogi/com/events.ncidl"), &common).unwrap(),
        parse_module(Path::new("proof/events.ncidl"), source).unwrap(),
    ])
    .unwrap();
    let origins = &graph.values.definitions["proof.events.Origins"];
    assert!(!origins.copied);
    assert_eq!(origins.persistence, neocogi_idl::Persistence::Unsupported);
    assert_eq!(origins.fields[0], neocogi_idl::ValueType::WeakBoxObject("proof.events.Widget".into()));
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        format!(
            "[package]\nname='event-interfaces-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            root.join("crates/com/api"),
        ),
    )
    .unwrap();
    let mut paths = neocogi_idl::generate::RustModulePaths::default();
    paths.insert("neocogi.com.events", "neocogi_com::events").unwrap();
    fs::write(
        project.path().join("src/lib.rs"),
        neocogi_idl::generate::generate_rust_with_paths(&graph, "proof.events", &paths).unwrap() + include_str!("fixtures/event_interfaces.rs"),
    )
    .unwrap();
    let output = Command::new("cargo")
        .args(["test", "--offline", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target/event-interfaces-proof"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("5 passed"), "generated ABI tests must execute");
}

/// Exclusive observers are repeatable, but owners and persistence remain restricted.
#[test]
fn weak_box_fields_reject_invalid_targets_and_persistence() {
    for body in [
        "#[version(1)] struct Bad { value: WeakComBox<u32> }",
        "#[version(1)] struct Bad { value: WeakComBox<ComPtr<IValue>> }",
        "#[version(1)] #[persistence(default)] struct Bad { value: WeakComBox<IValue> }",
        "#[version(1)] struct Bad { value: ComBox<IValue> }",
    ] {
        let source = format!("#[version(1)] interface IValue {{}} {body}");
        let parsed = parse_module(Path::new("proof/invalid.ncidl"), &source).unwrap();
        assert!(resolve_modules(vec![parsed]).is_err(), "accepted {body}");
    }
}
