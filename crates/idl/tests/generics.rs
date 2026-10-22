//! Closed generic contracts must agree across independent consuming modules.
use std::path::Path;
use neocogi_idl::{generate_rust, DeclarationKind, Module, ResolvedModules, parse_module, resolve_modules};

/// Gives an inline contract its actual logical module identity.
fn module(path: &str, source: &str) -> Module {
    parse_module(Path::new(path), source).unwrap()
}

/// Shared generic contracts have no knowledge of widgets or application messages.
fn templates() -> Module {
    module(
        "test/transport.ncidl",
        r#"
        #[version(1)] interface IReceiver<T> { fn post(&mut self, message: &T) -> Result<()>; }
        #[version(1)] interface IStream<T> { fn attach(&mut self, queue: &IReceiver<T>) -> Result<()>; }
    "#,
    )
}

/// An event lives independently of the modules that instantiate its stream.
fn messages() -> Module {
    module("test/messages.ncidl", "#[version(1)] enum Event { Changed(u32), Submitted }")
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
        #[version(1)] interface Object: IObject + IStream<Event> {{}}
        #[version(1)] interface Factory {{ fn create(&self) -> Result<ComPtr<Object>>; }}
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
        let rust = generate_rust(&together, owner).unwrap();
        syn::parse_file(&rust).expect("closed bindings are ordinary Rust interfaces");
    }
    let object = first.module("test.first").unwrap().source.declaration("Object").unwrap();
    assert!(matches!(&object.kind, DeclarationKind::Interface(_)));
}

/// The full event ABI, including nested field changes, participates in stream negotiation.
#[test]
fn changing_argument_layout_changes_signature_but_not_nominal_identity() {
    let first = resolve_modules(vec![templates(), messages(), consumer("first", "")]).unwrap();
    let changed = module("test/messages.ncidl", "#[version(1)] enum Event { Changed(u64), Submitted }");
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
            #[version(1)] interface Invalid: {reference} {{}}
        "#
            ),
        );
        assert!(resolve_modules(vec![templates(), messages(), source]).is_err(), "{reference} must fail");
    }
    assert!(parse_module(Path::new("test/generic.ncidl"), "#[version(1)] interface Object<T> {}").is_ok());
}

