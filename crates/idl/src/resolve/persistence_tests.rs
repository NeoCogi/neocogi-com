//! Real semantic diagnostics distinguish nominal graph depth from inline height.
use std::{fmt::Write, path::Path};
use crate::{ModuleGraphError, parse_module, resolve_modules};

/// Source-located DFS reasons depend on the request root, not an SCC-wide cache.
#[test]
fn persistence_cycle_keeps_later_siblings_and_per_request_reasons() {
    let source = "interface IA {}\ninterface IB {}\n\
        struct A { first: Vec<B>, leaf: ComPtr<IA> }\n\
        struct B { first: Vec<A>, leaf: ComPtr<IB> }\n\
        #[persistence(default)] struct RootA { value: A }\n\
        #[persistence(default)] struct RootB { value: B }\n";
    let ModuleGraphError::Resolution(error) = resolve_modules(vec![parse_module(Path::new("proof/cycle.ncidl"), source).unwrap()]).unwrap_err() else {
        panic!("expected owned resolution diagnostics")
    };
    assert_eq!(error.diagnostics.len(), 2);
    for (diagnostic, root, leaf, line) in [(&error.diagnostics[0], "RootA", "IB", 5), (&error.diagnostics[1], "RootB", "IA", 6)] {
        assert_eq!(diagnostic.code, "EIDL064");
        assert!(diagnostic.message.contains(root));
        assert!(diagnostic.notes[0].starts_with(&format!("ComPtr<proof.cycle.{leaf}>")));
        assert_eq!(diagnostic.labels[0].span.path(), "proof/cycle.ncidl");
        assert_eq!(diagnostic.labels[0].span.start().line, line);
    }
}

/// A generic request retains the supplying field and closed-specialization label.
#[test]
fn imported_persistence_request_keeps_specialization_sources() {
    use crate::{ModuleSource, parse_module_parts};
    let path = Path::new("proof/tree/internal/part.ncidl");
    let definition_text = "#[persistence(default)] struct Required<T> { value: T }";
    let request_text = "extern proof.tree; use proof.tree::Required; struct Root { value: Required<ComPtr<IUnknown>> }";
    let definitions = parse_module_parts(Path::new("proof/tree"), &[ModuleSource { path, text: definition_text }]).unwrap();
    let request = parse_module_parts(Path::new("proof/tree/internal"), &[ModuleSource { path, text: request_text }]).unwrap();
    let ModuleGraphError::Resolution(error) = resolve_modules(vec![request, definitions]).unwrap_err() else {
        panic!("expected owned resolution diagnostics")
    };
    let diagnostic = error.diagnostics.iter().find(|d| d.code == "EIDL064").unwrap();
    assert_eq!(diagnostic.labels[0].span.path(), path.to_str().unwrap());
    assert_eq!(diagnostic.labels[1].span.path(), diagnostic.labels[0].span.path());
    assert_eq!(diagnostic.labels[0].source().unwrap().text(), definition_text);
    assert_eq!(diagnostic.labels[1].source().unwrap().text(), request_text);
    assert_eq!(diagnostic.labels.len(), 2);
    assert!(diagnostic.notes[0].contains("IUnknown"));
}

/// Finite records concatenate individually admitted inline expressions; their
/// diagnostic walk must not concatenate native stacks. Run in a watchdog child
/// so the old abort is an ordinary failing test, with no core dump on Linux.
#[cfg(not(miri))]
#[test]
fn persistence_graph_diagnostics_fit_a_small_caller_stack() {
    use std::process::Command;
    const SELECT: &str = "NEOCOGI_IDL_PERSISTENCE_GRAPH_CHILD";
    const TEST: &str = "resolve::persistence_tests::persistence_graph_diagnostics_fit_a_small_caller_stack";
    if let Ok(case) = std::env::var(SELECT) {
        #[cfg(target_os = "linux")]
        crate::bounded_child::cap();
        std::thread::Builder::new()
            .stack_size(1024 * 1024)
            .spawn(move || {
                let (records, wrappers, supported) = match case.as_str() {
                    "unsupported" => (64, 95, false),
                    "supported" => (64, 95, true),
                    "short" => (16, 95, false),
                    "shallow" => (64, 0, false),
                    _ => panic!("unknown case"),
                };
                let mut source = String::from("#[persistence(default)] struct Root { value: S0 }\n");
                for i in 0..records {
                    if i + 1 == records {
                        writeln!(source, "struct S{i} {{ value: {} }}", if supported { "bool" } else { "ComPtr<IUnknown>" }).unwrap();
                    } else {
                        writeln!(source, "struct S{i} {{ value: {}S{}{} }}", "Vec<".repeat(wrappers), i + 1, ">".repeat(wrappers)).unwrap();
                    }
                }
                let module = parse_module(Path::new("proof/persistence.ncidl"), &source).unwrap();
                let result = resolve_modules(vec![module]);
                if supported {
                    let graph = result.unwrap();
                    assert_eq!(graph.module("proof.persistence").unwrap().declarations.len(), records + 1);
                    drop(graph.clone());
                    // Stream Debug output without retaining a second large representation.
                    /// Counts streamed formatting without retaining another model/string.
                    struct Discard(usize);
                    impl std::fmt::Write for Discard {
                        fn write_str(&mut self, text: &str) -> std::fmt::Result {
                            self.0 += text.len();
                            Ok(())
                        }
                    }
                    let mut discarded = Discard(0);
                    write!(&mut discarded, "{graph:?}").unwrap();
                    assert!(discarded.0 > source.len());
                } else {
                    let ModuleGraphError::Resolution(error) = result.unwrap_err() else {
                        panic!("expected semantic error")
                    };
                    assert_eq!(error.diagnostics.len(), 1);
                    let diagnostic = &error.diagnostics[0];
                    assert_eq!(diagnostic.code, "EIDL064");
                    assert!(diagnostic.notes[0].starts_with("ComPtr<IUnknown>"));
                    assert_eq!(diagnostic.labels[0].span.path(), "proof/persistence.ncidl");
                    assert_eq!(diagnostic.labels[0].span.start().line, 1);
                    drop(error.clone());
                    assert!(error.to_string().contains("Root field value"));
                }
            })
            .unwrap()
            .join()
            .unwrap();
        return;
    }
    let mut failures = Vec::new();
    for case in ["unsupported", "supported", "short", "shallow"] {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", TEST, "--nocapture", "--test-threads=1"]).env(SELECT, case);
        let output = crate::bounded_child::run(command);
        if !output.status.success() {
            failures.push(format!("{case}: {}\n{}", output.status, String::from_utf8_lossy(&output.stderr)));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
