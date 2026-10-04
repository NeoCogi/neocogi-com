//! Constructor helper names must be checked where they originate, before Rust emission.
use neocogi_idl::{LabelStyle, Module, ResolutionError, ResolvedModules, parse_module, parse_module_parts, resolve_modules, ModuleSource};
use std::path::Path;

/// Current public helper families of a product with a shared constructor.
const SUFFIXES: [&str; 2] = ["EventBinding", "EventPort"];
/// Small ordinary contract used to isolate naming from unrelated codec constraints.
const PRODUCT: &str = r#"
    extern neocogi.com.events;
    use neocogi.com.events::{EventBinding, IEvent};
    #[version(1)] interface IChangedEvent: IEvent {}
    #[version(1)] enum Event { Changed }
    #[version(1)] class Counter {}
    #[version(1)] factory Factory {
        fn create(&self, events: &EventBinding<Counter, Event>) -> Result<ComPtr<Counter>>;
    }
"#;

/// Parses test-owned IDL without hiding malformed syntax behind semantic assertions.
fn module(path: &str, source: &str) -> Module {
    parse_module(Path::new(path), source).unwrap()
}
/// Uses the shipped generic library instead of recreating a parallel event protocol.
fn library() -> Module {
    module("neocogi/com/events.ncidl", include_str!("../../api/idl/neocogi/com/events/events.ncidl"))
}
/// Resolves one product module with optional explicit imported declarations.
fn resolve(source: &str, dependencies: impl IntoIterator<Item = Module>) -> Result<ResolvedModules, ResolutionError> {
    let mut modules = vec![library(), module("proof/names.ncidl", source)];
    modules.extend(dependencies);
    resolve_modules(modules)
}

/// Each actual alias family must fail at IDL resolution, not downstream rustc.
#[test]
fn rejects_every_authored_constructor_helper_collision() {
    for suffix in SUFFIXES {
        let name = format!("Counter{suffix}");
        let source = format!("{PRODUCT}\n#[version(1)] struct {name} {{ value: u32 }}");
        let error = resolve(&source, []).unwrap_err();
        let diagnostic = error.diagnostics.iter().find(|d| d.code == "EIDL021" && d.message.contains(&name)).unwrap();
        assert_eq!(diagnostic.labels.len(), 3, "constructor, authored name, and product must all be located");
        let primary = diagnostic.labels.iter().find(|label| label.style == LabelStyle::Primary).unwrap();
        assert!(source.lines().nth(primary.span.start().line - 1).unwrap().contains("fn create"));
        assert!(
            diagnostic
                .labels
                .iter()
                .any(|label| source.lines().nth(label.span.start().line - 1).unwrap().contains(&format!("struct {name}")))
        );
        let rendered = error.to_string();
        assert!(rendered.contains("proof/names.ncidl"), "{rendered}");
        assert!(rendered.contains("struct"), "{rendered}");
    }
}

/// Imports occupy the same emitted module namespace as local declarations.
#[test]
fn rejects_every_imported_constructor_helper_collision() {
    for suffix in SUFFIXES {
        let name = format!("Counter{suffix}");
        let source = format!("{PRODUCT}\nextern proof.models; use proof.models::{name};");
        let error = resolve(
            &source,
            [module("proof/models.ncidl", &format!("#[version(1)] struct {name} {{ value: u32 }}"))],
        )
        .unwrap_err();
        let diagnostic = error.diagnostics.iter().find(|d| d.code == "EIDL021" && d.message.contains(&name)).unwrap();
        assert!(diagnostic.message.contains("imported name"));
        assert!(
            diagnostic
                .labels
                .iter()
                .any(|label| source.lines().nth(label.span.start().line - 1).unwrap().contains("use proof.models"))
        );
    }
}

/// A conflict can involve an ordinary COM companion as well as a source item.
#[test]
fn validates_constructor_helpers_against_other_generated_companions() {
    // The obsolete queue alias family is no longer reserved by constructors.
    let graph = resolve(&format!("{PRODUCT}\n#[version(1)] interface CounterEventQueue {{}}"), []).unwrap();
    assert!(generate_rust(&graph, "proof.names").unwrap().contains("pub trait CounterEventQueue"));
}

