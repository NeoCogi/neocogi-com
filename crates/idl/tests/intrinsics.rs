//! Intrinsic spellings share identities while ownership remains explicit.
use std::path::Path;
use neocogi_idl::{AbiType, Callable, DeclarationKind, parse_module, resolve_modules};

/// Resolves one isolated source under a fixed logical identity for ABI comparisons.
fn graph(source: &str) -> neocogi_idl::ResolvedModules {
    resolve_modules(vec![parse_module(Path::new("proof/intrinsics.ncidl"), source).unwrap()]).unwrap()
}

/// Equivalent intrinsic spellings must not manufacture incompatible contracts.
#[test]
fn array_and_slice_shorthands_have_identical_fingerprints() {
    let bracket = graph("#[version(1)] struct Data { values: [bool; 3] } #[version(1)] interface IInput { fn read(&self, values: &[u32]) -> Result<()>; fn write(&self, values: &mut [u32]) -> Result<()>; }");
    let named = graph("#[version(1)] struct Data { values: Array<bool, 3> } #[version(1)] interface IInput { fn read(&self, values: &Slice<u32>) -> Result<()>; fn write(&self, values: &mut Slice<u32>) -> Result<()>; }");
    assert_eq!(bracket.modules()[0].abi_fingerprint, named.modules()[0].abi_fingerprint);
    for name in ["Data", "IInput"] {
        assert_eq!(
            bracket.modules()[0].declaration(name).unwrap().abi_signature,
            named.modules()[0].declaration(name).unwrap().abi_signature
        );
    }
}

/// Closing a borrowed generic referent selects the ordinary slice calling convention.
#[test]
fn generic_slice_referents_normalize_after_substitution() {
    let resolved = graph("#[version(1)] interface IInput<T> { fn read(&self, values: &T) -> Result<()>; } #[version(1)] class Reader: IInput<Slice<u32>> {}");
    let module = &resolved.modules()[0];
    let instance = module.source.declarations.iter().find(|value| value.specialization.is_some()).unwrap();
    let DeclarationKind::Interface(interface) = &instance.kind else {
        panic!("interface instance expected")
    };
    let Callable::Method(method) = &interface.callables[0];
    assert_eq!(method.parameters[0].ty, AbiType::Slice(Box::new(AbiType::Scalar("u32".into()))));
    neocogi_idl::generate_rust(&resolved, "proof.intrinsics").unwrap();
}

/// A borrowed spelling cannot silently become owned storage in a record.
#[test]
fn vector_fields_own_values_and_slice_fields_are_rejected() {
    let resolved = graph("#[version(1)] struct Node { children: Vec<Node> }");
    assert!(matches!(
        resolved.values.definitions["proof.intrinsics.Node"].fields[0],
        neocogi_idl::ValueType::Sequence(_)
    ));
    for field in ["&[u8]", "&Slice<u8>", "Slice<u8>", "[u8]", "Vec<Slice<u8>>"] {
        let source = format!("#[version(1)] struct Data {{ items: {field} }}");
        let parsed = parse_module(Path::new("proof/invalid.ncidl"), &source).unwrap();
        let error = resolve_modules(vec![parsed]).unwrap_err().to_string();
        assert!(error.contains("EIDL060"), "{field}: {error}");
    }
}

/// Intrinsic counts are fixed compile-time integers, with no implicit evaluator.
#[test]
fn arrays_reject_invalid_counts_and_arity() {
    for ty in [
        "Array<u8>",
        "Array<u8, u32>",
        "Array<u8, -1>",
        "Array<u8, 4294967296>",
        "Array<u8, {1 + 1}>",
        "Vec<u8, u32>",
    ] {
        let source = format!("#[version(1)] struct Data {{ items: {ty} }}");
        assert!(parse_module(Path::new("proof/invalid.ncidl"), &source).is_err(), "{ty}");
    }
    graph("#[version(1)] struct Empty { items: Array<u8, 0> }");
}
