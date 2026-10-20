//! Bound references retain their lexical meaning in base, event and ordinary type positions.
use std::{fs, path::Path, process::Command};
use neocogi_idl::{parse_module, resolve_modules, Module, ResolvedModules};

/// Parses a module with stable logical identity for independent graph comparisons.
fn module(path: &str, source: &str) -> Module {
    parse_module(Path::new(path), source).unwrap()
}

/// Real event root plus ordinary declarations and generic reference-only interfaces.
fn graph(parameters: (&str, &str), base: &str, event: &str, instantiate: bool) -> Result<ResolvedModules, neocogi_idl::ResolutionError> {
    let (base_parameter, event_parameter) = parameters;
    let templates = format!(
        r#"
        extern neocogi.com.events;
        use neocogi.com.events::IEvent;
        #[version(1)] interface IBase {{ fn value(&self) -> Result<u32>; }}
        #[version(1)] interface IChanged: IEvent {{ fn number(&self) -> Result<u32>; }}
        #[version(1)] interface ForwardBase<{base_parameter}>: {base} {{}}
        #[version(1)] #[events({event})] interface ForwardEvents<{event_parameter}> {{}}
    "#
    );
    let consumer = if instantiate {
        r#"
        extern proof.templates;
        use proof.templates::{ForwardBase, ForwardEvents, IBase, IChanged};
        #[version(1)] interface Counter: ForwardBase<IBase> + ForwardEvents<IChanged> {}
    "#
    } else {
        "#[version(1)] interface Anchor {}"
    };
    resolve_modules(vec![
        module("neocogi/com/events.ncidl", include_str!("../../api/idl/neocogi/com/events/events.ncidl")),
        module("proof/templates.ncidl", &templates),
        module("proof/consumer.ncidl", consumer),
    ])
}

/// Direct base/event parameters are accepted before and after closed instantiation.
#[test]
fn direct_bound_bases_and_events_resolve_and_ignore_parameter_spelling() {
    for instantiate in [false, true] {
        let first = graph(("B", "E"), "B", "E", instantiate).unwrap();
        let renamed = graph(("Base", "Event"), "Base", "Event", instantiate).unwrap();
        for left in first.modules() {
            let right = renamed.module(&left.source.name.to_string()).unwrap();
            assert_eq!(left.module_id, right.module_id);
            assert_eq!(left.abi_fingerprint, right.abi_fingerprint);
            assert_eq!(left.declarations, right.declarations);
        }
    }
}

/// A qualified same-spelled declaration stays free rather than binding to a parameter.
#[test]
fn qualified_references_remain_free_in_base_and_event_positions() {
    for instantiate in [false, true] {
        let first = graph(("IBase", "IChanged"), "proof::templates::IBase", "proof::templates::IChanged", instantiate).unwrap();
        let renamed = graph(
            ("UnusedBase", "UnusedEvent"),
            "proof::templates::IBase",
            "proof::templates::IChanged",
            instantiate,
        )
        .unwrap();
        assert_eq!(
            first.module("proof.templates").unwrap().abi_fingerprint,
            renamed.module("proof.templates").unwrap().abi_fingerprint
        );
        assert_eq!(
            first.module("proof.consumer").unwrap().declarations,
            renamed.module("proof.consumer").unwrap().declarations
        );
    }
}

/// Binding positions remain revision data even when the supplied concrete arguments are equal.
#[test]
fn switching_base_or_event_parameter_changes_revision_without_changing_nominal_identity() {
    for event in [false, true] {
        let resolve = |selected: &str| {
            let declaration = if event {
                format!("#[version(1)] #[events({selected})] interface Select<A, B> {{}}")
            } else {
                format!("#[version(1)] interface Select<A, B>: {selected} {{}}")
            };
            let argument = if event { "IChanged" } else { "IBase" };
            let source = format!(
                r#"
                extern neocogi.com.events; use neocogi.com.events::IEvent;
                #[version(1)] interface IBase {{}}
                #[version(1)] interface IChanged: IEvent {{}}
                {declaration}
                #[version(1)] interface Concrete: Select<{argument}, {argument}> {{}}
            "#
            );
            resolve_modules(vec![
                module("neocogi/com/events.ncidl", include_str!("../../api/idl/neocogi/com/events/events.ncidl")),
                module("proof/selection.ncidl", &source),
            ])
            .unwrap()
        };
        let first = resolve("A");
        let changed = resolve("B");
        let left = first.module("proof.selection").unwrap();
        let right = changed.module("proof.selection").unwrap();
        assert_ne!(left.abi_fingerprint, right.abi_fingerprint);
        for (a, b) in left.declarations.iter().zip(&right.declarations) {
            assert_eq!(a.id, b.id);
            assert_ne!(a.abi_signature, b.abi_signature);
        }
    }
}