/// Source parts are one semantic namespace and diagnostics must retain both files.
#[test]
fn collision_labels_survive_separate_source_parts() {
    let product = parse_module_parts(
        Path::new("proof/names"),
        &[
            ModuleSource {
                path: Path::new("proof/names/product.ncidl"),
                text: PRODUCT,
            },
            ModuleSource {
                path: Path::new("proof/names/data.ncidl"),
                text: "#[version(1)] struct CounterEventBinding { value: u32 }",
            },
        ],
    )
    .unwrap();
    let error = resolve_modules(vec![library(), product]).unwrap_err();
    let rendered = error.to_string();
    assert!(rendered.contains("proof/names/product.ncidl"), "{rendered}");
    assert!(rendered.contains("proof/names/data.ncidl"), "{rendered}");
    assert!(rendered.contains("fn create"), "{rendered}");
    assert!(rendered.contains("struct CounterEventBinding"), "{rendered}");
}

/// Constructor presence alone does not reserve a global family of suffixes.
#[test]
fn products_without_event_bindings_leave_all_helper_names_available() {
    let mut source = String::from("#[version(1)] class Counter {} #[version(1)] factory Factory { fn create(&self) -> Result<ComBox<Counter>>; }");
    for suffix in SUFFIXES {
        source.push_str(&format!("#[version(1)] struct Counter{suffix} {{ value: u32 }}"));
    }
    let graph = resolve(&source, []).unwrap();
    let generated = generate_rust(&graph, "proof.names").unwrap();
    assert!(!generated.contains("impl ::neocogi_com::events::EventSource"));
    assert!(!generated.contains("pub type CounterEventBinding"));
    for suffix in SUFFIXES {
        assert!(generated.contains(&format!("pub struct Counter{suffix}")));
    }
}

/// Multiple consistent factories share one product plan rather than emitting duplicates.
#[test]
fn consistent_local_constructors_emit_one_helper_family() {
    let source = format!(
        r#"{PRODUCT}
        #[version(1)] interface AnotherFactory {{
            fn restore(&self, events: &neocogi::com::events::EventBinding<Counter, Event>) -> Result<ComPtr<Counter>>;
        }}
    "#
    );
    let graph = resolve(&source, []).unwrap();
    let generated = generate_rust(&graph, "proof.names").unwrap();
    for name in ["CounterEventBinding", "CounterEventPort"] {
        assert_eq!(generated.matches(&format!("pub type {name} =")).count(), 1);
    }
    assert_eq!(generated.matches("impl ::neocogi_com::events::EventSource for CounterAbi").count(), 1);
}

/// A downstream constructor cannot reserve names or change a dependency's metadata.
#[test]
fn imported_product_constructors_do_not_change_the_owning_module() {
    let owner = module(
        "proof/owner.ncidl",
        "#[version(1)] class Counter {} #[version(1)] struct CounterEventBinding { value: u32 }",
    );
    let source = r#"
        extern proof.owner; use proof.owner::Counter;
        extern neocogi.com.events; use neocogi.com.events::{EventBinding, IEvent};
        #[version(1)] interface IChangedEvent: IEvent {}
    #[version(1)] enum Event { Changed }
        #[version(1)] factory Factory { fn create(&self, events: &EventBinding<Counter, Event>) -> Result<ComPtr<Counter>>; }
        #[version(1)] struct CounterEventQueue { value: u32 }
    "#;
    let graph = resolve(source, [owner]).unwrap();
    for name in ["proof.owner", "proof.names"] {
        let generated = generate_rust(&graph, name).unwrap();
        assert!(!generated.contains("impl ::neocogi_com::events::EventSource for CounterAbi"));
        assert!(!generated.contains("pub type CounterEventBinding ="));
    }
}

