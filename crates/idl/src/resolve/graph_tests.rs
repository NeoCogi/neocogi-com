//! Public semantic graph regressions isolate small-stack failures from the harness.
use std::{path::Path, process::Command};
use crate::{Module, ModuleGraphError, ResolvedModules, parse_module, resolve_modules};

/// More nominal edges than the original 513-record crashing input; each source
/// expression stays shallow and each successful inline layout remains one byte.
const EDGES: usize = 1024;

/// Builds ordered or cyclic local layout edges without deep source syntax.
fn layout_module(interface: bool, cycle: bool, reverse: bool) -> Module {
    let mut lines = (0..=EDGES)
        .map(|index| {
            let next = (index < EDGES).then_some(index + 1).or(cycle.then_some(0));
            if interface {
                match next {
                    Some(next) => format!("interface A{index:04}: A{next:04} {{}}\n"),
                    None => format!("interface A{index:04} {{}}\n"),
                }
            } else {
                match next {
                    Some(next) => format!("struct A{index:04} {{ next: A{next:04} }}\n"),
                    None => format!("struct A{index:04} {{ value: u8 }}\n"),
                }
            }
        })
        .collect::<Vec<_>>();
    if reverse {
        lines.reverse();
    }
    parse_module(Path::new("proof/layout.ncidl"), &lines.concat()).unwrap()
}

/// Builds a module chain whose dependency order must determine fingerprints,
/// independent of the supplied module order. No layout edge crosses modules.
fn dependency_modules(cycle: bool) -> Vec<Module> {
    (0..=EDGES)
        .map(|index| {
            let next = (index < EDGES).then_some(index + 1).or(cycle.then_some(0));
            let dependency = next.map_or_else(String::new, |next| format!("extern proof.m{next:04};\n"));
            parse_module(
                Path::new(&format!("proof/m{index:04}.ncidl")),
                &format!("{dependency}struct Value {{ value: u8 }}\n"),
            )
            .unwrap()
        })
        .collect()
}

/// Checks the whole canonical graph, not only the success status of traversal.
fn assert_same_identities(first: &ResolvedModules, second: &ResolvedModules) {
    assert_eq!(first.modules().len(), second.modules().len());
    for (first, second) in first.modules().iter().zip(second.modules()) {
        assert_eq!(first.module_id, second.module_id);
        assert_eq!(first.abi_fingerprint, second.abi_fingerprint);
        assert_eq!(first.declarations, second.declarations);
        assert_eq!(first.dependencies, second.dependencies);
    }
}

/// Frozen with the recursive resolver before this traversal change. A diamond,
/// an unused external edge and a disconnected component retain exact digest bytes,
/// including canonical ordering when source dependency/input order differs.
#[test]
fn authored_graph_fingerprints_match_the_previous_encoding() {
    let mut modules = [
        ("proof/leaf.ncidl", "struct Value { value: u32 }"),
        ("proof/left.ncidl", "extern proof.leaf; struct Left { value: u8 }"),
        ("proof/right.ncidl", "extern proof.leaf; struct Right { value: bool }"),
        ("proof/root.ncidl", "extern proof.right; extern proof.left; interface Root {}"),
        ("proof/idle.ncidl", "struct Idle { value: u8 }"),
    ]
    .into_iter()
    .map(|(path, source)| parse_module(Path::new(path), source).unwrap())
    .collect::<Vec<_>>();
    let first = resolve_modules(modules.clone()).unwrap();
    modules.reverse();
    let reversed = resolve_modules(modules).unwrap();
    assert_same_identities(&first, &reversed);
    for (name, expected) in [
        ("proof.idle", "386a227802b7203725e79d6d746bba833e15ab04ff2f5a8490067166a63bd8ef"),
        ("proof.leaf", "969818735a138aeef7196e447a25c57348622ed4d2e3e3711e35bcafa533bfd2"),
        ("proof.left", "a9f47eed602f2d8c17171c8236b6c619d0b080d8b2f667ee62a3dd01f463561a"),
        ("proof.right", "855e7fc0041fc199852d48c4e87d461a60657157131daa207b4a26c45463d2f2"),
        ("proof.root", "04eec737b7d22c873617f4149541bdbcbe749ab15771845b34feec7314ea084e"),
    ] {
        assert_eq!(first.module(name).unwrap().fingerprint_hex(), expected, "{name}");
    }
}

