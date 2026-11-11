//! Authored contract revisions share one encoding across templates and concrete types.
use std::path::Path;
use neocogi_idl::{parse_module, resolve_modules, ResolvedModules};

/// Resolves a fresh graph so equality never depends on shared compiler caches.
fn graph(source: &str) -> ResolvedModules {
    resolve_modules(vec![parse_module(Path::new("proof/fingerprints.ncidl"), source).unwrap()]).unwrap()
}

/// Reads the exact module revision without conflating it with a declaration's nominal ID.
fn fingerprint(source: &str) -> [u8; 32] {
    graph(source).module("proof.fingerprints").unwrap().abi_fingerprint
}

/// Authored enum assignments change signatures even when the resolved numbers agree.
#[test]
fn integer_assignment_presence_is_contract_data_for_templates_and_concrete_enums() {
    for parameters in ["", "<T>"] {
        let implicit = format!("enum State{parameters}: u32 {{ Empty, Ready }}");
        let explicit = format!("enum State{parameters}: u32 {{ Empty = 0, Ready = 1 }}");
        assert_ne!(fingerprint(&implicit), fingerprint(&explicit));
        assert_eq!(fingerprint(&explicit), fingerprint(&explicit));
    }
}

/// Authored generation policies cannot depend on whether a declaration is generic.
#[test]
fn declaration_policies_are_encoded_uniformly() {
    for parameters in ["", "<T>"] {
        let ordinary = format!("struct Data{parameters} {{ value: u32 }}");
        let changed = format!("#[persistence(default)] {ordinary}");
        assert_ne!(fingerprint(&ordinary), fingerprint(&changed));
        let ordinary = format!("enum State{parameters}: u8 {{ Empty, Ready }}");
        let dense = format!("#[dense] {ordinary}");
        assert_ne!(fingerprint(&ordinary), fingerprint(&dense));
        assert_ne!(fingerprint(&dense), fingerprint(&format!("#[names(snake_case)] {dense}")));
    }
}

/// Structural changes retain nominal identity and change the exact contract signature.
#[test]
fn declaration_names_and_exact_signatures_define_compatibility() {
    let first = graph("enum State: u8 { Empty, Ready }");
    let second = graph("enum State: u8 { Empty = 0, Ready = 1 }");
    let left = first.module("proof.fingerprints").unwrap().declaration("State").unwrap();
    let right = second.module("proof.fingerprints").unwrap().declaration("State").unwrap();
    assert_eq!(left.id, right.id);
    assert_ne!(left.abi_signature, right.abi_signature);
    let renamed = graph("enum StateWithTags: u8 { Empty = 0, Ready = 1 }");
    assert_ne!(right.id, renamed.module("proof.fingerprints").unwrap().declaration("StateWithTags").unwrap().id);
    let duplicate = "enum State { Empty } enum State { Empty }";
    assert!(
        parse_module(Path::new("proof/fingerprints.ncidl"), duplicate)
            .unwrap_err()
            .to_string()
            .contains("EIDL010")
    );
}

/// Neither old spelling nor the ignored integer may influence identity or compatibility.
#[test]
fn deprecated_versions_do_not_enter_module_or_closed_contract_signatures() {
    for source in [
        "struct Value { count: u32 }",
        "enum Value: u8 { Empty = 0, Ready = 7 }",
        "bitenum Value: u8 { ACTIVE = 1 }",
        "interface Value { fn read(&self) -> Result<u32>; }",
        "struct Value<T> { count: T } struct Closed { value: Value<u32> }",
        "enum Value<T> { Empty, Present(T) } struct Closed { value: Value<u32> }",
        "interface Value<T> { fn read(&self) -> Result<T>; } interface Closed: Value<u32> {}",
    ] {
        for explicit in [false, true] {
            let uuid = "9be73186-f123-47bc-8140-4a57825eeb21";
            let current = if explicit { format!("#[id(\"{uuid}\")] {source}") } else { source.to_owned() };
            let expected = graph(&current);
            let expected = expected.module("proof.fingerprints").unwrap();
            assert!(expected.source.warnings.is_empty());
            for version in [0, 1, 2, u32::MAX] {
                let old = if explicit {
                    format!("#[version({version})] #[id({version}, \"{uuid}\")] {source}")
                } else {
                    format!("#[version({version})] {source}")
                };
                let actual = graph(&old);
                let actual = actual.module("proof.fingerprints").unwrap();
                assert_eq!(expected.abi_fingerprint, actual.abi_fingerprint, "{old}");
                assert_eq!(expected.declarations, actual.declarations, "{old}");
                assert_eq!(actual.source.warnings.len(), if explicit { 2 } else { 1 });
            }
        }
    }
}

/// Resolved free identities and parameter ordinals ignore imports, source parts and binder spelling.
#[test]
fn template_free_names_use_resolved_identity() {
    let before = "struct Item { value: u32 } struct Data<T> { item: Item, value: T }";
    let after = "struct Item { value: u32 } struct Data<Value> { item: proof::fingerprints::Item, value: Value }";
    assert_eq!(fingerprint(before), fingerprint(after));
}

/// Pins canonical encoding v14, including versionless declarations and generic payloads.
#[test]
fn canonical_module_encoding_has_stable_golden_digest() {
    let source = "enum State: u8 { Empty = 0, Ready = 7 } enum Data<T> { Empty, Value(T) } struct Snapshot { state: State, value: Data<bool> }";
    assert_eq!(
        fingerprint(source),
        [
            126, 201, 84, 113, 186, 13, 216, 40, 168, 192, 34, 217, 239, 11, 211, 16, 205, 172, 127, 52, 81, 132, 132, 71, 60, 249, 250, 232, 221, 158, 154,
            113
        ]
    );
}
