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

/// Rust method-wrapper companion names do not enter canonical interface identities.
/// These pre-rename goldens include a generic imported base, mutable derived method,
/// and the legitimate authored interface named Client.
#[test]
fn canonical_interfaces_keep_pre_wrapper_rename_identity() {
    let sources = [
        (
            "proof/base.ncidl",
            "struct Value { number: u32 } interface Base<T> { fn echo(&self, value: T) -> Result<T>; }",
        ),
        (
            "proof/derived.ncidl",
            "extern proof.base; interface Derived: proof::base::Base<u32> { fn set_value(&mut self, value: u32) -> Result<()>; } interface Client { fn call(&self) -> Result<u32>; }",
        ),
    ];
    let graph = resolve_modules(
        sources
            .into_iter()
            .map(|(path, source)| parse_module(Path::new(path), source).unwrap())
            .collect(),
    )
    .unwrap();
    assert_eq!(
        graph.module("proof.base").unwrap().abi_fingerprint,
        [
            142, 14, 235, 13, 245, 26, 79, 200, 77, 118, 136, 67, 174, 97, 146, 88, 170, 173, 197, 198, 69, 151, 169, 68, 212, 77, 254, 11, 54, 214, 160, 70
        ]
    );
    assert_eq!(
        graph.module("proof.derived").unwrap().abi_fingerprint,
        [
            122, 58, 230, 147, 66, 224, 36, 247, 67, 167, 107, 54, 213, 4, 178, 184, 178, 230, 218, 82, 19, 107, 127, 226, 0, 156, 191, 124, 33, 91, 230, 34
        ]
    );
    let declaration = graph
        .module("proof.base")
        .unwrap()
        .declarations
        .iter()
        .find(|declaration| declaration.qualified_name == "proof.base.Value")
        .unwrap();
    assert_eq!(declaration.id.to_string(), "d5cbba42-a5b8-5fc5-bea5-6501f55276b3");
    assert_eq!(
        declaration.abi_signature,
        [
            28, 250, 113, 65, 251, 68, 154, 208, 83, 98, 45, 66, 248, 237, 154, 23, 252, 81, 178, 197, 131, 221, 67, 105, 253, 5, 63, 40, 239, 20, 36, 250
        ]
    );
    let declaration = graph
        .module("proof.derived")
        .unwrap()
        .declarations
        .iter()
        .find(|declaration| declaration.qualified_name == "proof.base.BaseInstance3d933e6484685ca6993bfb6bd7a4aedb")
        .unwrap();
    assert_eq!(declaration.id.to_string(), "3d933e64-8468-5ca6-993b-fb6bd7a4aedb");
    assert_eq!(
        declaration.abi_signature,
        [
            50, 254, 148, 1, 189, 75, 183, 186, 160, 218, 220, 12, 41, 73, 206, 76, 9, 19, 60, 159, 139, 19, 79, 220, 82, 62, 60, 36, 249, 229, 117, 205
        ]
    );
    let declaration = graph
        .module("proof.derived")
        .unwrap()
        .declarations
        .iter()
        .find(|declaration| declaration.qualified_name == "proof.derived.Client")
        .unwrap();
    assert_eq!(declaration.id.to_string(), "fb47ac05-e084-5081-8d76-45f013e6b236");
    assert_eq!(
        declaration.abi_signature,
        [
            113, 69, 231, 30, 5, 134, 11, 251, 32, 62, 65, 80, 61, 190, 209, 135, 74, 72, 187, 31, 247, 225, 202, 247, 141, 251, 113, 75, 47, 218, 75, 215
        ]
    );
    let declaration = graph
        .module("proof.derived")
        .unwrap()
        .declarations
        .iter()
        .find(|declaration| declaration.qualified_name == "proof.derived.Derived")
        .unwrap();
    assert_eq!(declaration.id.to_string(), "4648447b-8190-5503-a7bc-3137a4546be3");
    assert_eq!(
        declaration.abi_signature,
        [
            17, 83, 249, 232, 75, 206, 126, 213, 148, 106, 210, 60, 118, 179, 231, 91, 136, 187, 209, 194, 218, 201, 50, 115, 196, 159, 126, 146, 18, 145, 92,
            58
        ]
    );
}