/// Closed generic products reserve helpers for their actual concrete projection,
/// not for the erased template or for an independently guessed Rust spelling.
#[test]
fn generic_product_helpers_participate_in_name_validation() {
    let source = r#"
        extern neocogi.com.events; use neocogi.com.events::{EventBinding, IEvent};
        #[version(1)] enum Event<T> { Changed(T) }
        #[version(1)] interface Counter<T> { fn read(&self) -> Result<T>; }
        #[version(1)] factory Factory {
            fn create(&self, events: &EventBinding<Counter<u32>, Event<u32>>) -> Result<ComPtr<Counter<u32>>>;
        }
    "#;
    let graph = resolve(source, []).unwrap();
    let product = graph
        .module("proof.names")
        .unwrap()
        .source
        .declarations
        .iter()
        .find(|declaration| {
            declaration
                .specialization
                .as_ref()
                .is_some_and(|instance| instance.template.segments.last().unwrap() == "Counter")
        })
        .unwrap();
    let generated = generate_rust(&graph, "proof.names").unwrap();
    for suffix in SUFFIXES {
        let name = format!("{}{suffix}", product.name);
        assert!(generated.contains(&name), "missing {name}");
        let error = resolve(&format!("{source}\n#[version(1)] struct {name} {{ value: u32 }}"), []).unwrap_err();
        assert!(
            error
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "EIDL021" && diagnostic.message.contains(&name)),
            "{error}"
        );
    }
}

/// A malformed library adapter must fail resolution even if no local constructor
/// uses it. Generators receive only checked contracts and need no shape checker.
#[test]
fn rejects_malformed_binding_contract_before_generation() {
    let original = include_str!("../../api/idl/neocogi/com/events/events.ncidl");
    for altered in [
        original.replace("Unobserved,", "Absent,"),
        original.replace("Unobserved,", "Unobserved(u32),"),
        original.replace("Observed(ComPtr<IEventReceiver>)", "Observed"),
        original.replace("Observed(ComPtr<IEventReceiver>)", "Observed(u32)"),
    ] {
        let error = resolve_modules(vec![
            module("neocogi/com/events.ncidl", &altered),
            module(
                "proof/names.ncidl",
                &PRODUCT.replace(
                    "#[version(1)] factory Factory {\n        fn create(&self, events: &EventBinding<Counter, Event>) -> Result<ComPtr<Counter>>;\n    }",
                    "#[version(1)] struct Request { binding: EventBinding<Counter, Event> }",
                ),
            ),
        ])
        .unwrap_err();
        assert!(
            error
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "EIDL065" && !diagnostic.labels.is_empty()),
            "{error}"
        );
        assert!(error.to_string().contains("proof/names.ncidl"), "{error}");
    }
}

/// Exclusive and weak-exclusive constructor results must not reserve an unusable
/// shared-weak port alias; all four queue/binding names still require protection.
#[test]
fn exclusive_constructors_reserve_only_the_helpers_they_emit() {
    for ownership in ["ComBox", "WeakComBox"] {
        let source = PRODUCT.replace("ComPtr<Counter>", &format!("{ownership}<Counter>"));
        for suffix in &SUFFIXES[..1] {
            let name = format!("Counter{suffix}");
            for (source, dependencies) in [
                (format!("{source} #[version(1)] struct {name} {{ value: u32 }}"), vec![]),
                (
                    format!("{source} extern proof.models; use proof.models::{name};"),
                    vec![module("proof/models.ncidl", &format!("#[version(1)] struct {name} {{ value: u32 }}"))],
                ),
            ] {
                assert!(
                    resolve(&source, dependencies)
                        .unwrap_err()
                        .diagnostics
                        .iter()
                        .any(|d| d.code == "EIDL021" && d.message.contains(&name))
                );
            }
        }
        let generated = generate_rust(&resolve(&source, []).unwrap(), "proof.names").unwrap();
        assert!(!generated.contains("CounterEventPort"));
        assert!(generated.contains("impl ::neocogi_com::events::EventSource for CounterAbi"));
        for (source, dependencies) in [
            (format!("{source} #[version(1)] struct CounterEventPort {{ value: u32 }}"), vec![]),
            (
                format!("{source} extern proof.models; use proof.models::CounterEventPort;"),
                vec![module("proof/models.ncidl", "#[version(1)] struct CounterEventPort { value: u32 }")],
            ),
        ] {
            assert!(generate_rust(&resolve(&source, dependencies).unwrap(), "proof.names").is_ok());
        }
    }
}

