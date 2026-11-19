//! Admitted ownership and generated Rust are exercised by ordinary SDK consumers.
use std::{fs, path::Path, process::Command};
use super::test_consumer::Consumer;

/// A detached model remains useful, but no public operation can reinstall it in
/// an admitted source or resolved graph. Diagnose actual privacy/type errors.
#[test]
fn recursive_model_installation_is_private_to_the_compiler() {
    let consumer = Consumer::new("admitted-model-owner");
    let root = consumer.directory.path();
    let manifest = root.join("Cargo.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str(&format!("neocogi-idl={{path={:?}}}\n", Path::new(env!("CARGO_MANIFEST_DIR"))));
    fs::write(&manifest, text).unwrap();
    let prefix = "use neocogi_idl::*; fn source() -> Module { parse_module(std::path::Path::new(\"proof/owner.ncidl\"), \"struct Item { value: u8 } struct Template<T> { value: T }\").unwrap() }";
    let run = |body: &str| {
        fs::write(root.join("src/lib.rs"), format!("{prefix}\n{body}")).unwrap();
        Command::new("cargo")
            .args(["check", "--offline", "--message-format=json", "--manifest-path"])
            .arg(&manifest)
            .output()
            .unwrap()
    };
    let positive = run(
        "pub fn inspect() { let module = source(); let _detached = module.declarations().to_vec(); let _templates = module.templates().to_vec(); let graph = resolve_modules(vec![module.clone()]).unwrap(); let _values = graph.values().clone(); assert!(graph.module(\"proof.owner\").unwrap().source.declaration(\"Item\").is_some()); }",
    );
    assert!(positive.status.success(), "{}", String::from_utf8_lossy(&positive.stderr));
    for (body, code) in [
        ("pub fn bad() { source().declarations.clear(); }", "E0616"),
        ("pub fn bad() { source().templates.clear(); }", "E0616"),
        ("pub fn bad() { let mut m = source(); m.declarations = m.declarations().to_vec(); }", "E0616"),
        (
            "pub fn bad() { let m = source(); let _ = Module { name: m.name, sources: m.sources, warnings: m.warnings, externals: m.externals, imports: m.imports, declarations: vec![], templates: vec![] }; }",
            "E0451",
        ),
        ("pub fn bad() { let m = source(); let _ = Module { name: m.name.clone(), ..m }; }", "E0451"),
        ("pub fn bad() { resolve_modules(vec![source()]).unwrap().values.definitions.clear(); }", "E0616"),
        ("pub fn bad() { let mut m = source(); m.declarations()[0].name.clear(); }", "E0596"),
        (
            "pub fn bad() { let mut g = resolve_modules(vec![source()]).unwrap(); g.values().definitions.clear(); }",
            "E0596",
        ),
    ] {
        let output = run(body);
        assert!(!output.status.success(), "unchecked installation compiled: {body}");
        let codes = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter_map(|v| v.get("message")?.get("code")?.get("code")?.as_str().map(str::to_owned))
            .collect::<Vec<_>>();
        assert!(
            codes.iter().any(|actual| actual == code),
            "{body}: {codes:?}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// Flat field breadth must stay flat in the emitted borrowed-layout proof. Two
/// independently generated bindings still negotiate and execute a real COM call.
#[test]
fn broad_copied_values_generate_shallow_proofs_and_call_across_bindings() {
    let fields = (0..512).map(|index| format!("field_{index}: u8,")).collect::<String>();
    let source = format!("struct Wide {{ {fields} }} interface Echo {{ fn echo(&self, value: Wide) -> Result<Wide>; }}");
    let emit = || {
        let graph = crate::resolve_modules(vec![crate::parse_module(Path::new("proof/wide.ncidl"), &source).unwrap()]).unwrap();
        crate::generate_rust_with_source_map(&graph, "proof.wide").unwrap().source
    };
    let consumer = Consumer::new("wide-layout-proof");
    let root = consumer.directory.path();
    let generated = emit();
    // Inspect expression shape, independently of the concrete accumulator spelling.
    // A flat source must not manufacture a linear-depth binary-expression AST.
    #[derive(Default)]
    struct BinaryDepth {
        active: usize,
        maximum: usize,
    }
    impl<'ast> syn::visit::Visit<'ast> for BinaryDepth {
        fn visit_expr_binary(&mut self, expression: &'ast syn::ExprBinary) {
            self.active += 1;
            self.maximum = self.maximum.max(self.active);
            syn::visit::visit_expr_binary(self, expression);
            self.active -= 1;
        }
    }
    let mut depth = BinaryDepth::default();
    syn::visit::Visit::visit_file(&mut depth, &syn::parse_file(&generated).unwrap());
    assert!(depth.maximum < 16, "flat fields produced {} nested binary expressions", depth.maximum);
    fs::write(root.join("src/producer.rs"), generated).unwrap();
    fs::write(root.join("src/consumer.rs"), emit()).unwrap();
    let initializers = (0..512).map(|index| format!("field_{index}: {}u8,", index % 251)).collect::<String>();
    fs::write(
        root.join("src/lib.rs"),
        format!(
            r#"
        mod producer; mod consumer;
        #[neocogi_com::com_object(producer::Echo)] struct Probe;
        impl producer::Echo for Probe {{
            fn echo(&self, value: producer::Wide) -> Result<producer::Wide, neocogi_com::NcStatus> {{ Ok(value) }}
        }}
        #[test] fn real_call() {{
            use neocogi_com::ComValue;
            assert!(producer::Wide::BORROW_ABI_REPR);
            assert!(consumer::Wide::BORROW_ABI_REPR);
            assert_eq!(producer::ABI_FINGERPRINT, consumer::ABI_FINGERPRINT);
            let echo = Probe::from_value(Probe).query::<consumer::EchoAbi>().unwrap();
            let result = echo.echo(consumer::Wide {{ {initializers} }}).unwrap();
            assert_eq!((result.field_0, result.field_511), (0, 9));
        }}
    "#
        ),
    )
    .unwrap();
    for profile in [vec!["test", "--offline"], vec!["test", "--offline", "--release"]] {
        let output = Command::new("cargo")
            .args(profile)
            .arg("--manifest-path")
            .arg(root.join("Cargo.toml"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("test real_call ... ok"));
    }
}

/// The maximum admitted inline shapes include emitted schema call/reference
/// amplification, formatted mappings, supported bootstrap and caller-side drops.
#[cfg(not(miri))]
#[test]
fn maximum_inline_height_generates_and_drops_on_small_callers() {
    const CHILD: &str = "NC_IDL_MAX_GENERATION_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "backend_tests::nesting_admission_tests::maximum_inline_height_generates_and_drops_on_small_callers",
                "--nocapture",
            ])
            .env(CHILD, "1");
        let output = crate::bounded_child::run(command);
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    #[cfg(target_os = "linux")]
    crate::bounded_child::cap();
    std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(|| {
            let element = format!("{}u8{}", "[".repeat(94), "; 1]".repeat(94));
            for text in [
                format!("struct Deep {{ value: [{element}; 1] }}"),
                format!("interface Deep {{ fn read(&self, output: &mut [{element}]) -> Result<()>; }}"),
            ] {
                let graph = crate::resolve_modules(vec![crate::parse_module(Path::new("proof/max.ncidl"), &text).unwrap()]).unwrap();
                let normal = crate::generate_rust_with_source_map(&graph, "proof.max").unwrap();
                let paths = crate::generate::RustModulePaths::default();
                let scoped = crate::generate::generate_rust_with_paths(&graph, "proof.max", &paths).unwrap();
                if text.starts_with("interface") {
                    let bootstrap = crate::generate::generate_bootstrap(&graph, "proof.max").unwrap();
                    drop(bootstrap);
                }
                drop((normal, scoped, graph));
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
