//! Production metadata collectors run on small callers without changing syntax ownership.
use super::*;
use std::{fmt::Write, path::Path, process::Command};

/// Extracts an observable method name, independently of the collector's work state.
fn method_name(tokens: TokenStream) -> String {
    let syn::Expr::Struct(value) = syn::parse2(tokens).unwrap() else {
        panic!("method metadata struct")
    };
    let field = value
        .fields
        .iter()
        .find(|field| matches!(&field.member, syn::Member::Named(name) if name == "name"))
        .unwrap();
    let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(name), .. }) = &field.expr else {
        panic!("literal method name")
    };
    name.value()
}

/// Each operation runs alone in a watchdog child. Shallow syntax keeps this a
/// nominal-depth test; TokenStreams are created, inspected and dropped on their
/// owning test thread. Public generation separately retains its syntax worker.
#[test]
fn nominal_metadata_collectors_fit_a_small_stack() {
    const SELECT: &str = "NEOCOGI_IDL_METADATA_GRAPH_CHILD";
    const TEST: &str = "generate::traversal_tests::nominal_metadata_collectors_fit_a_small_stack";
    const EDGES: usize = 2048;
    if let Ok(case) = std::env::var(SELECT) {
        #[cfg(target_os = "linux")]
        crate::bounded_child::cap();
        std::thread::Builder::new()
            .stack_size(1024 * 1024)
            .spawn(move || {
                let edges = if case == "public" { 32 } else { EDGES };
                let mut source = String::new();
                for i in 0..=edges {
                    let base = if i < edges { format!(": I{}", i + 1) } else { String::new() };
                    writeln!(source, "interface I{i}{base} {{ fn m{i}(&self) -> Result<u32>; }}").unwrap();
                }
                let graph = crate::resolve_modules(vec![crate::parse_module(Path::new("proof/metadata.ncidl"), &source).unwrap()]).unwrap();
                let module = graph.module("proof.metadata").unwrap();
                let declaration = module.source.declaration("I0").unwrap();
                let paths = RustModulePaths::default();
                let generation = Generation::new(&graph, &paths);
                match case.as_str() {
                    "ancestry" => {
                        let chain = interface_metadata::ancestry(&generation, module, declaration).unwrap();
                        assert_eq!(chain.len(), edges + 1);
                        for (i, entry) in chain.iter().enumerate() {
                            let expected = interface_metadata::identity(module.declaration(&format!("I{i}")).unwrap());
                            assert_eq!(entry.to_string(), expected.to_string());
                        }
                    }
                    "methods" => {
                        let methods = method_metadata::descriptions(&generation, module, declaration).unwrap();
                        assert_eq!(methods.len(), edges + 1);
                        for (i, entry) in methods.into_iter().enumerate() {
                            assert_eq!(method_name(entry), format!("m{}", edges - i));
                        }
                    }
                    "public" => {
                        let plain = crate::generate_rust(&graph, "proof.metadata").unwrap();
                        let mapped = crate::generate_rust_with_source_map(&graph, "proof.metadata").unwrap();
                        assert_eq!(plain, mapped.source);
                        let offset = mapped.source.find("pub trait I0").unwrap();
                        let before = &mapped.source[..offset];
                        let position = crate::SourceMapPosition {
                            line: before.bytes().filter(|byte| *byte == b'\n').count() + 1,
                            column: before.rsplit('\n').next().unwrap().chars().count() + 1,
                        };
                        let origin = mapped.source_map.lookup(position).unwrap();
                        assert_eq!(mapped.source_map.sources[origin.original.source].path, "proof/metadata.ncidl");
                        assert_eq!(origin.original.range.start.line, 1);
                    }
                    _ => panic!("unknown metadata collector"),
                }
            })
            .unwrap()
            .join()
            .unwrap();
        return;
    }
    let mut failures = Vec::new();
    for case in ["ancestry", "methods", "public"] {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", TEST, "--nocapture", "--test-threads=1"]).env(SELECT, case);
        let output = crate::bounded_child::run(command);
        if !output.status.success() {
            failures.push(format!("{case}: {}\n{}", output.status, String::from_utf8_lossy(&output.stderr)));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