/// Long finite source graphs succeed and late cycles remain structured errors
/// on the caller stack used by MSVC build scripts. Child watchdogs turn an
/// accidental abort/hang into an ordinary failing regression on every native OS.
#[test]
fn authored_graphs_resolve_on_a_small_caller_stack() {
    const SELECT: &str = "NEOCOGI_IDL_AUTHORED_GRAPH_CASE";
    const TEST: &str = "resolve::graph_tests::authored_graphs_resolve_on_a_small_caller_stack";
    if let Ok(case) = std::env::var(SELECT) {
        #[cfg(target_os = "linux")]
        crate::bounded_child::cap();
        std::thread::Builder::new()
            .stack_size(1024 * 1024)
            .spawn(move || match case.as_str() {
                "records" | "interfaces" => {
                    let interface = case == "interfaces";
                    let first = resolve_modules(vec![layout_module(interface, false, false)]).unwrap();
                    assert_eq!(first.module("proof.layout").unwrap().declarations.len(), EDGES + 1);
                    let reverse = resolve_modules(vec![layout_module(interface, false, true)]).unwrap();
                    assert_same_identities(&first, &reverse);
                }
                "record-cycle" | "interface-cycle" => {
                    let module = layout_module(case == "interface-cycle", true, false);
                    let ModuleGraphError::Resolution(error) = resolve_modules(vec![module]).unwrap_err() else {
                        panic!("expected semantic cycle error")
                    };
                    let diagnostic = &error.diagnostics[0];
                    assert_eq!(diagnostic.code, "EIDL063");
                    assert!(diagnostic.message.ends_with("A1024 -> A0000"));
                    assert_eq!(diagnostic.labels[0].span.path(), "proof/layout.ncidl");
                    assert_eq!(diagnostic.labels[0].span.start().line, EDGES + 1);
                }
                "modules" => {
                    let mut modules = dependency_modules(false);
                    let first = resolve_modules(modules.clone()).unwrap();
                    modules.reverse();
                    let reverse = resolve_modules(modules).unwrap();
                    assert_same_identities(&first, &reverse);
                    for index in 0..EDGES {
                        let current = first.module(&format!("proof.m{index:04}")).unwrap();
                        let next = first.module(&format!("proof.m{:04}", index + 1)).unwrap();
                        assert_eq!(current.dependencies[0].abi_fingerprint, next.abi_fingerprint);
                    }
                }
                "module-cycle" => {
                    let error = resolve_modules(dependency_modules(true)).unwrap_err();
                    assert!(error.to_string().contains("contract dependency cycle: proof.m0000 -> proof.m0001"));
                    assert!(error.to_string().contains("proof.m1024 -> proof.m0000"));
                }
                other => panic!("unknown graph case {other}"),
            })
            .unwrap()
            .join()
            .unwrap();
        println!("authored graph completed without native recursion");
        return;
    }
    let mut failures = Vec::new();
    for case in ["records", "record-cycle", "interfaces", "interface-cycle", "modules", "module-cycle"] {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", TEST, "--nocapture", "--test-threads=1"]).env(SELECT, case);
        let output = crate::bounded_child::run(command);
        if !output.status.success() {
            failures.push(format!("{case}: {}\n{}", output.status, String::from_utf8_lossy(&output.stderr)));
        } else {
            assert!(String::from_utf8_lossy(&output.stdout).contains("authored graph completed without native recursion"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