/// A shared path still needs the port even when a consistent exclusive factory
/// is visited first. Conversely, a factory without an event binding cannot add it.
#[test]
fn mixed_constructors_preserve_one_shared_port_regardless_of_order() {
    for order in [false, true] {
        let exclusive = PRODUCT.replace("ComPtr<Counter>", "ComBox<Counter>");
        let other = r#"#[version(1)] factory AnotherFactory {
            fn create(&self, events: &EventBinding<Counter, Event>) -> Result<ComPtr<Counter>>;
        }"#;
        let source = match order {
            false => format!("{exclusive} {other}"),
            true => format!("{exclusive} {other}").replace("AnotherFactory", "ZFactory"),
        };
        let graph = resolve(&source, []).unwrap();
        let generated = generate_rust(&graph, "proof.names").unwrap();
        assert_eq!(generated.matches("pub type CounterEventPort =").count(), 1);
        assert!(resolve(&format!("{source} #[version(1)] struct CounterEventPort {{ value: u32 }}"), []).is_err());
    }
    let source = format!(
        "{} #[version(1)] factory SharedFactory {{ fn create(&self) -> Result<ComPtr<Counter>>; }}",
        PRODUCT.replace("ComPtr<Counter>", "ComBox<Counter>")
    );
    let generated = generate_rust(&resolve(&source, []).unwrap(), "proof.names").unwrap();
    assert!(!generated.contains("CounterEventPort"));
}

/// Compile accepted names and the actual queue trait aliases, then execute an
/// exclusive factory/event round trip and inspect closed-generic event metadata.
#[test]
fn accepted_helper_names_compile_and_exclusive_events_cross_the_abi() {
    use std::{fs, process::Command};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        format!(
            "[package]\nname='constructor-names-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            root.join("crates/com/api"),
        ),
    )
    .unwrap();
    for (name, ownership) in [("exclusive", "ComBox"), ("weak_only", "WeakComBox")] {
        let source = format!(
            "{} #[version(1)] struct CounterEventPort {{ value: u32 }}",
            PRODUCT
                .replace("ComPtr<Counter>", &format!("{ownership}<Counter>"))
                .replace("class Counter {}", "class Counter { fn emit(&mut self, value: &Event) -> Result<()>; }")
        );
        fs::write(
            project.path().join(format!("src/{name}.rs")),
            generate_rust(&resolve(&source, []).unwrap(), "proof.names").unwrap(),
        )
        .unwrap();
    }
    let mut plain = String::from("#[version(1)] class Counter {} #[version(1)] factory Factory { fn create(&self) -> Result<ComBox<Counter>>; }");
    for suffix in ["EventBinding", "EventQueueAbi", "EventQueue", "EventQueueVtblFor", "EventPort"] {
        plain.push_str(&format!("#[version(1)] struct Counter{suffix} {{ value: u32 }}"));
    }
    fs::write(
        project.path().join("src/plain.rs"),
        generate_rust(&resolve(&plain, []).unwrap(), "proof.names").unwrap(),
    )
    .unwrap();
    let generic = resolve(
        r#"
        extern neocogi.com.events; use neocogi.com.events::{EventBinding, IEvent};
        #[version(1)] struct Event<T> { value: T }
        #[version(1)] interface Counter<T> { fn read(&self) -> Result<T>; }
        #[version(1)] factory Factory {
            fn create(&self, events: &EventBinding<Counter<u32>, Event<u32>>) -> Result<ComPtr<Counter<u32>>>;
        }
    "#,
        [],
    )
    .unwrap();
    let product = generic
        .module("proof.names")
        .unwrap()
        .source
        .declarations
        .iter()
        .find(|declaration| {
            declaration
                .specialization
                .as_ref()
                .is_some_and(|instance| instance.template.segments.last().unwrap() == "Counter")
        })
        .unwrap();
    fs::write(project.path().join("src/generic.rs"), generate_rust(&generic, "proof.names").unwrap()).unwrap();
    fs::write(
        project.path().join("src/lib.rs"),
        include_str!("fixtures/constructor_names.rs").replace("GENERIC_PRODUCT", &product.name),
    )
    .unwrap();
    let result = Command::new("cargo")
        .args(["test", "--offline", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target/message-proof"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

/// Standalone fixtures consume the shipped common receiver from its owning crate.
fn generate_rust(graph: &neocogi_idl::ResolvedModules, module: &str) -> Result<String, neocogi_idl::GenerateError> {
    let mut paths = neocogi_idl::generate::RustModulePaths::default();
    if graph.module("neocogi.com.events").is_some() { paths.insert("neocogi.com.events", "neocogi_com::events").unwrap(); }
    neocogi_idl::generate::generate_rust_with_paths(graph, module, &paths)
}
