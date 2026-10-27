//! Production-path small quotas and guarded public regressions. Expected limits
//! come from fixture structure, not the production estimator's returned costs.

use std::path::Path;
use crate::{Module, ModuleGraphError, parse_module};
use super::budget::{CopyKind, Limits};

/// Parses authored source through the same owned syntax boundary as callers.
fn source(path: &str, text: &str) -> Module {
    parse_module(Path::new(path), text).unwrap()
}

/// Runs actual admission, binding, specialization and downstream resolution.
fn resolve(text: &str, limits: Limits) -> Result<crate::ResolvedModules, ModuleGraphError> {
    crate::resolve::resolve_with_limits(vec![source("proof/budget.ncidl", text)], limits)
}

/// One finite phantom instantiation, without unrelated stored layout constraints.
const FINITE: &str = "#[version(1)] struct Marker<T> { value: u8 } #[version(1)] struct Root { item: Marker<u32> }";

/// No specialization means no generated allowance, however many authored fields.
#[test]
fn semantic_budget_excludes_source_normalization() {
    let text = format!(
        "#[version(1)] struct Source {{ {} }}",
        (0..200).map(|i| format!("field{i}: u32,")).collect::<String>()
    );
    resolve(
        &text,
        Limits {
            work: 0,
            bytes: 0,
            descent: 0,
            ..Limits::default()
        },
    )
    .unwrap();
}

