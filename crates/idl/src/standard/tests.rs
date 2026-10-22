//! Real graph admission rejects worker failure before semantic resolution.
use std::{error::Error, io, path::Path};
use crate::{ModuleGraphError, ParseError, parse_module, resolve_modules, syntax::failure};

/// An unavailable worker is environmental even when the source is compiler-owned.
#[test]
fn bundled_worker_failure_is_returned_for_plain_and_standard_consumers() {
    for source in [
        "#[version(1)] interface Plain {}",
        "#[version(1)] struct Value { value: Option<u32> }",
        "extern neocogi.lang; use neocogi.lang::Option; #[version(1)] struct Value { value: Option<u32> }",
    ] {
        let module = parse_module(Path::new("proof/admission.ncidl"), source).unwrap();
        // Only the bundled parser remains: authored syntax has already been admitted.
        let denial = failure::after(0);
        let error = resolve_modules(vec![module.clone()]).unwrap_err();
        assert!(error.to_string().contains("bundled neocogi.lang"));
        let parser = error.source().unwrap();
        assert!(parser.source().unwrap().to_string().contains("test denied syntax worker start"));
        let ModuleGraphError::BundledSource(ParseError::Worker(cause)) = error else {
            panic!("expected bundled worker failure")
        };
        assert_eq!(cause.kind(), io::ErrorKind::WouldBlock);
        // One-shot consumption must not prevent an ordinary retry in the same scope.
        resolve_modules(vec![module]).unwrap();
        drop(denial);
    }
}

/// Empty inputs need no worker; reserved source checks follow bundled syntax admission.
#[test]
fn graph_failure_precedence_preserves_semantic_diagnostics() {
    let reserved = parse_module(Path::new("neocogi/lang.ncidl"), "#[version(1)] interface Reserved {}").unwrap();
    let denial = failure::after(0);
    let ModuleGraphError::Resolution(empty) = resolve_modules(vec![]).unwrap_err() else {
        panic!("expected empty-input diagnostic")
    };
    assert!(empty.to_string().contains("at least one IDL module"));
    // Empty input left the denial unconsumed; the reserved check is after parsing.
    assert!(matches!(
        resolve_modules(vec![reserved.clone()]),
        Err(ModuleGraphError::BundledSource(ParseError::Worker(_)))
    ));
    drop(denial);
    let error = resolve_modules(vec![reserved]).unwrap_err();
    let rendered = error.to_string();
    let ModuleGraphError::Resolution(semantic) = &error else {
        panic!("expected reserved-source diagnostic")
    };
    assert_eq!(semantic.diagnostics[0].code, "EIDL067");
    assert_eq!(rendered, semantic.to_string());
    assert_eq!(error.source().unwrap().to_string(), rendered);
}