/// Rejection belongs to substitution/type validation, never to an internal fingerprint failure.
#[test]
fn invalid_closed_references_have_source_diagnostics() {
    for (declaration, code) in [
        ("#[version(1)] interface Select<B>: B {} #[version(1)] interface Bad: Select<u32> {}", "EIDL060"),
        (
            "#[version(1)] interface Select<B>: B {} #[version(1)] interface Bad: Select<Data> {}",
            "EIDL060",
        ),
        (
            "#[version(1)] #[events(E)] interface Select<E> {} #[version(1)] interface Bad: Select<u32> {}",
            "EIDL060",
        ),
        (
            "#[version(1)] #[events(E)] interface Select<E> {} #[version(1)] interface Bad: Select<Ordinary> {}",
            "EIDL065",
        ),
        ("#[version(1)] interface Select<B>: B<u32> {}", "EIDL060"),
        ("#[version(1)] #[events(E<u32>)] interface Select<E> {}", "EIDL060"),
    ] {
        let source = format!("#[version(1)] struct Data {{ value: u32 }} #[version(1)] interface Ordinary {{}} {declaration}");
        let error = resolve_modules(vec![
            module("neocogi/com/events.ncidl", include_str!("../../api/idl/neocogi/com/events/events.ncidl")),
            module("proof/invalid.ncidl", &source),
        ])
        .unwrap_err();
        assert!(!error.diagnostics.is_empty());
        assert!(error.diagnostics.iter().all(|d| d.code == code), "{declaration}: {error}");
        assert!(error.to_string().contains("proof/invalid.ncidl"), "source context missing: {error}");
    }
}

/// Returns a generated specialization name without hard-coding nominal UUID spellings.
fn instance(graph: &ResolvedModules, template: &str) -> String {
    graph
        .module("proof.consumer")
        .unwrap()
        .source
        .declarations
        .iter()
        .find(|d| d.specialization.as_ref().is_some_and(|s| s.template.segments.last().unwrap() == template))
        .unwrap()
        .name
        .clone()
}

/// Two independently resolved/printed peers negotiate and execute inherited calls and event getters.
#[test]
fn generated_bound_reference_peers_execute_real_com_and_event_metadata() {
    let first = graph(("B", "E"), "B", "E", true).unwrap();
    let second = graph(("Base", "Event"), "Base", "Event", true).unwrap();
    let base = instance(&first, "ForwardBase");
    let events = instance(&first, "ForwardEvents");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        format!(
            "[package]\nname='bound-reference-peers'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            root.join("crates/com/api")
        ),
    )
    .unwrap();
    let mut paths = neocogi_idl::generate::RustModulePaths::default();
    paths.insert("neocogi.com.events", "neocogi_com::events").unwrap();
    paths.insert("proof.templates", "bound_reference_peers::templates").unwrap();
    fs::write(
        project.path().join("src/templates.rs"),
        neocogi_idl::generate::generate_rust_with_paths(&first, "proof.templates", &paths).unwrap(),
    )
    .unwrap();
    for (name, graph) in [("first", &first), ("second", &second)] {
        let mut paths = paths.clone();
        paths.insert("proof.consumer", &format!("bound_reference_peers::{name}")).unwrap();
        let rust = neocogi_idl::generate::generate_rust_with_paths(graph, "proof.consumer", &paths).unwrap();
        fs::write(project.path().join(format!("src/{name}.rs")), rust).unwrap();
    }
    fs::write(
        project.path().join("src/lib.rs"),
        format!(
            r#"
        extern crate self as bound_reference_peers;
        pub mod templates; pub mod first; pub mod second;
        #[neocogi_com::com_object(first::Counter)]
        struct Counter;
        impl templates::IBase for Counter {{ fn value(&self) -> Result<u32, neocogi_com::NcStatus> {{ Ok(73) }} }}
        impl first::{base} for Counter {{}}
        impl first::{events} for Counter {{}}
        impl first::Counter for Counter {{}}
        #[neocogi_com::com_object(interfaces(templates::IChanged), ownership = exclusive)]
        struct Fact;
        impl neocogi_com::events::IEvent for Fact {{}}
        impl templates::IChanged for Fact {{ fn number(&self) -> Result<u32,neocogi_com::NcStatus> {{ Ok(91) }} }}
        #[test]
        fn independently_generated_peers_negotiate() {{
            let object=Counter::from_value(Counter).query::<first::CounterAbi>().unwrap();
            let peer=object.query::<second::CounterAbi>().unwrap();
            assert_eq!(peer.value().unwrap(),73);
            let declared=second::{events}Abi::IDL_INTERFACE.events;
            assert_eq!(declared.len(),1);
            assert_eq!(declared[0].qualified_name,"proof.templates.IChanged");
            assert_eq!(declared[0].fields[0].name,"number");
            let event=Fact::into_box(Fact).query::<neocogi_com::events::IEventAbi>().unwrap();
            assert_eq!(event.query_ref::<templates::IChangedAbi>().unwrap().number().unwrap(),91);
        }}
    "#
        ),
    )
    .unwrap();
    let output = Command::new("cargo")
        .args(["test", "--offline", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target/bound-reference-peers"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
}
