//! Printed bindings must preserve values after macro syntax contexts are lost.
use std::{fs, path::Path, process::Command};
use quote::quote;

/// Expands the same value/interface macros used by an IDL consumer, then prints
/// them as standalone Rust. The resulting crate cannot benefit from mixed-site
/// contexts, so its runtime checks prove structural name separation as well.
#[test]
fn printed_bindings_preserve_authored_values_and_arguments() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        format!(
            "[package]\nname='printed-hygiene-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            root.join("crates/com/api")
        ),
    )
    .unwrap();
    for (module, source) in [
        ("values", include_str!("../../idl/tests/fixtures/identifier_hygiene.ncidl")),
        (
            "shadow",
            "#[version(1)] struct Option { enabled: bool } #[version(1)] struct Envelope { value: Option }",
        ),
    ] {
        let graph = neocogi_idl::resolve_modules(vec![neocogi_idl::parse_module(Path::new(&format!("proof/{module}.ncidl")), source).unwrap()]).unwrap();
        let generated = neocogi_idl::generate_rust(&graph, &format!("proof.{module}")).unwrap();
        let mut expanded = proc_macro2::TokenStream::new();
        for item in syn::parse_file(&generated).unwrap().items {
            // Strip only the macro being expanded; retain documentation, derives,
            // public names, and metadata exactly as an ordinary consumer does.
            let tokens = match item {
                syn::Item::Struct(mut item) => {
                    let index = item
                        .attrs
                        .iter()
                        .position(|attr| matches!(attr.path().segments.last().unwrap().ident.to_string().as_str(), "com_struct" | "com_value"))
                        .unwrap();
                    let attr = item.attrs.remove(index);
                    match attr.path().segments.last().unwrap().ident.to_string().as_str() {
                        "com_struct" => crate::structs::expand_com_struct(attr.parse_args().unwrap(), item).unwrap(),
                        "com_value" => crate::value::expand_owned(attr.parse_args().unwrap(), syn::parse2(quote! { #item }).unwrap()).unwrap(),
                        _ => unreachable!(),
                    }
                }
                syn::Item::Enum(mut item) => {
                    let index = item
                        .attrs
                        .iter()
                        .position(|attr| matches!(attr.path().segments.last().unwrap().ident.to_string().as_str(), "com_enum" | "com_value"))
                        .unwrap();
                    let attr = item.attrs.remove(index);
                    match attr.path().segments.last().unwrap().ident.to_string().as_str() {
                        "com_enum" => crate::enums::expand_com_enum(attr.parse_args().unwrap(), item).unwrap(),
                        "com_value" => crate::value::expand_owned(attr.parse_args().unwrap(), syn::parse2(quote! { #item }).unwrap()).unwrap(),
                        _ => unreachable!(),
                    }
                }
                syn::Item::Trait(mut item) => {
                    let index = item
                        .attrs
                        .iter()
                        .position(|attr| attr.path().segments.last().unwrap().ident == "com_interface")
                        .unwrap();
                    let args = item.attrs.remove(index).parse_args().unwrap();
                    crate::interface::expand_com_interface(args, item).unwrap()
                }
                item => quote! { #item },
            };
            expanded.extend(tokens);
        }
        fs::write(
            project.path().join(format!("src/{module}.rs")),
            prettyplease::unparse(&syn::parse2(expanded).unwrap()),
        )
        .unwrap();
    }
    // Share the semantic assertions with the ordinary procedural-macro path.
    fs::write(
        project.path().join("src/lib.rs"),
        include_str!("../../idl/tests/fixtures/identifier_hygiene.rs"),
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
