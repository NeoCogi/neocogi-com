//! Literal backend prose, authored provenance and canonical bootstrap roots.
use std::{fs, path::Path, process::Command};
use syn::visit::Visit;

/// Valid deeply nested ABI types must parse, format and map on an MSVC-sized caller
/// stack. Keep the source self-contained so extracted SDK tests exercise it too.
#[test]
fn deeply_nested_output_types_generate_and_map_on_a_one_mebibyte_stack() {
    std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(|| {
            let element = (0..65).fold(String::from("u8"), |element, _| format!("[{element}; 1]"));
            let methods = format!(
                "interface Deep {{\nfn ordinary(&self, output: &mut [{element}]) -> Result<()>;\nfn consuming(&self, owner: ComBox<Owned>, output: &mut [{element}]) -> Result<ComBox<Owned>>;\n}}"
            );
            let owner = "interface Owned {}";
            let single = crate::parse_module(Path::new("proof/deep.ncidl"), &format!("{methods}\n{owner}")).unwrap();
            let parts = crate::parse_module_parts(
                Path::new("proof/deep"),
                &[
                    crate::ModuleSource { path: Path::new("proof/deep/methods.ncidl"), text: &methods },
                    crate::ModuleSource { path: Path::new("proof/deep/owned.ncidl"), text: owner },
                ],
            )
            .unwrap();
            for module in [single, parts] {
                let graph = crate::resolve_modules(vec![module]).unwrap();
                let bootstrap = crate::generate::generate_bootstrap(&graph, "proof.deep").unwrap();
                assert!(bootstrap.abi.contains("pub trait Deep"));
                assert!(bootstrap.provider.contains("ordinary"));
                let generated = crate::generate_rust_with_source_map(&graph, "proof.deep").unwrap();
                assert!(generated.source.contains("pub trait Deep"));
                for (name, line) in [("::ordinary", 2), ("::consuming", 3)] {
                    assert!(generated.source_map.mappings.iter().any(|mapping| {
                        mapping.kind == crate::SourceMapKind::Method && mapping.name.ends_with(name) && mapping.original.range.start.line == line
                    }));
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

/// Parser-local spans become owned diagnostics before the syntax worker exits.
/// Deep malformed inputs must preserve both duplicate labels on a small caller.
#[test]
fn deeply_nested_duplicate_parameters_keep_their_source_after_worker_exit() {
    let error = std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(|| {
            let element = (0..65).fold(String::from("u8"), |element, _| format!("[{element}; 1]"));
            let source = format!("interface Deep {{\nfn ordinary(&self,\noutput: &mut [{element}],\noutput: u8) -> Result<()>;\n}}");
            crate::parse_module(Path::new("proof/deep.ncidl"), &source).unwrap_err()
        })
        .unwrap()
        .join()
        .unwrap();
    let crate::ParseError::Source(diagnostic) = error else {
        panic!("expected an authored diagnostic")
    };
    assert_eq!(diagnostic.code, "EIDL017");
    assert_eq!(diagnostic.labels.len(), 2);
    for (label, line) in diagnostic.labels.iter().zip([4, 3]) {
        assert_eq!(label.span.path(), "proof/deep.ncidl");
        assert_eq!(label.span.start(), crate::SourcePosition { line, column: 1 });
        assert_eq!(label.span.end(), crate::SourcePosition { line, column: 7 });
    }
    assert!(diagnostic.to_string().contains("first declared here"));
}

/// Collect actual doc values throughout traits, wrappers and raw tables.
#[derive(Default)]
struct Docs {
    /// Literal descriptions in emitted order; authored strings stay independent.
    text: Vec<String>,
}
impl<'ast> Visit<'ast> for Docs {
    fn visit_attribute(&mut self, attribute: &'ast syn::Attribute) {
        if attribute.path().is_ident("doc") {
            let syn::Meta::NameValue(value) = &attribute.meta else {
                // Visibility controls such as doc(hidden) contain no prose.
                return;
            };
            let syn::Expr::Lit(value) = &value.value else {
                panic!("backend emitted executable doc expression")
            };
            let syn::Lit::Str(value) = &value.lit else { panic!("doc string required") };
            self.text.push(value.value());
        }
        syn::visit::visit_attribute(self, attribute);
    }
}

/// All ownership/argument/return branches produce literals; poisoned consumer
/// macros cannot replace their meaning. Real calls still use the emitted ABI.
#[test]
fn descriptions_are_literals_across_all_method_shapes_and_printed_consumers() {
    let source = r#"
        /// Authored α "quotes" and :: neocogi_com_sdk remain literal prose.
        interface Owned {}
        struct Payload { text: String }
        interface Shapes {
            /// Authored method documentation stays before generated descriptions.
            fn echo(&self, raw_0: [u8; 2]) -> Result<[u8; 2]>;
            fn borrow(&self, value: &Payload, object: &Owned) -> Result<()>;
            fn buffers(&self, input: &[u32], output: &mut [u32]) -> Result<ComPtr<Owned>>;
            fn consume(&mut self, owner: ComBox<Owned>) -> Result<u32>;
        }
    "#;
    let graph = crate::resolve_modules(vec![crate::parse_module(Path::new("proof/docs.ncidl"), source).unwrap()]).unwrap();
    let generated = crate::generate_rust_with_source_map(&graph, "proof.docs").unwrap();
    let file = syn::parse_file(&generated.source).unwrap();
    let mut docs = Docs::default();
    docs.visit_file(&file);
    for expected in [
        "Authored α \"quotes\" and :: neocogi_com_sdk remain literal prose.",
        "Calls [`Shapes::echo`]",
        "- `raw_0`",
        "- `value`",
        "- `object`",
        "- `input`",
        "- `output`",
        "- `owner`",
        "On success, returns `()`",
        "The caller receives one shared owner",
        "# Safety",
    ] {
        assert!(docs.text.iter().any(|text| text.contains(expected)), "missing {expected}");
    }
    // Return prose must begin its own paragraph after argument bullet lists.
    // Literal descriptions now participate in downstream Markdown/Clippy checks.
    for pair in docs.text.windows(2) {
        if pair[1].starts_with("On success,") {
            assert!(pair[0].is_empty(), "return paragraph continues an argument list: {pair:?}");
        }
    }
    assert!(!docs.text.iter().any(|text| text.contains("Preparation discards")));
    assert!(!generated.source_map.mappings.is_empty(), "final formatted source retains origins");
    let file = prettyplease::unparse(&file);
    let cases = ["concat", "stringify", "both"].into_iter().flat_map(|poison| {
        let file = file.clone();
        let ordinary = generated.source.clone();
        [false, true].map(move |printed| {
            let mut macros = String::new();
            for name in ["concat", "stringify"] {
                if poison == name || poison == "both" {
                    macros += &format!("macro_rules! {name} {{ ($($input:tt)*) => {{ ::core::compile_error!(\"consumer doc macro selected\") }}; }}\n");
                }
            }
            let binding = if printed { &file } else { &ordinary };
            let source = format!(
                r#"
                #![allow(dead_code, unused_macros)]
                {macros}
                {binding}
                // Shapes need not have a provider to exercise every ABI declaration.
                // A separate actual provider proves this consumer can make COM calls.
                mod calls {{
                    $CALL_BINDING
                    #[neocogi_com_sdk::com_object(interfaces(Call))] struct Provider;
                    impl Call for Provider {{ fn echo(&self, raw_0: [u8;2]) -> Result<[u8;2], neocogi_com_sdk::NcStatus> {{ Ok(raw_0) }} }}
                    #[test] fn real_call() {{
                        let owner = Provider::from_value(Provider).query::<CallAbi>().unwrap();
                        ::core::assert_eq!(owner.echo([17,73]), Ok([17,73]));
                    }}
                }}
            "#
            )
            .replace(
                "$CALL_BINDING",
                &super::name_ownership_tests::contract("interface Call { fn echo(&self, raw_0: [u8;2]) -> Result<[u8;2]>; }"),
            );
            (format!("{poison}, printed={printed}"), source)
        })
    });
    super::name_ownership_tests::run_consumers("literal-method-doc-consumers", cases);
}

/// Extracts literal attributes on a selected definition rather than searching
/// unrelated documentation elsewhere in the same generated module.
fn definition_docs(attributes: &[syn::Attribute]) -> String {
    let mut docs = Docs::default();
    for attribute in attributes {
        docs.visit_attribute(attribute);
    }
    docs.text.join("\n")
}

/// Both scoped caller projections document the effect of a visitor panic on
/// their own public generated helper.
#[test]
fn scoped_caller_projections_document_the_visitor_panic_boundary() {
    /// The printed helpers live inside a private binding scope.
    #[derive(Default)]
    struct Methods<'a>(Vec<&'a syn::ImplItemFn>);
    impl<'a> Visit<'a> for Methods<'a> {
        fn visit_impl_item_fn(&mut self, method: &'a syn::ImplItemFn) {
            self.0.push(method);
        }
    }

    let source = "interface Loan {} interface Service {
        #[scoped] fn direct(&self) -> Option<&Loan>;
        #[scoped] fn checked(&self) -> Result<Option<&Loan>>;
    }";
    let module = crate::parse_module(Path::new("proof/scoped.ncidl"), source).unwrap();
    let graph = crate::resolve_modules(vec![module]).unwrap();
    let generated = crate::generate_rust(&graph, "proof.scoped").unwrap();
    let file = syn::parse_file(&generated).unwrap();
    let mut methods = Methods::default();
    methods.visit_file(&file);
    for helper in ["with_direct", "with_checked"] {
        let method = methods
            .0
            .iter()
            .find(|method| method.sig.ident == helper)
            .unwrap_or_else(|| panic!("missing generated {helper}"));
        let docs = definition_docs(&method.attrs);
        let lines = docs.lines().map(str::trim).collect::<Vec<_>>();
        assert!(lines.windows(3).any(|lines| lines == ["", "# Panics", ""]), "{docs}");
        assert!(docs.contains("An uncaught panic in the visitor terminates the process"), "{docs}");
        assert!(docs.contains("non-unwinding C callback boundary"), "{docs}");
    }
}

