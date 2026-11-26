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

/// Pins canonical encoding v16, including versionless declarations and generic payloads.
#[test]
fn canonical_module_encoding_has_stable_golden_digest() {
    let source = "enum State: u8 { Empty = 0, Ready = 7 } enum Data<T> { Empty, Value(T) } struct Snapshot { state: State, value: Data<bool> }";
    assert_eq!(
        fingerprint(source),
        [
            240, 84, 186, 72, 111, 85, 205, 171, 107, 214, 205, 189, 239, 226, 121, 231, 28, 176, 228, 44, 113, 60, 159, 198, 157, 195, 174, 178, 172, 59, 118,
            224
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
            133, 228, 8, 222, 28, 30, 21, 89, 71, 148, 240, 152, 175, 96, 27, 219, 17, 241, 54, 37, 137, 99, 76, 214, 179, 91, 18, 139, 196, 74, 144, 50
        ]
    );
    assert_eq!(
        graph.module("proof.derived").unwrap().abi_fingerprint,
        [
            150, 127, 89, 215, 40, 46, 5, 15, 227, 87, 150, 26, 203, 2, 148, 119, 67, 212, 245, 26, 84, 131, 36, 234, 77, 190, 38, 123, 174, 177, 54, 50
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
            18, 1, 255, 232, 191, 15, 191, 21, 112, 162, 78, 175, 131, 49, 166, 9, 220, 72, 169, 68, 32, 178, 247, 234, 236, 36, 83, 93, 49, 207, 70, 159
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
            73, 216, 1, 153, 68, 5, 73, 0, 253, 207, 155, 184, 39, 65, 70, 80, 10, 218, 210, 97, 85, 40, 169, 51, 15, 248, 227, 34, 235, 215, 202, 1
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
            28, 114, 134, 188, 74, 137, 180, 25, 123, 120, 174, 111, 47, 40, 129, 156, 188, 59, 11, 190, 234, 221, 239, 211, 252, 109, 55, 113, 141, 3, 86, 125
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
            253, 225, 174, 1, 185, 242, 111, 157, 39, 128, 152, 214, 238, 173, 41, 202, 189, 124, 30, 0, 144, 40, 169, 85, 243, 100, 194, 99, 169, 154, 193,
            115
        ]
    );
}
