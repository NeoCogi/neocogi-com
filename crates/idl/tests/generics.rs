//! Closed generic contracts must agree across independent consuming modules.
use std::path::Path;
use neocogi_idl::{DeclarationKind, InterfaceRole, Module, ResolvedModules, parse_module, resolve_modules};

/// Gives an inline contract its actual logical module identity.
fn module(path: &str, source: &str) -> Module {
    parse_module(Path::new(path), source).unwrap()
}

/// Shared generic contracts have no knowledge of widgets or application messages.
fn templates() -> Module {
    module(
        "test/transport.ncidl",
        r#"
        #[version(1)] interface ISink<T> { fn post(&mut self, message: &T) -> Result<()>; }
        #[version(1)] interface IStream<T> { fn attach(&mut self, sink: &ISink<T>) -> Result<()>; }
    "#,
    )
}

/// An event lives independently of the modules that instantiate its stream.
fn messages() -> Module {
    module("test/messages.ncidl", "#[version(1)] #[message] enum Event { Changed(u32), Submitted }")
}

/// Builds a concrete class with generic sibling interfaces and a constructor.
fn consumer(name: &str, extra: &str) -> Module {
    module(
        &format!("test/{name}.ncidl"),
        &format!(
            r#"
        extern test.transport;
        extern test.messages;
        use test.transport::IStream;
        use test.messages::Event;
        #[version(1)] interface IObject {{ fn value(&self) -> Result<u32>; }}
        #[version(1)] class Object: IObject + IStream<Event> {{}}
        #[version(1)] factory Factory {{ fn create(&self) -> Result<Object>; }}
        {extra}
    "#
        ),
    )
}

/// Extracts semantic identities without depending on generated specialization names.
fn instances(graph: &ResolvedModules, owner: &str) -> Vec<(String, neocogi_com_core::NcGuid, [u8; 32])> {
    let module = graph.module(owner).unwrap();
    module
        .source
        .declarations
        .iter()
        .zip(&module.declarations)
        .filter(|(source, _)| source.specialization.is_some())
        .map(|(_, identity)| (identity.qualified_name.clone(), identity.id, identity.abi_signature))
        .collect()
}

/// Separate compilation and unrelated consumer declarations cannot change a closed contract.
#[test]
fn independent_consumers_share_specialization_identity_and_signature() {
    let first = resolve_modules(vec![templates(), messages(), consumer("first", "")]).unwrap();
    let second = resolve_modules(vec![consumer("second", "#[version(1)] struct Extra { count: u64 }"), messages(), templates()]).unwrap();
    assert_eq!(instances(&first, "test.first"), instances(&second, "test.second"));
    assert_eq!(instances(&first, "test.first").len(), 2);
    let together = resolve_modules(vec![templates(), messages(), consumer("first", ""), consumer("second", "")]).unwrap();
    assert_eq!(instances(&together, "test.first"), instances(&together, "test.second"));
    for owner in ["test.transport", "test.first", "test.second"] {
        let rust = neocogi_idl::generate_rust(&together, owner).unwrap();
        syn::parse_file(&rust).expect("closed bindings are ordinary Rust interfaces");
    }
    let object = first.module("test.first").unwrap().source.declaration("Object").unwrap();
    assert!(matches!(&object.kind, DeclarationKind::Interface(interface) if interface.role == InterfaceRole::Class));
}

/// The full event ABI, including nested field changes, participates in stream negotiation.
#[test]
fn changing_argument_layout_changes_signature_but_not_nominal_identity() {
    let first = resolve_modules(vec![templates(), messages(), consumer("first", "")]).unwrap();
    let changed = module("test/messages.ncidl", "#[version(1)] #[message] enum Event { Changed(u64), Submitted }");
    let second = resolve_modules(vec![templates(), changed, consumer("first", "")]).unwrap();
    for ((_, left_id, left_sig), (_, right_id, right_sig)) in instances(&first, "test.first").into_iter().zip(instances(&second, "test.first")) {
        assert_eq!(left_id, right_id);
        assert_ne!(left_sig, right_sig);
    }
}