/// The exact parsed ABI slot supplies expected physical types. Native-enum and
/// scalar-domain buffers, consuming inputs and each returned owner category
/// must agree with their descriptions in normal and printed executable consumers.
#[test]
fn raw_documentation_matches_physical_slots_and_returned_ownership() {
    use quote::ToTokens;
    let source = r#"
        enum Action { Idle, Set(u32) }
        enum Mode: u8 { Off = 0, On = 1 }
        struct Payload { value: u32 }
        interface Owned {}
        interface Shapes {
            fn actions(&self, input: &[Action], output: &mut [Action]) -> Result<()>;
            fn modes(&self, input: &[Mode]) -> Result<Mode>;
            fn borrow(&self, value: &Payload, object: &Owned) -> Result<()>;
            fn consume(&self, owner: ComBox<Owned>) -> Result<()>;
            fn number(&self, value: u32) -> Result<u32>;
            fn shared(&self) -> Result<ComPtr<Owned>>;
            fn exclusive(&self) -> Result<ComBox<Owned>>;
            fn weak_shared(&self) -> Result<WeakComPtr<Owned>>;
            fn borrowed(&self) -> Result<&Owned>;
        }
        interface ExclusiveLoans {
            fn borrowed_mut(&mut self) -> Option<&mut Owned>;
            #[scoped]
            fn scoped_mut(&self) -> Result<Option<&mut Owned>>;
        }
    "#;
    let graph = crate::resolve_modules(vec![crate::parse_module(Path::new("proof/physical.ncidl"), source).unwrap()]).unwrap();
    let generated = crate::generate_rust(&graph, "proof.physical").unwrap();
    let file = syn::parse_file(&generated).unwrap();
    let table = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Struct(item) if item.ident == "ShapesVTable" => Some(item),
            _ => None,
        })
        .unwrap();
    let raw_borrow = table
        .fields
        .iter()
        .find(|field| field.ident.as_ref().is_some_and(|ident| ident == "borrow"))
        .unwrap();
    assert!(definition_docs(&raw_borrow.attrs).contains("lends live shared access through the call"));
    assert!(generated.contains("is a required shared loan"));
    for (table, method, receiver, loan, presence) in [
        (
            "ShapesVTable",
            "borrowed",
            "shared target access for the call",
            "lends shared target access and transfers no ownership",
            "A successful required result has two non-null pointers",
        ),
        (
            "ExclusiveLoansVTable",
            "borrowed_mut",
            "exclusive target access across all projections for the call",
            "lends exclusive target access and transfers no ownership",
            "Both-null denotes successful absence",
        ),
        (
            "ExclusiveLoansVTable",
            "scoped_mut",
            "shared target access for the call",
            "lends exclusive target access for the complete callback",
            "Both-null denotes successful absence",
        ),
    ] {
        let slot = file
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Struct(item) if item.ident == table => item.fields.iter().find(|field| field.ident.as_ref().is_some_and(|ident| ident == method)),
                _ => None,
            })
            .unwrap();
        let docs = definition_docs(&slot.attrs);
        let (_, safety) = docs.split_once("# Safety").unwrap();
        for contract in [receiver, loan, presence, "must jointly obey their borrowing and threading rules"] {
            assert!(safety.contains(contract), "{method}: {contract}: {docs}");
        }
        if method != "scoped_mut" {
            assert!(safety.contains("Keep the receiver borrow active until the returned loan's last use"), "{docs}");
        }
    }
    for (method, parameters) in [
        ("actions", vec![3, 6]),
        ("modes", vec![3]),
        ("borrow", vec![1, 2]),
        ("consume", vec![1]),
        ("number", vec![1]),
    ] {
        let slot = table
            .fields
            .iter()
            .find(|field| field.ident.as_ref().is_some_and(|ident| ident == method))
            .unwrap();
        let syn::Type::BareFn(signature) = &slot.ty else {
            panic!("raw callable slot")
        };
        let docs = definition_docs(&slot.attrs);
        for parameter in parameters {
            let physical = signature.inputs[parameter].ty.to_token_stream().to_string();
            assert!(docs.contains(&format!("ABI storage `{physical}`")), "{method}: {physical}: {docs}");
        }
    }
    for (method, contract) in [
        ("shared", "one shared owner"),
        ("exclusive", "sole exclusive owner"),
        ("weak_shared", "shared weak control token"),
    ] {
        let slot = table
            .fields
            .iter()
            .find(|field| field.ident.as_ref().is_some_and(|ident| ident == method))
            .unwrap();
        let syn::Type::BareFn(signature) = &slot.ty else {
            panic!("raw callable slot")
        };
        let syn::Type::Ptr(out) = &signature.inputs.last().unwrap().ty else {
            panic!("output pointer")
        };
        let physical = out.elem.to_token_stream().to_string();
        let docs = definition_docs(&slot.attrs);
        let returned = docs.lines().find(|line| line.starts_with("On success,")).unwrap();
        assert!(returned.contains(&format!("`out` receives `{physical}`")), "{method}: {docs}");
        assert!(returned.contains(contract), "{method}: {docs}");
        if method == "weak_shared" {
            assert!(physical.contains("NcSharedWeakRaw"));
            assert!(!returned.contains("object/table pair"), "a shared weak result transfers one header token");
        }
    }
    let implementation = r#"
        #![allow(dead_code)]
        $BINDING
        use neocogi_com_sdk::*;
        #[com_object(Owned)] struct SharedValue;
        impl Owned for SharedValue {}
        #[com_object(interfaces(Owned), ownership = exclusive)] struct UniqueValue;
        impl Owned for UniqueValue {}
        #[com_object(Shapes)] struct Provider { shared: ComPtr<OwnedAbi> }
        impl Shapes for Provider {
            fn actions(&self, input: NcCopyFromBufferOffset<'_, Action>, mut output: NcOutputWriter<'_, Action>) -> Result<(), NcStatus> {
                output.extend_from_slice(input.as_slice())
            }
            fn modes(&self, input: NcCopyFromBufferOffset<'_, Mode>) -> Result<Mode, NcStatus> { Ok(input.as_slice()[0]) }
            fn borrow(&self, value: NcValueRef<'_, Payload>, _object: ComRef<'_, OwnedAbi>) -> Result<(), NcStatus> {
                assert_eq!(value.to_owned()?.value, 73); Ok(())
            }
            fn consume(&self, _owner: ComBox<OwnedAbi>) -> Result<(), (NcStatus, ComBox<OwnedAbi>)> { Ok(()) }
            fn number(&self, value: u32) -> Result<u32, NcStatus> { Ok(value) }
            fn shared(&self) -> Result<ComPtr<OwnedAbi>, NcStatus> { Ok(self.shared.clone()) }
            fn exclusive(&self) -> Result<ComBox<OwnedAbi>, NcStatus> {
                UniqueValue::into_box(UniqueValue).query().map_err(|(status, _)| status)
            }
            fn weak_shared(&self) -> Result<WeakComPtr<OwnedAbi>, NcStatus> { self.shared.downgrade() }
            fn borrowed(&self) -> Result<ComRef<'_, OwnedAbi>, NcStatus> { Ok(self.shared.as_com_ref()) }
        }
        #[test] fn actual_documented_shapes_cross_com() {
            let shared = SharedValue::from_value(SharedValue).query::<OwnedAbi>().unwrap();
            let calls = Provider::from_value(Provider { shared }).query::<ShapesAbi>().unwrap();
            let input = [Action::Set(73), Action::Idle];
            let mut output = NcOutputBuffer::new();
            calls.actions(NcCopyFromBufferOffset::new(0, &input), output.output(0, 2)).unwrap();
            assert_eq!(output.as_slice(), &input);
            assert_eq!(calls.modes(NcCopyFromBufferOffset::new(0, &[Mode::On])), Ok(Mode::On));
            let shared = calls.shared().unwrap();
            calls.borrow(NcValueRef::new(&Payload { value: 73 }).unwrap(), shared.as_com_ref()).unwrap();
            assert_eq!(calls.borrowed().unwrap().as_raw(), shared.as_raw());
            let unique = calls.exclusive().unwrap();
            calls.consume(unique).unwrap();
            assert_eq!(calls.number(37), Ok(37));
            let weak = calls.weak_shared().unwrap();
            assert!(weak.upgrade().unwrap().is_some());
            drop(shared);
            drop(calls);
            assert!(weak.upgrade().unwrap().is_none());
        }
    "#;
    super::name_ownership_tests::run_consumers(
        "physical-method-doc-consumers",
        [
            ("normal".to_owned(), implementation.replace("$BINDING", &generated)),
            ("printed".to_owned(), implementation.replace("$BINDING", &prettyplease::unparse(&file))),
        ],
    );
}

