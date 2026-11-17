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
                    "reachable-interfaces" => {
                        let requested = ResolvedDeclaration {
                            module,
                            declaration,
                            identity: module.declaration("I0").unwrap(),
                        };
                        assert!(reachable_metadata::definitions(&generation, requested).unwrap().is_empty());
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
    for case in ["ancestry", "methods", "public", "reachable-interfaces"] {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", TEST, "--nocapture", "--test-threads=1"]).env(SELECT, case);
        let output = crate::bounded_child::run(command);
        if !output.status.success() {
            failures.push(format!("{case}: {}\n{}", output.status, String::from_utf8_lossy(&output.stderr)));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Reachable discovery is the production seam before syntax emission. Mixed
/// near-limit syntax is emitted only by the public syntax-worker path, whereas
/// deep borrowed discovery stays on the small semantic test caller.
#[test]
fn reachable_values_graphs_fit_a_small_stack() {
    const SELECT: &str = "NEOCOGI_IDL_REACHABLE_VALUES_CHILD";
    const TEST: &str = "generate::traversal_tests::reachable_values_graphs_fit_a_small_stack";
    if let Ok(case) = std::env::var(SELECT) {
        #[cfg(target_os = "linux")]
        crate::bounded_child::cap();
        std::thread::Builder::new().stack_size(1024 * 1024).spawn(move || {
            let (count, wrappers) = match case.as_str() {
                "shallow" => (2049, 1),
                "mixed" => (64, 95),
                "mixed-public" => (4, 95),
                "cycle" | "phantom" | "enum" => (0, 0),
                _ => panic!("unknown reachability case"),
            };
            let source = match case.as_str() {
                "cycle" => "struct Zeta { next: Vec<Zeta>, later: Alpha } struct Alpha { value:u32 } interface Root { fn read(&self, value: &Zeta) -> Result<()>; }".into(),
                "phantom" => "struct Alpha { value:u32 } interface Phantom<T> {} interface Root: Phantom<Alpha> {}".into(),
                "enum" => "struct Alpha { value:u32 } struct Zeta { value:u32 } enum Event { Empty, First(Alpha), Second { value: Zeta } } interface Root { fn read(&self, value: &Event) -> Result<()>; }".into(),
                _ => {
                    let mut source = String::from("interface Root { fn read(&self, value: &S0) -> Result<()>; }\n");
                    for i in 0..count {
                        if i+1 == count {
                            writeln!(source,"struct S{i} {{ value: bool }}").unwrap();
                        } else {
                            writeln!(source,"struct S{i} {{ value: {}S{}{} }}", "Vec<".repeat(wrappers),i+1,">".repeat(wrappers)).unwrap();
                        }
                    }
                    source
                }
            };
            let graph = crate::resolve_modules(vec![crate::parse_module(Path::new("proof/values.ncidl"),&source).unwrap()]).unwrap();
            let module = graph.module("proof.values").unwrap();
            let paths = RustModulePaths::default();
            let generation = Generation::new(&graph,&paths);
            let root = ResolvedDeclaration {
                module, declaration: module.source.declaration("Root").unwrap(), identity: module.declaration("Root").unwrap(),
            };
            let definitions = reachable_metadata::definitions(&generation,root).unwrap();
            let names = definitions.iter().map(|value| value.identity.qualified_name.as_str()).collect::<Vec<_>>();
            let mut expected = match case.as_str() {
                "cycle" => vec!["proof.values.Alpha".into(),"proof.values.Zeta".into()],
                "phantom" => vec!["proof.values.Alpha".into()],
                "enum" => vec!["proof.values.Alpha".into(),"proof.values.Event".into(),"proof.values.Zeta".into()],
                _ => (0..count).map(|i| format!("proof.values.S{i}")).collect::<Vec<_>>(),
            };
            expected.sort();
            assert_eq!(names,expected);
            if case == "mixed-public" {
                let generated = crate::generate_rust_with_source_map(&graph,"proof.values").unwrap();
                assert!(!generated.source.is_empty());
                assert_eq!(generated.source_map.module,"proof.values");
                assert!(generated.source_map.sources.iter().any(|s|s.path=="proof/values.ncidl"));
            }
        }).unwrap().join().unwrap();
        return;
    }
    let mut failures = Vec::new();
    for case in ["shallow", "mixed", "cycle", "phantom", "enum", "mixed-public"] {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", TEST, "--nocapture", "--test-threads=1"]).env(SELECT, case);
        let output = crate::bounded_child::run(command);
        if !output.status.success() {
            failures.push(format!("{case}: {}\n{}", output.status, String::from_utf8_lossy(&output.stderr)));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
