//! Advertised events describe their real getter interfaces without constructor inference.
use neocogi_idl::{parse_module, resolve_modules, Module, ResolvedModules};
use std::path::Path;

/// Parses a self-contained source with its canonical module location.
fn module(path: &str, source: &str) -> Module {
    parse_module(Path::new(path), source).unwrap()
}
/// Includes the real common event root for every independent producer fixture.
fn resolve(source: &str) -> Result<ResolvedModules, neocogi_idl::ResolutionError> {
    resolve_modules(vec![
        module("neocogi/com/events.ncidl", include_str!("../../api/idl/neocogi/com/events/events.ncidl")),
        module("proof/events.ncidl", source),
    ])
}
/// Common imports used by the fixture declarations.
const PREFIX: &str = "extern neocogi.com.events; use neocogi.com.events::{IEvent, EventBinding};";

#[test]
fn common_binding_does_not_generate_product_helpers_or_reserve_their_names() {
    let graph = resolve(&format!(
        r#"{PREFIX}
        #[version(1)] interface IChanged: IEvent {{ fn value(&self) -> Result<u32>; }}
        #[version(1)] #[events(IChanged)] class Counter {{}}
        #[version(1)] struct CounterEventBinding {{ value: u32 }}
        #[version(1)] struct CounterEventPort {{ value: u32 }}
        #[version(1)] factory Factory {{ fn create(&self, events: &EventBinding) -> Result<ComBox<Counter>>; }}
    "#
    ))
    .unwrap();
    let source = neocogi_idl::generate_rust(&graph, "proof.events").unwrap();
    syn::parse_file(&source).unwrap();
    assert!(source.contains("struct CounterEventBinding"));
    assert!(source.contains("struct CounterEventPort"));
    assert!(!source.contains("pub type CounterEvent"));
    assert!(!source.contains("EventSource"));
    assert!(source.contains("IChangedAbi::INTERFACE_DESCRIPTOR"));
}

#[test]
fn advertised_events_need_no_constructor_and_use_imported_getter_scope() {
    let graph = resolve_modules(vec![
        module("neocogi/com/events.ncidl", include_str!("../../api/idl/neocogi/com/events/events.ncidl")),
        module(
            "proof/facts.ncidl",
            &format!(
                r#"{PREFIX}
            #[version(1)] struct Detail {{ enabled: bool }}
            #[version(1)] interface IBase: IEvent {{ fn detail(&self) -> Result<Detail>; }}
            #[version(1)] interface IChanged: IBase {{ fn count(&self) -> Result<u32>; }}
        "#
            ),
        ),
        module(
            "proof/events.ncidl",
            r#"extern proof.facts; use proof.facts::IChanged;
            #[version(1)] #[events(IChanged)] class Counter {}
        "#,
        ),
    ])
    .unwrap();
    let source = neocogi_idl::generate_rust(&graph, "proof.events").unwrap();
    assert!(source.contains("proof.facts.IChanged"));
    assert!(source.contains("proof.facts.Detail"));
    assert!(source.contains("proof_facts_contracts::Detail"));
    assert!(source.find("name: \"detail\"").unwrap() < source.find("name: \"count\"").unwrap());
}

#[test]
fn emitted_interfaces_must_derive_from_the_common_event_root() {
    for declaration in ["struct Fact { value: u32 }", "interface Fact {}", "class Fact {}"] {
        let error = resolve(&format!("{PREFIX} #[version(1)] {declaration} #[version(1)] #[events(Fact)] class Source {{}}")).unwrap_err();
        assert!(error.to_string().contains("IEvent"), "{error}");
    }
}

#[test]
fn event_getters_are_immutable_parameterless_and_repeatable() {
    for getter in [
        "fn read(&mut self) -> Result<u32>;",
        "fn read(&self, index: u32) -> Result<u32>;",
        "fn take(&self) -> Result<ComBox<IEvent>>;",
    ] {
        let error = resolve(&format!(
            "{PREFIX} #[version(1)] interface Fact: IEvent {{ {getter} }} #[version(1)] #[events(Fact)] class Source {{}}"
        ))
        .unwrap_err();
        assert!(error.to_string().contains("immutable"), "{error}");
    }
}

