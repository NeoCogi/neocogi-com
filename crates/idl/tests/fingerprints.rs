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

/// Pins canonical encoding v15, including versionless declarations and generic payloads.
#[test]
fn canonical_module_encoding_has_stable_golden_digest() {
    let source = "enum State: u8 { Empty = 0, Ready = 7 } enum Data<T> { Empty, Value(T) } struct Snapshot { state: State, value: Data<bool> }";
    assert_eq!(
        fingerprint(source),
        [
            10, 77, 28, 226, 51, 40, 133, 92, 251, 58, 138, 154, 123, 186, 58, 191, 205, 35, 109, 242, 147, 109, 177, 128, 133, 115, 157, 229, 47, 233, 12, 121
        ]
    );
}

/// Rust method-wrapper companion names do not enter canonical interface identities.
/// These goldens include a generic imported base, mutable derived method,
/// and the legitimate authored interface named Client.
#[test]
fn canonical_interfaces_keep_exact_identities() {
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
            218, 43, 127, 45, 75, 189, 101, 126, 23, 148, 193, 254, 146, 65, 60, 138, 131, 183, 112, 193, 195, 69, 211, 93, 68, 236, 76, 67, 116, 206, 193, 77
        ]
    );
    assert_eq!(
        graph.module("proof.derived").unwrap().abi_fingerprint,
        [
            91, 165, 94, 57, 226, 48, 246, 231, 245, 58, 64, 131, 236, 14, 98, 27, 75, 167, 185, 141, 189, 110, 55, 95, 149, 106, 215, 145, 242, 189, 54, 99
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
            2, 171, 107, 211, 195, 73, 28, 118, 111, 104, 50, 194, 58, 25, 162, 180, 136, 1, 204, 160, 166, 33, 96, 86, 6, 142, 4, 187, 237, 161, 123, 32
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
            43, 106, 133, 9, 58, 143, 126, 247, 42, 230, 209, 54, 97, 73, 180, 68, 58, 97, 235, 146, 6, 249, 187, 118, 80, 97, 172, 215, 73, 123, 78, 209
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
            180, 180, 164, 129, 185, 107, 44, 187, 131, 26, 60, 61, 83, 101, 56, 85, 213, 50, 135, 97, 218, 183, 125, 68, 79, 228, 11, 86, 231, 233, 125, 206
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
            207, 111, 140, 148, 53, 59, 215, 98, 135, 104, 183, 41, 30, 208, 139, 194, 250, 179, 54, 208, 159, 217, 73, 159, 128, 130, 221, 220, 90, 9, 210,
            107
        ]
    );
}
