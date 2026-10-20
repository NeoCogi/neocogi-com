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

/// Even an incorrectly unchanged version must not hide authored enum assignments.
#[test]
fn integer_assignment_presence_is_contract_data_for_templates_and_concrete_enums() {
    for parameters in ["", "<T>"] {
        let implicit = format!("#[version(1)] enum State{parameters}: u32 {{ Empty, Ready }}");
        let explicit = format!("#[version(1)] enum State{parameters}: u32 {{ Empty = 0, Ready = 1 }}");
        assert_ne!(fingerprint(&implicit), fingerprint(&explicit));
        let revised = explicit.replace("version(1)", "version(2)");
        assert_ne!(fingerprint(&explicit), fingerprint(&revised));
        assert_eq!(fingerprint(&revised), fingerprint(&revised));
    }
}

/// Authored generation policies cannot depend on whether a declaration is generic.
#[test]
fn declaration_policies_are_encoded_uniformly() {
    for parameters in ["", "<T>"] {
        for attribute in ["#[persistence(default)]"] {
            let ordinary = format!("#[version(1)] struct Data{parameters} {{ value: u32 }}");
            let changed = format!("{attribute} {ordinary}");
            assert_ne!(fingerprint(&ordinary), fingerprint(&changed));
        }
        let ordinary = format!("#[version(1)] enum State{parameters}: u8 {{ Empty, Ready }}");
        let dense = format!("#[dense] {ordinary}");
        assert_ne!(fingerprint(&ordinary), fingerprint(&dense));
        assert_ne!(fingerprint(&dense), fingerprint(&format!("#[names(snake_case)] {dense}")));
    }
}

/// Version increments change ordinary nominal identity; duplicate revisions cannot share a module.
#[test]
fn declaration_versions_and_names_remain_authoritative() {
    let first = graph("#[version(1)] enum State: u8 { Empty, Ready }");
    let second = graph("#[version(2)] enum State: u8 { Empty = 0, Ready = 1 }");
    let left = first.module("proof.fingerprints").unwrap().declaration("State").unwrap();
    let right = second.module("proof.fingerprints").unwrap().declaration("State").unwrap();
    assert_ne!(left.id, right.id);
    assert_ne!(left.abi_signature, right.abi_signature);
    let duplicate = "#[version(1)] enum State { Empty } #[version(2)] enum State { Empty }";
    assert!(parse_module(Path::new("proof/fingerprints.ncidl"), duplicate).unwrap_err().to_string().contains("EIDL010"));
}

/// Resolved free identities and parameter ordinals ignore imports, source parts and binder spelling.
#[test]
fn template_free_names_use_resolved_identity() {
    let before = "#[version(1)] struct Item { value: u32 } #[version(1)] struct Data<T> { item: Item, value: T }";
    let after = "#[version(1)] struct Item { value: u32 } #[version(1)] struct Data<Value> { item: proof::fingerprints::Item, value: Value }";
    assert_eq!(fingerprint(before), fingerprint(after));
}

/// Pins canonical encoding v11, including positional generic payloads.
#[test]
fn canonical_module_encoding_has_stable_golden_digest() {
    let source = "#[version(2)] enum State: u8 { Empty = 0, Ready = 7 } #[version(1)] enum Data<T> { Empty, Value(T) } #[version(1)] struct Snapshot { state: State, value: Data<bool> }";
    assert_eq!(fingerprint(source), [115, 198, 123, 150, 22, 202, 79, 4, 165, 129, 14, 58, 232, 140, 243, 120, 125, 112, 169, 94, 4, 149, 53, 19, 255, 27, 237, 49, 133, 67, 172, 150]);
}