/// Invalid argument counts and ABI positions fail before generating a COM vtable.
#[test]
fn rejects_open_and_invalid_specializations() {
    for reference in ["IStream", "IStream<Event, Event>", "IStream<u32>"] {
        let source = module(
            "test/invalid.ncidl",
            &format!(
                r#"
            extern test.transport; extern test.messages;
            use test.transport::IStream; use test.messages::Event;
            #[version(1)] class Invalid: {reference} {{}}
        "#
            ),
        );
        assert!(resolve_modules(vec![templates(), messages(), source]).is_err(), "{reference} must fail");
    }
    assert!(parse_module(Path::new("test/invalid.ncidl"), "#[version(1)] class Object<T> {}").is_err());
}

/// Ordinary ABI-safe values also work; generics are not an event-language feature.
#[test]
fn generic_interface_accepts_scalar_arguments_in_valid_positions() {
    let source = module(
        "test/scalars.ncidl",
        r#"
        #[version(1)] interface IValue<T> { fn get(&self) -> Result<T>; fn set(&mut self, value: T) -> Result<()>; }
        #[version(1)] class Counter: IValue<u32> {}
    "#,
    );
    let graph = resolve_modules(vec![source]).unwrap();
    let rust = neocogi_idl::generate_rust(&graph, "test.scalars").unwrap();
    assert!(rust.contains("value: u32"));
}

/// Two independently generated projections can query and call the same COM object.
#[test]
fn closed_generic_calls_cross_generated_module_boundaries() {
    use std::{fs, process::Command};
    let template = module(
        "proof/contracts.ncidl",
        "#[version(1)] interface IValue<T> { fn get(&self) -> Result<T>; fn set(&mut self, value: T) -> Result<()>; }",
    );
    let source = "extern proof.contracts; use proof.contracts::IValue; #[version(1)] class Counter: IValue<u32> {}";
    let graph = resolve_modules(vec![template, module("proof/producer.ncidl", source), module("proof/consumer.ncidl", source)]).unwrap();
    let owner = graph.module("proof.producer").unwrap();
    let contract = &owner.source.declarations.iter().find(|source| source.specialization.is_some()).unwrap().name;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        format!(
            "[package]\nname='generic-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            root.join("crates/com/api")
        ),
    )
    .unwrap();
    for name in ["producer", "consumer"] {
        fs::write(
            project.path().join(format!("src/{name}.rs")),
            neocogi_idl::generate_rust(&graph, &format!("proof.{name}")).unwrap(),
        )
        .unwrap();
    }
    fs::write(
        project.path().join("src/lib.rs"),
        format!(
            r#"
        pub mod producer;
        pub mod consumer;
        #[neocogi_com::com_object(producer::Counter)]
        /// Concrete storage implements one generated specialization.
        struct Counter {{ value: u32 }}
        impl producer::Counter for Counter {{}}
        impl producer::{contract} for Counter {{
            /// Reads ordinary exclusively owned state through COM.
            fn get(&self) -> Result<u32, neocogi_com::NcStatus> {{ Ok(self.value) }}
            /// Mutates the same state through either independently generated client.
            fn set(&mut self, value: u32) -> Result<(), neocogi_com::NcStatus> {{ self.value = value; Ok(()) }}
        }}
        #[test]
        fn calls_peer_specialization() {{
            let owner = Counter::from_value(Counter {{ value: 7 }});
            let mut peer = owner.query::<consumer::{contract}Abi>().unwrap();
            assert_eq!(peer.get(), Ok(7));
            peer.set(41).unwrap();
            assert_eq!(owner.query::<producer::{contract}Abi>().unwrap().get(), Ok(41));
            assert!(owner.query::<consumer::CounterAbi>().is_err());
        }}
    "#
        ),
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
