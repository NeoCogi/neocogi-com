//! Literal backend prose, authored provenance and canonical bootstrap roots.
use std::{fs, path::Path, process::Command};
use syn::visit::Visit;

/// Collect actual doc values throughout traits, clients and raw tables.
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
        /// Authored α "quotes" and :: neocogi_com remain literal prose.
        #[version(1)] interface Owned {}
        #[version(1)] struct Payload { text: String }
        #[version(1)] interface Shapes {
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
        "Authored α \"quotes\" and :: neocogi_com remain literal prose.",
        "Calls [`Shapes::echo`]",
        "- `raw_0`",
        "- `value`",
        "- `object`",
        "- `input`",
        "- `output`",
        "- `owner`",
        "On success, returns `()`",
        "On success, adopts the owned",
        "# Safety",
    ] {
        assert!(docs.text.iter().any(|text| text.contains(expected)), "missing {expected}");
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
                    #[neocogi_com::com_object(interfaces(Call))] struct Provider;
                    impl Call for Provider {{ fn echo(&self, raw_0: [u8;2]) -> Result<[u8;2], neocogi_com::NcStatus> {{ Ok(raw_0) }} }}
                    #[test] fn real_call() {{
                        let owner = Provider::from_value(Provider).query::<CallAbi>().unwrap();
                        ::core::assert_eq!(owner.echo([17,73]), Ok([17,73]));
                    }}
                }}
            "#
            )
            .replace(
                "$CALL_BINDING",
                &super::name_ownership_tests::contract("#[version(1)] interface Call { fn echo(&self, raw_0: [u8;2]) -> Result<[u8;2]>; }"),
            );
            (format!("{poison}, printed={printed}"), source)
        })
    });
    super::name_ownership_tests::run_consumers("literal-method-doc-consumers", cases);
}

/// Bootstrap's ABI half needs only core, including executable support paths inside
/// macro proofs. Canonical paths and literal sentinels survive without adaptation.
#[test]
fn canonical_bootstrap_with_consuming_proofs_builds_against_core_only() {
    let consumer = super::test_consumer::Consumer::new("bootstrap-core-root-control");
    let source = r#"
        /// Literal :: neocogi_com text; link [`::neocogi_com::ComBox`].
        #[version(1)] interface Owned {}
        #[version(1)] interface Takes { fn take(&mut self, owner: ComBox<Owned>) -> Result<u32>; }
    "#;
    let graph = crate::resolve_modules(vec![crate::parse_module(Path::new("proof/bootstrap.ncidl"), source).unwrap()]).unwrap();
    let generated = crate::generate::generate_bootstrap(&graph, "proof.bootstrap").unwrap();
    assert!(generated.abi.contains("::neocogi_com::ComValue"));
    assert!(generated.abi.contains("Literal :: neocogi_com text"));
    assert!(generated.abi.contains("MAX_COST.is_some()"));
    let mut docs = Docs::default();
    docs.visit_file(&syn::parse_file(&generated.abi).unwrap());
    assert!(docs.text.iter().any(|text| text.contains("[`::neocogi_com::ComBox`]")));
    let manifest = consumer.directory.path().join("Cargo.toml");
    fs::write(&manifest, format!("[package]\nname='bootstrap-core-root-control'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com-core={{path={:?},default-features=false}}\n", consumer.core)).unwrap();
    fs::write(consumer.directory.path().join("src/abi.rs"), &generated.abi).unwrap();
    fs::write(
        consumer.directory.path().join("src/lib.rs"),
        "#![no_std]\n#![allow(dead_code)]\nextern crate self as neocogi_com;\npub use neocogi_com_core::*;\ninclude!(\"abi.rs\");\n",
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