/// Actual copy-site denials cannot be satisfied by estimator-only unit tests.
#[test]
fn semantic_budget_gates_real_declaration_and_argument_copies() {
    for (kind, text) in [
        (CopyKind::Declaration, FINITE.to_owned()),
        (
            CopyKind::Type,
            "#[version(1)] struct Twice<T> { a: T, b: T } #[version(1)] struct Root { item: Twice<u32> }".to_owned(),
        ),
    ] {
        let error = resolve(
            &text,
            Limits {
                denied_copy: Some(kind),
                ..Limits::default()
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("test denied generated copy"), "{error}");
    }
}

/// The two visits required just to inspect the first application cannot happen
/// at zero allowance; returning the error still preserves its real source span.
#[test]
fn semantic_budget_zero_work_and_storage_return_source_errors() {
    for limits in [Limits { work: 0, ..Limits::default() }, Limits { bytes: 0, ..Limits::default() }] {
        let error = resolve(FINITE, limits).unwrap_err();
        let ModuleGraphError::Resolution(error) = error else { panic!("wrong phase") };
        assert_eq!(error.diagnostics[0].code, "EIDL060");
        assert_eq!(error.diagnostics[0].labels.len(), 1);
        let span = &error.diagnostics[0].labels[0].span;
        assert_eq!(span.module(), "proof.budget");
        assert_eq!(span.path(), "proof/budget.ncidl");
        assert!(span.start().column > 1);
    }
}

/// One complete request fits in 100 visits; a hundred cache hits cannot fit in
/// 100. Each later independent resolve gets a fresh operation, not a global quota.
#[test]
fn semantic_budget_cache_hits_spend_batch_work() {
    let limits = Limits { work: 100, ..Limits::default() };
    resolve(FINITE, limits).unwrap();
    let repeated = format!(
        "#[version(1)] struct Marker<T> {{ value: u8 }} #[version(1)] struct Root {{ {} }}",
        (0..100).map(|i| format!("field{i}: Marker<u32>,")).collect::<String>()
    );
    let error = resolve(&repeated, limits).unwrap_err().to_string();
    assert!(error.contains("compiler work limit"), "{error}");
    resolve(FINITE, limits).unwrap();
}

/// Module entry cannot refresh the shared generated-storage allowance.
#[test]
fn semantic_budget_is_shared_across_consumer_modules() {
    let template = source(
        "proof/template.ncidl",
        &format!("/// {}\n#[version(1)] struct Marker<T> {{ value: u8 }}", "x".repeat(8192)),
    );
    let consumer = |name: &str| {
        source(
            name,
            "extern proof.template; use proof.template::Marker; #[version(1)] struct Root { value: Marker<u8> }",
        )
    };
    // Each individual copy fits; two copies of the 8 KiB docs alone exceed 12 KiB.
    let limits = Limits { bytes: 12 * 1024, ..Limits::default() };
    crate::resolve::resolve_with_limits(vec![template.clone(), consumer("proof/a.ncidl")], limits).unwrap();
    crate::resolve::resolve_with_limits(vec![template.clone(), consumer("proof/b.ncidl")], limits).unwrap();
    let error = crate::resolve::resolve_with_limits(vec![template, consumer("proof/a.ncidl"), consumer("proof/b.ncidl")], limits)
        .unwrap_err()
        .to_string();
    assert!(error.contains("compiler storage limit"), "{error}");
}

/// Long owned docs/span/key payloads are charged by actual template admission.
#[test]
fn semantic_budget_charges_generated_documentation() {
    let text = format!("/// {}\n{FINITE}", "x".repeat(8192));
    let error = resolve(&text, Limits { bytes: 4096, ..Limits::default() }).unwrap_err().to_string();
    assert!(error.contains("compiler storage limit"), "{error}");
    resolve(&text, Limits::default()).unwrap();
}

/// Separate template and argument heights fitting does not authorize their sum.
#[test]
fn semantic_budget_checks_destination_plus_argument_height() {
    let text = "#[version(1)] struct Place<T> { value: Vec<Vec<T>> } #[version(1)] struct Root { value: Place<Vec<Vec<u8>>> }";
    let error = resolve(text, Limits { descent: 6, ..Limits::default() }).unwrap_err().to_string();
    assert!(error.contains("compiler nesting limit"), "{error}");
    resolve(text, Limits::default()).unwrap();
}

/// A finite permutation cycle must preserve descent across declaration reentry.
#[test]
fn semantic_budget_composes_reentrant_descent() {
    let text = "#[version(1)] struct Rotate<A, B> { next: Vec<Vec<Vec<Rotate<B, A>>>> } #[version(1)] struct Root { value: Rotate<u8, u16> }";
    let error = resolve(text, Limits { descent: 10, ..Limits::default() }).unwrap_err().to_string();
    assert!(error.contains("compiler nesting limit"), "{error}");
    resolve(text, Limits::default()).unwrap();
}

/// At the admission boundary, an exact active cache hit is not a new instance.
#[test]
fn semantic_budget_preserves_recursive_reuse_at_admission_limit() {
    let text = "#[version(1)] struct Node<T> { value: T, children: Vec<Node<T>> } #[version(1)] struct Root { value: Node<u8> }";
    resolve(
        text,
        Limits {
            instances_depth: 1,
            instances: 1,
            ..Limits::default()
        },
    )
    .unwrap();
}

/// Borrowed slice normalization and interface bases move their existing values;
/// substitutions still cross the genuine copy gate and preserve callable shapes.
#[test]
fn semantic_budget_covers_bases_and_borrowed_slice_substitutions() {
    for text in [
        "#[version(1)] interface IBase<T> { fn take(&self, value: &T) -> Result<()>; } #[version(1)] interface IDerived: IBase<Slice<u8>> {}",
        "#[version(1)] interface IBase<T> {} #[version(1)] interface IDerived: IBase<u8> {}",
    ] {
        let error = resolve(
            text,
            Limits {
                denied_copy: Some(CopyKind::Declaration),
                ..Limits::default()
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("test denied generated copy"), "{error}");
        resolve(text, Limits::default()).unwrap();
    }
}

/// Finite duplicate/phantom constructors proceed through exact origins,
/// signatures and qualified borrowed documentation, not just expansion alone.
#[test]
fn bounded_application_generates_qualified_spelling() {
    let text = "#[version(1)] struct Pair<A, B> { marker: u8 } #[version(1)] struct Marker<T> { marker: u8 } #[version(1)] struct Root { value: Marker<Pair<Pair<u8, u8>, Pair<u8, u8>>> }";
    let graph = resolve(
        text,
        Limits {
            work: 4000,
            bytes: 100_000,
            ..Limits::default()
        },
    )
    .unwrap();
    let generated = crate::generate_rust(&graph, "proof.budget").unwrap();
    assert!(generated.contains("proof::budget::Marker<proof::budget::Pair<"));
}

/// Tiny input that formerly manufactured an exponentially duplicated phantom tree.
#[cfg(all(target_os = "linux", not(miri)))]
const PAIR_GROWTH: &str = "#[version(1)] struct Pair<A, B> { marker: u8 } #[version(1)] struct Grow<T> { children: Vec<Grow<Pair<T, T>>> } #[version(1)] struct Root { value: Grow<u8> }";

/// Public production limits, including real bundled parsing, must return a
/// source-located error under a memory ceiling and on a one-MiB semantic caller.
#[cfg(all(target_os = "linux", not(miri)))]
#[test]
fn public_expansion_limits_are_bounded_on_small_caller_stack() {
    const SELECT: &str = "NEOCOGI_IDL_PUBLIC_BUDGET_CASE";
    const TEST: &str = "specialize::tests::public_expansion_limits_are_bounded_on_small_caller_stack";
    if let Ok(case) = std::env::var(SELECT) {
        let text = match case.as_str() {
            "pair" => PAIR_GROWTH.to_owned(),
            "triple" => PAIR_GROWTH.replace("Pair<A, B>", "Pair<A, B, C>").replace("Pair<T, T>", "Pair<T, T, T>"),
            "linear" => "#[version(1)] struct Grow<T> { children: Vec<Grow<Option<T>>> } #[version(1)] struct Root { value: Grow<u8> }".to_owned(),
            "finite" => FINITE.to_owned(),
            "boundary" => {
                let mut text = String::new();
                for index in 0..64 {
                    text.push_str(&format!("#[version(1)] struct Link{index}<T> {{ next: Vec<Link{}<T>> }}", (index + 1).min(63)));
                }
                text.push_str("#[version(1)] struct Root { value: Link0<u8> }");
                text
            }
            other => panic!("unknown child {other}"),
        };
        let parsed = source("proof/growth.ncidl", &text);
        crate::bounded_child::cap();
        std::thread::Builder::new()
            .stack_size(1024 * 1024)
            .spawn(move || {
                let result = crate::resolve_modules(vec![parsed]);
                if case == "finite" || case == "boundary" {
                    result.unwrap();
                } else {
                    let error = result.unwrap_err();
                    let ModuleGraphError::Resolution(error) = error else { panic!("wrong phase") };
                    let diagnostic = &error.diagnostics[0];
                    assert_eq!(diagnostic.code, "EIDL060");
                    if case != "linear" {
                        assert!(
                            ["compiler work limit", "compiler storage limit", "compiler nesting limit"]
                                .iter()
                                .any(|reason| diagnostic.message.contains(reason)),
                            "{diagnostic:?}"
                        );
                    }
                    assert_eq!(diagnostic.labels.len(), 1);
                    assert_eq!(diagnostic.labels[0].span.module(), "proof.growth");
                    assert_eq!(diagnostic.labels[0].span.path(), "proof/growth.ncidl");
                    assert!(diagnostic.labels[0].span.start().column > 1);
                    eprintln!("{case}: {}", diagnostic.message);
                }
            })
            .unwrap()
            .join()
            .unwrap();
        println!("observed bounded public resolution");
        return;
    }
    for case in ["finite", "boundary", "pair", "triple", "linear"] {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", TEST, "--nocapture", "--test-threads=1"]).env(SELECT, case);
        let output = crate::bounded_child::run(command);
        assert!(
            output.status.success(),
            "{case}: {:?}\n{}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("observed bounded public resolution"));
    }
}

/// A retained output-slice argument must not be documented as a readable slice.
#[test]
fn closed_output_slice_spelling_is_preserved() {
    let graph = resolve(
        "#[version(1)] struct Marker<T> { marker: u8 } #[version(1)] struct Root { value: Marker<&mut [u8]> }",
        Limits::default(),
    )
    .unwrap();
    let generated = crate::generate_rust(&graph, "proof.budget").unwrap();
    assert!(generated.contains("proof::budget::Marker<&mut [u8]>"), "{generated}");
}
