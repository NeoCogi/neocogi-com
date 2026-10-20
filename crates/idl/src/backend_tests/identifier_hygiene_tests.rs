//! Printed bindings must preserve values after macro syntax contexts are lost.
use crate as neocogi_idl;
use std::{fs, path::Path, process::Command};
use quote::quote;

/// Expands the same value/interface macros used by an IDL consumer, then prints
/// them as standalone Rust. The resulting crate cannot benefit from mixed-site
/// contexts, so its runtime checks prove structural name separation as well.
#[test]
fn printed_bindings_preserve_authored_values_and_arguments() {
    let consumer = super::test_consumer::Consumer::new("printed-hygiene-proof");
    let project = &consumer.directory;
    for (module, source) in [
        ("values", include_str!("fixtures/identifier_hygiene.ncidl")),
        ("shadow", include_str!("fixtures/identifier_shadow.ncidl")),
        ("arguments", include_str!("fixtures/identifier_arguments.ncidl")),
        ("support", include_str!("fixtures/identifier_support.ncidl")),
    ] {
        let graph = neocogi_idl::resolve_modules(vec![neocogi_idl::parse_module(Path::new(&format!("proof/{module}.ncidl")), source).unwrap()]).unwrap();
        let generated = neocogi_idl::generate_rust(&graph, &format!("proof.{module}")).unwrap();
        // The backend already emitted the complete typed ABI. Printing and
        // reparsing cannot rely on proc-macro hygiene or a second contract parser.
        fs::write(
            project.path().join(format!("src/{module}.rs")),
            prettyplease::unparse(&syn::parse_file(&generated).unwrap()),
        )
        .unwrap();
    }
    // Share the semantic assertions with the ordinary procedural-macro path.
    fs::write(project.path().join("src/lib.rs"), include_str!("fixtures/identifier_hygiene.rs")).unwrap();
    let result = Command::new("cargo")
        .args(["test", "--offline", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        // Share the caller's build cache when available; all child work completes
        // before the temporary source project and standalone target are removed.
        .env(
            "CARGO_TARGET_DIR",
            std::env::var_os("CARGO_TARGET_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| project.path().join("target")),
        )
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

/// Object expansion must survive printing with authored support names in both ownership modes.
#[test]
fn printed_object_bindings_keep_storage_and_support_names_private() {
    let consumer = super::test_consumer::Consumer::new("printed-object-storage-proof");
    let mut generated = String::from(
        r#"
        use neocogi_com::NcStatus;
        mod core {}
        type Result = ();
        type Send = ();
        type Sync = ();
        struct SharedComObject;
        struct ExclusiveComObject;
    "#,
    );
    generated += &super::name_ownership_tests::contract("#[version(1)] interface IValue { fn value(&self) -> Result<u32>; }");
    for (name, options) in [
        ("Shared", quote! { interfaces(IValue), thread_safe }),
        ("Exclusive", quote! { IValue, ownership = exclusive }),
    ] {
        let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
        let object = syn::parse2(quote! { struct #ident<Result> { value: u32, _state: Result } }).unwrap();
        let expanded = neocogi_com_codegen::expand_com_object(syn::parse2(options).unwrap(), object);
        generated += &prettyplease::unparse(&syn::parse2(expanded).unwrap());
        generated += &format!(
            r#"
            impl<Result> IValue for {name}<Result> {{
                fn value(&self) -> ::core::result::Result<u32, NcStatus> {{
                    ::core::result::Result::Ok(self.value)
                }}
            }}
        "#
        );
    }
    generated += r#"
        #[test]
        fn printed_providers_execute_real_calls() {
            let shared = Shared::from_value(Shared { value: 73, _state: () });
            let shared = shared.query::<IValueAbi>().unwrap();
            assert_eq!(shared.value(), ::core::result::Result::Ok(73));
            let exclusive = Exclusive::into_box(Exclusive { value: 91, _state: () });
            let projection = exclusive.query_weak::<IValueAbi>().unwrap();
            assert_eq!(projection.value(), ::core::result::Result::Ok(91));
            drop(exclusive);
            assert_eq!(projection.value(), ::core::result::Result::Err(neocogi_com::NC_E_DROPPED));
        }
    "#;
    fs::write(consumer.directory.path().join("src/lib.rs"), generated).unwrap();
    let output = Command::new("cargo")
        .args(["test", "--offline", "--manifest-path"])
        .arg(consumer.directory.path().join("Cargo.toml"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
}