/// Bootstrap's ABI half needs only core, including executable support paths inside
/// macro proofs. Generated paths target the including crate and authored docs stay intact.
#[test]
fn crate_relative_bootstrap_with_consuming_proofs_builds_against_core_only() {
    let consumer = super::test_consumer::Consumer::new("bootstrap-core-root-control");
    let source = r#"
        /// Literal :: neocogi_com text; link [`::neocogi_com::ComBox`](crate::ComBox).
        interface Owned {}
        interface Takes { fn take(&mut self, owner: ComBox<Owned>, output: &mut [u32]) -> Result<u32>; }
    "#;
    let graph = crate::resolve_modules(vec![crate::parse_module(Path::new("proof/bootstrap.ncidl"), source).unwrap()]).unwrap();
    let generated = crate::generate::generate_bootstrap(&graph, "proof.bootstrap").unwrap();
    assert!(generated.abi.contains("crate::ComValue"));
    assert!(generated.abi.contains("[`crate::NcOutput`]"));
    assert!(!generated.abi.contains("::neocogi_com_sdk::ComValue"));
    assert!(generated.abi.contains("Literal :: neocogi_com text"));
    assert!(generated.abi.contains("MAX_COST.is_some()"));
    let mut docs = Docs::default();
    docs.visit_file(&syn::parse_file(&generated.abi).unwrap());
    assert!(docs.text.iter().any(|text| text.contains("[`::neocogi_com::ComBox`]")));
    let manifest = consumer.directory.path().join("Cargo.toml");
    fs::write(&manifest, format!("[package]\nname='bootstrap-core-root-control'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?},default-features=false}}\n", consumer.core)).unwrap();
    fs::write(consumer.directory.path().join("src/abi.rs"), &generated.abi).unwrap();
    fs::write(
        consumer.directory.path().join("src/lib.rs"),
        "#![no_std]\n#![allow(dead_code)]\npub use neocogi_com::*;\ninclude!(\"abi.rs\");\n",
    )
    .unwrap();
    for command in [vec!["check", "--offline"], vec!["doc", "--offline", "--no-deps"]] {
        let output = Command::new("cargo")
            .args(command)
            .arg("--manifest-path")
            .arg(&manifest)
            .env("RUSTDOCFLAGS", "-D warnings -D rustdoc::broken_intra_doc_links")
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    }
}