#[test]
fn duplicate_event_identities_and_inherited_getter_names_are_rejected() {
    for source in [
        "#[version(1)] interface Fact: IEvent {} #[version(1)] #[events(Fact, Fact)] class Source {}",
        "#[version(1)] interface Base: IEvent { fn value(&self) -> Result<u32>; } #[version(1)] interface Fact: Base { fn value(&self) -> Result<u32>; } #[version(1)] #[events(Fact)] class Source {}",
    ] {
        assert!(resolve(&format!("{PREFIX} {source}")).is_err());
    }
}

#[test]
fn event_attribute_is_nonempty_unique_and_interface_only() {
    for source in [
        "#[version(1)] #[events()] class Source {}",
        "#[version(1)] #[events(Fact)] #[events(Fact)] class Source {}",
        "#[version(1)] #[events(Fact)] struct Source { value: u32 }",
    ] {
        assert!(parse_module(Path::new("proof/events.ncidl"), source).is_err(), "{source}");
    }
}

#[test]
fn generic_events_specialize_and_keep_getter_value_metadata() {
    let graph = resolve(&format!(
        r#"{PREFIX}
        #[version(1)] struct Value {{ count: u32 }}
        #[version(1)] interface Fact<T>: IEvent {{ fn value(&self) -> Result<T>; }}
        #[version(1)] #[events(Fact<Value>)] class Source {{}}
    "#
    ))
    .unwrap();
    let source = neocogi_idl::generate_rust(&graph, "proof.events").unwrap();
    syn::parse_file(&source).unwrap();
    assert!(source.contains("proof.events.Value"));
    assert!(source.contains("name: \"value\""));
}

#[test]
fn event_getters_cannot_hide_ambiguity_in_multiple_inheritance() {
    let source = format!(
        r#"{PREFIX}
        #[version(1)] interface Left: IEvent {{ fn value(&self) -> Result<u32>; }}
        #[version(1)] interface Right: IEvent {{ fn value(&self) -> Result<bool>; }}
        #[version(1)] interface Fact: Left + Right {{}}
        #[version(1)] #[events(Fact)] class Source {{}}
    "#
    );
    assert!(resolve(&source).is_err());
}

/// Closing one template with different arguments creates distinct getter contracts.
#[test]
fn distinct_generic_ancestors_cannot_hide_duplicate_getter_names() {
    let error = resolve(&format!(r#"{PREFIX}
        #[version(1)] interface Snapshot<T>: IEvent {{ fn value(&self) -> Result<T>; }}
        #[version(1)] interface Left: Snapshot<u32> {{}}
        #[version(1)] interface Right: Snapshot<bool> {{}}
        #[version(1)] interface Fact: Left + Right {{}}
        #[version(1)] #[events(Fact)] class Source {{}}
    "#)).unwrap_err();
    assert!(error.to_string().contains("ambiguous"), "{error}");
}

/// Metadata is contract data even when no vtable method or constructor changes.
#[test]
fn advertised_interfaces_change_signatures_without_becoming_widget_bases() {
    let fingerprint = |events: &str| {
        let graph = resolve(&format!(
            r#"{PREFIX}
            #[version(1)] interface First: IEvent {{}}
            #[version(1)] interface Second: IEvent {{}}
            #[version(1)] {events} class Source {{}}
        "#
        ))
        .unwrap();
        let module = graph.module("proof.events").unwrap();
        let source = module.source.declarations.iter().find(|declaration| declaration.name == "Source").unwrap();
        let neocogi_idl::DeclarationKind::Interface(interface) = &source.kind else {
            unreachable!()
        };
        assert!(interface.bases.is_empty(), "emission does not add a callable widget projection");
        module.declaration("Source").unwrap().abi_signature
    };
    let without = fingerprint("");
    let first = fingerprint("#[events(First)]");
    assert_ne!(without, first);
    assert_ne!(first, fingerprint("#[events(Second)]"));
    assert_ne!(fingerprint("#[events(First, Second)]"), fingerprint("#[events(Second, First)]"));
}
