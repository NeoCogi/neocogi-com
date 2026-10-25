//! UUID collisions cannot erase bound meaning before capability validation.

use std::path::Path;
use neocogi_com_core::{guid_from_name, guid_from_qualified_name, NcGuid};
use neocogi_idl::{parse_module, resolve_modules, Module, ModuleGraphError};

/// Parses source under a real logical module name.
fn module(name: &str, source: &str) -> Module {
    parse_module(Path::new(name), source).unwrap()
}

/// Existing intrinsic/constructor UUID encoding, deliberately unchanged by the fix.
fn legacy(bytes: &[u8]) -> NcGuid {
    guid_from_name(guid_from_qualified_name("neocogi.idl.closed-type:v1"), bytes)
}

/// Both origins must survive long enough to produce an author-facing diagnostic.
fn conflict(modules: Vec<Module>) -> String {
    let ModuleGraphError::Resolution(error) = resolve_modules(modules).unwrap_err() else {
        panic!("expected semantic conflict")
    };
    assert!(error.diagnostics.iter().any(|diagnostic| diagnostic.labels.len() >= 2), "{error}");
    error.to_string()
}

/// An unrelated scalar request must never turn an owning String record into u32.
#[test]
fn specialization_cache_checks_meaning_before_reusing_a_uuid() {
    let scalar = legacy(b"u32");
    assert_eq!(scalar.to_string(), "10805004-d8d2-5587-95f4-aa3989afe14f");
    let mut array = Vec::from(b"array".as_slice());
    array.extend_from_slice(&2u32.to_be_bytes());
    array.extend_from_slice(&scalar.to_uuid_bytes());
    for (argument, id) in [("u32", scalar), ("()", legacy(b"()")), ("[u32; 2]", legacy(&array))] {
        for phantom in [false, true] {
            for alias_first in [false, true] {
                let (first, second) = if alias_first { ("Alias", argument) } else { (argument, "Alias") };
                let fields = if phantom { "value: u32" } else { "value: T" };
                let common = format!(
                    r#"
                    #[id(1, "{id}")] struct Alias {{ owned: String }}
                    #[version(1)] struct Boxed<T> {{ {fields} }}
                    #[version(1)] struct BSecond {{ value: Boxed<{second}> }}
                    #[version(1)] interface Calls {{ fn send(&mut self, value: BSecond) -> Result<()>; }}
                "#
                );
                let unrelated = format!("#[version(1)] struct AFirst {{ value: Boxed<{first}> }}");
                for source in [format!("{unrelated}{common}"), format!("{common}{unrelated}")] {
                    for reversed in [false, true] {
                        let mut parsed = module("proof/collision.ncidl", &source);
                        if reversed {
                            parsed.declarations.reverse();
                        }
                        let error = conflict(vec![parsed]);
                        assert!(error.contains("conflicting semantic origins"), "{argument}, phantom={phantom}: {error}");
                        assert!(error.contains("proof/collision.ncidl"), "{error}");
                    }
                }
            }
        }
    }
    // The owning request alone stays owning and is rejected at its copied use.
    let source = format!(
        r#"
        #[id(1, "{scalar}")] struct Alias {{ owned: String }}
        #[version(1)] struct Boxed<T> {{ value: T }}
        #[version(1)] struct BSecond {{ value: Boxed<Alias> }}
        #[version(1)] interface Calls {{ fn send(&mut self, value: BSecond) -> Result<()>; }}
    "#
    );
    let error = resolve_modules(vec![module("proof/collision.ncidl", &source)]).unwrap_err().to_string();
    assert!(error.contains("copied"), "{error}");
}

/// Independently materialized projections cannot hide the same collision in two caches.
#[test]
fn canonical_admission_checks_origins_across_consumers() {
    let id = legacy(b"u32");
    let owner = format!(
        r#"
        #[id(1, "{id}")] struct Alias {{ owned: String }}
        #[version(1)] struct Boxed<T> {{ value: T }}
    "#
    );
    let first = "extern proof.templates; #[version(1)] struct First { value: proof::templates::Boxed<u32> }";
    let second = "extern proof.templates; #[version(1)] struct Second { value: proof::templates::Boxed<proof::templates::Alias> }";
    for swapped in [false, true] {
        let (a, b) = if swapped { (second, first) } else { (first, second) };
        let error = conflict(vec![
            module("proof/a.ncidl", a),
            module("proof/z.ncidl", b),
            module("proof/templates.ncidl", &owner),
        ]);
        assert!(error.contains("conflicting semantic origins"), "{error}");
    }
}

/// Canonical names and local generated Rust names are different namespaces.
#[test]
fn authored_names_cannot_impersonate_a_closed_canonical_definition() {
    for owner in ["aaa", "zzz"] {
        for template in ["struct Boxed<T> { value: T }", "enum Boxed<T> { Item(T), Empty }"] {
            let definition = format!("#[version(1)] {template}");
            let consumer = format!("extern proof.{owner}; #[version(1)] struct Payload {{ value: proof::{owner}::Boxed<u32> }}");
            let control = resolve_modules(vec![
                module(&format!("proof/{owner}.ncidl"), &definition),
                module("proof/middle.ncidl", &consumer),
            ])
            .unwrap();
            let concrete = control.module("proof.middle").unwrap();
            let (source, identity) = concrete
                .source
                .declarations
                .iter()
                .zip(&concrete.declarations)
                .find(|(source, _)| source.specialization.is_some())
                .unwrap();
            for same_id in [false, true] {
                let annotation = if same_id {
                    format!("#[id(1, \"{}\")]", identity.id)
                } else {
                    "#[version(1)]".into()
                };
                let authored = format!("{definition} {annotation} struct {} {{ owned: String }}", source.name);
                for reversed in [false, true] {
                    let mut inputs = vec![module(&format!("proof/{owner}.ncidl"), &authored), module("proof/middle.ncidl", &consumer)];
                    if reversed {
                        inputs.reverse();
                    }
                    let error = conflict(inputs);
                    assert!(error.contains(&source.name), "{error}");
                    assert!(error.contains("conflicting semantic origins") || error.contains("reuses the ID"), "{error}");
                }
            }
        }
    }
}

/// Active recursion and repeated imported references keep the same finite proof.
#[test]
fn repeated_recursive_nested_and_phantom_projections_remain_legal() {
    let owner = r#"
        #[version(1)] struct Data { value: u32 }
        #[version(1)] struct Node<T> { value: T, children: Vec<Node<T>> }
        #[version(1)] struct Phantom<T> { value: u32 }
    "#;
    let source = r#"
        extern proof.templates; use proof.templates::{Data, Node, Phantom};
        #[version(1)] struct Payload { first: Node<Data>, second: Node<proof::templates::Data>, phantom: Phantom<Node<Data>> }
    "#;
    let graph = resolve_modules(vec![
        module("proof/templates.ncidl", owner),
        module("proof/first.ncidl", source),
        module("proof/second.ncidl", source),
    ])
    .unwrap();
    let instances = |name: &str| {
        graph
            .module(name)
            .unwrap()
            .declarations
            .iter()
            .filter(|value| value.qualified_name.starts_with("proof.templates."))
            .map(|value| (value.qualified_name.clone(), value.id, value.abi_signature))
            .collect::<Vec<_>>()
    };
    assert_eq!(instances("proof.first"), instances("proof.second"));
    assert_eq!(instances("proof.first").len(), 2);
}
