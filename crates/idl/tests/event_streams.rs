//! End-to-end proof of the ordinary generic COM stream contract.
use std::{fs, path::Path, process::Command};
use neocogi_idl::{parse_module, resolve_modules, generate_rust};

/// Compiles the actual generated ABI and exercises attachment through queried projections.
#[test]
fn class_stream_maps_to_parent_queue_and_never_replaces_consumer() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let templates = fs::read_to_string(root.join("crates/com/api/idl/neocogi/com/events/events.ncidl")).unwrap();
    let source = r#"
        extern neocogi.com.events; use neocogi.com.events::IEventStream;
        #[version(1)] #[message] enum Event { Changed(String), Submitted }
        #[version(1)] interface ISource: IEventStream<Event> {}
        #[version(1)] class Counter: ISource {
            fn emit(&mut self, value: &Event) -> Result<()>;
            fn close(&mut self) -> Result<()>;
        }
    "#;
    let graph = resolve_modules(vec![
        parse_module(Path::new("neocogi/com/events.ncidl"), &templates).unwrap(),
        parse_module(Path::new("proof/emitter.ncidl"), source).unwrap(),
    ]).unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(project.path().join("Cargo.toml"), format!(
        "[package]\nname='stream-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n", root.join("crates/com/api")
    )).unwrap();
    fs::write(project.path().join("src/contract.rs"), generate_rust(&graph, "proof.emitter").unwrap()).unwrap();
    fs::write(project.path().join("src/lib.rs"), r#"
        pub mod contract;
        use neocogi_com::*;
        #[com_class(contract::Counter, event = events)]
        struct Counter { events: contract::CounterEventPort }
        impl contract::ISource for Counter {}
        impl contract::Counter for Counter {
            fn emit(&mut self, value: NcMessage<'_, contract::Event>) -> Result<(), NcStatus> {
                self.events.post(&value.to_owned()?)
            }
            fn close(&mut self) -> Result<(), NcStatus> { self.events.close(); Ok(()) }
        }
        #[test]
        fn one_lifetime_consumer() {
            let owner = Counter::from_value(Counter { events: Default::default() });
            let mut source = owner.query::<contract::CounterAbi>().unwrap();
            let (tx, rx) = std::sync::mpsc::channel();
            events::bind_source(source.as_com_ref(), tx, |event| match event {
                contract::Event::Changed(text) => text,
                contract::Event::Submitted => "submitted".to_owned(),
            }).unwrap();
            let (other_tx, other_rx) = std::sync::mpsc::channel();
            let other = owner.query::<contract::ISourceAbi>().unwrap();
            assert_eq!(events::bind_source(other.as_com_ref(), other_tx, |_| ()), Err(NC_E_ALREADY_BOUND));
            let text = contract::Event::Changed("owned after call".to_owned());
            source.emit(NcMessage::new(&text)).unwrap();
            drop(text);
            assert_eq!(rx.recv().unwrap(), "owned after call");
            assert!(other_rx.try_recv().is_err());
            source.close().unwrap();
            let (tx, _) = std::sync::mpsc::channel();
            assert_eq!(events::bind_source(source.as_com_ref(), tx, |_| ()), Err(NC_E_CLOSED));
            assert_eq!(source.emit(NcMessage::new(&contract::Event::Submitted)), Err(NC_E_CLOSED));
        }
    "#).unwrap();
    let output = Command::new("cargo").args(["test", "--offline", "--manifest-path"])
        .arg(project.path().join("Cargo.toml")).env("CARGO_TARGET_DIR", root.join("target/message-proof")).output().unwrap();
    assert!(output.status.success(), "{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
}

/// Concrete classes may not accumulate different event streams through their parents.
#[test]
fn rejects_two_transitive_stream_types_on_one_class() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let templates = fs::read_to_string(root.join("crates/com/api/idl/neocogi/com/events/events.ncidl")).unwrap();
    let source = r#"
        extern neocogi.com.events; use neocogi.com.events::IEventStream;
        #[version(1)] #[message] enum A { A }
        #[version(1)] #[message] enum B { B }
        #[version(1)] interface IA: IEventStream<A> {}
        #[version(1)] interface IB: IEventStream<B> {}
        #[version(1)] class Invalid: IA + IB {}
    "#;
    let graph = resolve_modules(vec![parse_module(Path::new("neocogi/com/events.ncidl"), &templates).unwrap(), parse_module(Path::new("proof/invalid.ncidl"), source).unwrap()]).unwrap();
    assert!(generate_rust(&graph, "proof.invalid").unwrap_err().to_string().contains("multiple event streams"));
}
