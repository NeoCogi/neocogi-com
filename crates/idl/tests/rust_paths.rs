//! Rust package placement must not change logical contracts or wire signatures.

use std::path::Path;

use neocogi_idl::{parse_module, resolve_modules};
use neocogi_idl::generate::{RustModulePaths, generate_rust_with_paths};

/// Relocated contracts keep signatures while all imported type families move together.
#[test]
fn rust_locations_only_change_import_paths() {
    let owner = parse_module(
        Path::new("example/contract.ncidl"),
        r#"
        #[version(1)] struct Value { number: u32 }
        #[version(1)] interface IBase { fn value(&self) -> Result<Value>; }
        #[version(1)] struct Changed { value: u32 }
    "#,
    )
    .unwrap();
    let consumer = parse_module(
        Path::new("example/consumer.ncidl"),
        r#"
        extern example.contract;
        use example.contract::{Value, IBase, Changed};
        #[version(1)] interface IConsumer: IBase {
            fn changed(&mut self, value: &Changed) -> Result<()>;
            fn set(&mut self, value: Value, source: &IBase) -> Result<()>;
        }
    "#,
    )
    .unwrap();
    let graph = resolve_modules(vec![owner, consumer]).unwrap();
    let baseline = neocogi_idl::generate_rust(&graph, "example.consumer").unwrap();
    let mut paths = RustModulePaths::default();
    paths.insert("example.contract", "shared_api::contracts").unwrap();
    let relocated = generate_rust_with_paths(&graph, "example.consumer", &paths).unwrap();
    // Descriptions display the actual type printer's whitespace; they are not
    // executable paths or transported metadata. Compare those separately, while
    // every non-doc token (including signature/identity/string metadata) agrees.
    fn without_docs(tokens: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        use proc_macro2::{Delimiter, Group, TokenTree};
        let mut input = tokens.into_iter().peekable();
        let mut output = proc_macro2::TokenStream::new();
        while let Some(token) = input.next() {
            if matches!(&token, TokenTree::Punct(value) if value.as_char() == '#')
                && matches!(input.peek(), Some(TokenTree::Group(group))
                    if group.delimiter() == Delimiter::Bracket
                    && syn::parse2::<syn::Meta>(group.stream()).is_ok_and(|meta| meta.path().is_ident("doc")))
            {
                input.next();
                continue;
            }
            output.extend([match token {
                TokenTree::Group(group) => {
                    let mut rebuilt = Group::new(group.delimiter(), without_docs(group.stream()));
                    rebuilt.set_span(group.span());
                    TokenTree::Group(rebuilt)
                }
                token => token,
            }]);
        }
        output
    }
    let normalize = |text: &str| {
        let formatted = prettyplease::unparse(&syn::parse_file(text).unwrap());
        without_docs(formatted.parse().unwrap()).to_string()
    };
    assert_eq!(
        normalize(&baseline.replace("example_contract_contracts", "shared_api::contracts")),
        normalize(&relocated)
    );
    // Removing documentation cannot make the relocation regression pass: both
    // outputs must describe the actual selected supplying interface projection.
    assert!(baseline.contains("- `source` borrows a `:: example_contract_contracts :: IBaseAbi`"));
    assert!(relocated.contains("- `source` borrows a `:: shared_api :: contracts :: IBaseAbi`"));

    assert!(relocated.contains("shared_api::contracts::Changed"));
}

/// Invalid setup never creates ambiguous, relative, or generic import locations.
#[test]
fn rust_locations_reject_invalid_and_conflicting_owners() {
    let mut paths = RustModulePaths::default();
    for module in [
        "proof.type",
        "async.proof",
        "proof.gen",
        "proof._",
        "proof.r#ordinary",
        "proof.Café",
        "proof.name:v1",
        "proof..name",
    ] {
        assert!(paths.insert(module, "api::events").is_err(), "{module}");
    }
    // Raw binding metadata is independent of the logical namespace's spelling.
    paths.insert("proof.union", "api::r#type").unwrap();
    let maximum = neocogi_com_core::NC_DESCRIPTOR_QUALIFIED_NAME_MAX_LEN;
    paths.insert(&"a".repeat(maximum), "api::events").unwrap();
    assert!(paths.insert(&"a".repeat(maximum + 1), "api::events").is_err());
    for path in [
        "",
        "crate::events",
        "self::events",
        "super::events",
        "api::Thing<u32>",
        "api::type",
        "api::gen",
        "api ::events",
        "api::Évent",
        "api::::Event",
    ] {
        assert!(paths.insert("example.events", path).is_err(), "{path}");
    }
    paths.insert("example.events", "api::events").unwrap();
    paths.insert("example.events", "::api::events").unwrap();
    assert!(paths.insert("example.events", "other_api::events").is_err());
    let graph = resolve_modules(vec![
        parse_module(Path::new("example/value.ncidl"), "#[version(1)] struct Value { value: u32 }").unwrap(),
    ])
    .unwrap();
    assert!(generate_rust_with_paths(&graph, "example.value", &paths).is_err());
}

/// The final transported spelling includes the declaration, not just its module prefix.
#[test]
fn public_metadata_paths_charge_the_complete_spelling() {
    let graph = resolve_modules(vec![
        parse_module(Path::new("example/value.ncidl"), "#[version(1)] struct Value { value: u32 }").unwrap(),
    ])
    .unwrap();
    let maximum = neocogi_com_core::NC_DESCRIPTOR_RUST_PATH_MAX_LEN;
    let mut paths = RustModulePaths::default();
    paths.insert("example.value", &"a".repeat(maximum - "::Value".len())).unwrap();
    generate_rust_with_paths(&graph, "example.value", &paths).unwrap();
    let mut oversized = RustModulePaths::default();
    oversized.insert("example.value", &"a".repeat(maximum)).unwrap();
    assert!(
        generate_rust_with_paths(&graph, "example.value", &oversized)
            .unwrap_err()
            .to_string()
            .contains("invalid public Rust path")
    );
}