/// Ordinary ABI-safe values also work; generics are not an event-language feature.
#[test]
fn generic_interface_accepts_scalar_arguments_in_valid_positions() {
    let source = module(
        "test/scalars.ncidl",
        r#"
        #[version(1)] interface IValue<T> { fn get(&self) -> Result<T>; fn set(&mut self, value: T) -> Result<()>; }
        #[version(1)] interface Counter: IValue<u32> {}
    "#,
    );
    let graph = resolve_modules(vec![source]).unwrap();
    let rust = generate_rust(&graph, "test.scalars").unwrap();
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
    let source = "extern proof.contracts; use proof.contracts::IValue; #[version(1)] interface Counter: IValue<u32> {}";
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
            generate_rust(&graph, &format!("proof.{name}")).unwrap(),
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
        #[neocogi_com::plugin(name = "proof.closed.generic")]
        /// Closed generic projections are ordinary plugin roots.
        fn acquire_root() -> Result<neocogi_com::ComPtr<producer::{contract}Abi>, neocogi_com::NcStatus> {{
            Counter::from_value(Counter {{ value: 7 }}).query()
        }}
        #[test]
        fn calls_peer_specialization() {{
            let mut raw = neocogi_com::ComRaw::null();
            // SAFETY: the generated adapter receives valid raw output storage.
            assert_eq!(unsafe {{ (plugin_descriptor().acquire.unwrap())(&mut raw) }}, neocogi_com::NC_OK);
            // SAFETY: the adapter transferred the exact generated specialization.
            let owner = unsafe {{ neocogi_com::ComPtr::<producer::{contract}Abi>::__nc_from_raw_owned_checked(raw.cast()) }}.unwrap();
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
}

/// Value templates use the same substitution and layout rules as interface templates.
#[test]
fn generic_values_reject_invalid_closed_layout_and_ownership() {
    for body in [
        "#[version(1)] struct Boxed<T> { value: T } #[version(1)] struct Bad { value: Boxed<IToken> }",
        "#[version(1)] struct Handle<I> { value: ComPtr<I> } #[version(1)] struct Bad { value: Handle<u32> }",
        "#[version(1)] struct Loop<T> { child: Option<Loop<T>> } #[version(1)] struct Bad { value: Loop<u32> }",
        "#[version(1)] struct Boxed<T> { value: T } #[version(1)] struct Bad { value: Boxed }",
        "#[version(1)] enum Choice<T> { Value(T) } #[version(1)] struct Bad { value: Choice<u32, bool> }",
        "#[version(1)] struct Grow<T> { children: Vec<Grow<Option<T>>> } #[version(1)] struct Bad { value: Grow<u32> }",
        "#[version(1)] struct Marker<T> { value: u32 } #[version(1)] struct Bad { value: Marker<ComPtr<u32>> }",
        "#[version(1)] struct Boxed<T> { value: T } #[version(1)] interface IBad { fn get(&self) -> Result<Boxed<String>>; }",
    ] {
        let source = module("proof/bad.ncidl", &format!("#[version(1)] interface IToken {{}} {body}"));
        assert!(resolve_modules(vec![source]).is_err(), "must reject {body}");
    }
    for source in [
        "#[version(1)] struct Invalid<String> { value: String }",
        "#[version(1)] struct Boxed<T, T> { value: T }",
    ] {
        assert!(parse_module(Path::new("proof/bad.ncidl"), source).is_err());
    }
}

/// Independently generated bindings agree after binder renaming and reject changed bindings.
/// Exercises copied calls and recursive enum events through actual checked COM projections.
#[test]
fn parameter_renames_preserve_cross_module_calls_and_event_adapters() {
    use std::{fs, process::Command};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let template_source = r#"
        #[version(1)] struct Pair<T, U> { first: T, second: U }
        #[version(1)] struct Node<T> { value: T, children: Vec<Node<T>> }
        #[version(1)] enum Change<T>: u8 { Set(T), Clear }
        #[version(1)] struct Handle<I> { object: ComPtr<I> }
        #[version(1)] interface IValueReceiver<T> { fn accept(&mut self, value: &T) -> Result<()>; }
        #[version(1)] interface IExchange<T> { fn roundtrip(&self, value: T) -> Result<T>; }
    "#;
    // Each declaration selects its own binder names, including recursive uses.
    // Only spelling changes; the consumer is compiled from a separate graph.
    let renamed_template_source = r#"
        #[version(1)] struct Pair<Left, Right> { first: Left, second: Right }
        #[version(1)] struct Node<Item> { value: Item, children: Vec<Node<Item>> }
        #[version(1)] enum Change<Payload>: u8 { Set(Payload), Clear }
        #[version(1)] struct Handle<Object> { object: ComPtr<Object> }
        #[version(1)] interface IValueReceiver<Item> { fn accept(&mut self, value: &Item) -> Result<()>; }
        #[version(1)] interface IExchange<Value> { fn roundtrip(&self, value: Value) -> Result<Value>; }
    "#;
    let consumer_source = r#"
        extern proof.templates;
        use proof.templates::{Pair, Node, Change, Handle, IExchange, IValueReceiver};

        #[version(1)] interface ExchangeFactory { fn create(&self, events: &Option<u32>) -> Result<ComPtr<Exchange>>; }
        #[version(1)] interface Exchange: IExchange<Pair<bool, bool>> {}
        #[version(1)] interface Receiver: IValueReceiver<Change<Node<String>>> {}
        #[version(1)] interface IData { fn inspect(&self, owner: &Handle<IUnknown>) -> Result<()>; }
    "#;
    // Resolve independently: accidental dependence on the other consumer's
    // generated declarations cannot make this ABI compatibility proof pass.
    let make_graph = |_name: &str, template_source: &str| {
        resolve_modules(vec![
            module("proof/templates.ncidl", template_source),
            module("proof/contract.ncidl", consumer_source),
        ])
        .unwrap()
    };
    let producer = make_graph("producer", template_source);
    let consumer = make_graph("consumer", renamed_template_source);
    // The concrete fields remain bool/bool, but swapping their binding positions
    // changes the template contract and must fail actual interface negotiation.
    let rebound_template_source = renamed_template_source.replace("first: Left, second: Right", "first: Right, second: Left");
    let rebound = make_graph("rebound", &rebound_template_source);
    assert_eq!(instances(&producer, "proof.contract"), instances(&consumer, "proof.contract"));
    assert_eq!(
        producer.values, consumer.values,
        "recursive metadata uses canonical identities, not consumer paths"
    );
    let owner = producer.module("proof.contract").unwrap();
    let instance = |template: &str| {
        owner
            .source
            .declarations
            .iter()
            .find(|declaration| {
                declaration
                    .specialization
                    .as_ref()
                    .is_some_and(|source| source.template.segments.last().unwrap() == template)
            })
            .unwrap()
            .name
            .clone()
    };
    let pair = instance("Pair");
    let node = instance("Node");
    let change = instance("Change");
    let exchange = instance("IExchange");
    let receiver = instance("IValueReceiver");
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        format!(
            "[package]\nname='generic-values-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            root.join("crates/com/api")
        ),
    )
    .unwrap();
    for (name, graph) in [("producer", &producer), ("consumer", &consumer), ("rebound", &rebound)] {
        fs::write(project.path().join(format!("src/{name}.rs")), generate_rust(graph, "proof.contract").unwrap()).unwrap();
    }
    fs::write(
        project.path().join("src/lib.rs"),
        format!(
            r#"
        pub mod producer;
        pub mod consumer;
        pub mod rebound;
        #[neocogi_com::com_object(producer::Exchange)]
        /// A real COM implementation uses its own generated Pair specialization.
        struct Exchange;
        impl producer::Exchange for Exchange {{}}
        impl producer::{exchange} for Exchange {{
            /// Returning the acquired value exercises both generated wire directions.
            fn roundtrip(&self, value: producer::{pair}) -> Result<producer::{pair}, neocogi_com::NcStatus> {{ Ok(value) }}
        }}
        /// Renamed template binders negotiate and round-trip copied boolean fields.
        #[test]
        fn independently_generated_values_share_wire_contract() {{
            let object = Exchange::from_value(Exchange);
            let peer = object.query::<consumer::{exchange}Abi>().unwrap();
            let result = peer.roundtrip(consumer::{pair} {{ first: true, second: false }}).unwrap();
            assert!(result.first);
            assert!(!result.second);
        }}
        /// Equal closed layouts do not erase a changed template binding contract.
        #[test]
        fn changed_parameter_positions_fail_interface_negotiation() {{
            let object = Exchange::from_value(Exchange);
            assert_eq!(object.query::<rebound::{exchange}Abi>().err(), Some(neocogi_com::NC_E_ABI_VERSION));
        }}
        /// A renamed peer receives an owned recursive enum through a real COM queue.
        #[test]
        fn recursive_value_and_generic_event_receiver_acquire_snapshots() {{
            let node = producer::{node} {{ value: "parent".into(), children: vec![producer::{node} {{ value: "child".into(), children: vec![] }}] }};
            let value = producer::{change}::Set(node);
            let copied = neocogi_com::NcValueRef::new(&value).unwrap().to_owned().unwrap();
            let producer::{change}::Set(node) = copied else {{ panic!("selected alternative changed") }};
            assert_eq!(node.children[0].value, "child");
            #[neocogi_com::com_object(consumer::Receiver)]
            /// Consumer-owned concrete queue is compiled against renamed template parameters.
            struct Queue;
            impl consumer::Receiver for Queue {{}}
            impl consumer::{receiver} for Queue {{
                /// Decode through the peer's own generated recursive wire representation.
                fn accept(&mut self, value: neocogi_com::NcValueRef<'_, consumer::{change}>) -> Result<(), neocogi_com::NcStatus> {{
                    let consumer::{change}::Set(node) = value.to_owned()? else {{ panic!("wrong event") }};
                    assert_eq!(node.value, "parent");
                    assert_eq!(node.children[0].value, "child");
                    Ok(())
                }}
            }}
            let receiver = Queue::from_value(Queue);
            let producer_view = receiver.query::<producer::{receiver}Abi>().unwrap();
            producer_view.as_com_ref().accept(neocogi_com::NcValueRef::new(&value).unwrap()).unwrap();
            let descriptor = producer::ReceiverAbi::IDL_INTERFACE;
            assert!(descriptor.values.iter().any(|value| value.qualified_name == producer::{node}::IDL_TYPE.qualified_name));
            assert_eq!(producer::{node}::IDL_TYPE.qualified_name, consumer::{node}::IDL_TYPE.qualified_name);
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
}

/// Field order changes the closed value ABI without changing its nominal identity.
#[test]
fn value_template_layout_is_part_of_specialized_signatures() {
    let graph = |fields: &str| {
        resolve_modules(vec![
            module("proof/templates.ncidl", &format!("#[version(1)] struct Pair<T> {{ {fields} }}")),
            module(
                "proof/consumer.ncidl",
                "extern proof.templates; use proof.templates::Pair; #[version(1)] struct Payload { pair: Pair<bool> }",
            ),
        ])
        .unwrap()
    };
    let first = instances(&graph("first: T, second: u32"), "proof.consumer");
    let changed = instances(&graph("second: u32, first: T"), "proof.consumer");
    assert_eq!(first[0].1, changed[0].1);
    assert_ne!(first[0].2, changed[0].2);
}
