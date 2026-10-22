//! Failure selection exercises the actual syntax entry and its phase-specific callers.
use std::{error::Error, io, path::Path};
use super::{failure, with_syntax_stack};
use crate::{ModuleSource, ParseError};

/// A denial is consumed once; nested guards restore their enclosing caller's state.
#[test]
fn scoped_denial_is_one_shot_nested_and_thread_local() {
    let outer = failure::after(1);
    {
        let _inner = failure::after(0);
        assert_eq!(with_syntax_stack(|| ()).unwrap_err().kind(), io::ErrorKind::WouldBlock);
        with_syntax_stack(|| ()).unwrap();
    }
    // Another caller thread must not inherit the enclosing one-shot denial.
    std::thread::spawn(|| with_syntax_stack(|| ()).unwrap()).join().unwrap();
    with_syntax_stack(|| ()).unwrap();
    assert!(with_syntax_stack(|| ()).is_err());
    drop(outer);
    with_syntax_stack(|| ()).unwrap();
}

/// Guard cleanup must restore an ordinary worker even when caller work unwinds.
#[cfg(panic = "unwind")]
#[test]
fn scoped_denial_restores_after_unwind() {
    let panic = std::panic::catch_unwind(|| {
        let _guard = failure::after(0);
        panic!("leave denial scope");
    });
    assert!(panic.is_err());
    with_syntax_stack(|| ()).unwrap();
}

/// Genuine worker panics retain their payload; admission errors do not catch them.
#[cfg(panic = "unwind")]
#[test]
#[should_panic(expected = "original syntax panic")]
fn worker_panic_keeps_its_original_message() {
    with_syntax_stack::<()>(|| panic!("original syntax panic")).unwrap();
}

/// File, multipart, normal and bootstrap entries classify their own startup failures.
#[test]
fn parser_and_generator_failures_keep_their_phase() {
    let source = "#[version(1)] interface Plain {}";
    let module = crate::parse_module(Path::new("proof/phases.ncidl"), source).unwrap();
    let graph = crate::resolve_modules(vec![module]).unwrap();
    for multipart in [false, true] {
        let _denial = failure::after(0);
        let result = if multipart {
            crate::parse_module_parts(
                Path::new("proof/phases"),
                &[ModuleSource {
                    path: Path::new("proof/phases/part.ncidl"),
                    text: source,
                }],
            )
        } else {
            crate::parse_module(Path::new("proof/phases.ncidl"), source)
        };
        let error = result.unwrap_err();
        assert!(error.source().unwrap().to_string().contains("test denied syntax worker start"));
        assert!(matches!(error, ParseError::Worker(_)));
    }
    for bootstrap in [false, true] {
        let _denial = failure::after(0);
        let error = if bootstrap {
            crate::generate::generate_bootstrap(&graph, "proof.phases").err().unwrap()
        } else {
            crate::generate_rust(&graph, "proof.phases").unwrap_err()
        };
        assert!(error.to_string().contains("cannot start generated-Rust syntax worker"));
    }
}

/// Invalid path/part admission precedes syntax work and cannot consume its denial.
#[test]
fn early_parser_errors_are_not_worker_errors() {
    let source = "#[version(1)] interface Plain {}";
    let _denial = failure::after(0);
    let path = crate::parse_module(Path::new(""), "").unwrap_err();
    assert!(path.source().is_some());
    assert!(matches!(path, ParseError::ModulePath(_)));
    let part = crate::parse_module_parts(
        Path::new("proof/parts"),
        &[ModuleSource {
            path: Path::new("elsewhere.ncidl"),
            text: "",
        }],
    )
    .unwrap_err();
    assert!(matches!(part, ParseError::Source(_)));
    assert!(matches!(
        crate::parse_module(Path::new("proof/valid.ncidl"), source),
        Err(ParseError::Worker(_))
    ));
    crate::parse_module(Path::new("proof/valid.ncidl"), source).unwrap();
}
