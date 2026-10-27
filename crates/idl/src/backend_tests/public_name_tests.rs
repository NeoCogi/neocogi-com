//! Public namespace rules are checked at authored inputs and through real COM clients.
use std::{fs, path::Path, process::Command};

/// Resolve a complete authored module without creating a Rust consumer first.
fn resolve(source: &str) -> Result<crate::ResolvedModules, crate::ModuleGraphError> {
    crate::resolve_modules(vec![crate::parse_module(Path::new("proof/names.ncidl"), source).unwrap()])
}

/// Prefix names depend on physical shape, including templates never instantiated.
#[test]
fn physical_prefix_conflicts_are_rejected_at_authored_methods() {
    for generic in ["", "<T>"] {
        for (base, name) in [("", "unknown"), (": Parent", "base")] {
            let source =
                format!("#[version(1)] interface Parent {{}}\n#[version(1)] interface Invalid{generic}{base} {{\n fn {name}(&self) -> Result<u32>;\n}}");
            let error = resolve(&source).unwrap_err();
            let crate::ModuleGraphError::Resolution(error) = error else {
                panic!("expected semantic diagnostics")
            };
            let diagnostic = error.diagnostics.iter().find(|d| d.code == "EIDL100").unwrap();
            let text = error.to_string();
            assert!(diagnostic.message.contains("vtable prefix"), "{text}");
            assert!(text.contains("proof/names.ncidl:3:"), "{text}");
            assert!(text.contains(&format!("fn {name}")), "{text}");
        }
    }
    resolve("#[version(1)] interface Parent {} #[version(1)] interface Root<T> { fn base(&self, unknown: T) -> Result<T>; } #[version(1)] interface Child<T>: Parent { fn unknown(&self, base: T) -> Result<T>; } #[version(1)] interface Closed: Child<u32> {}").unwrap();
}

/// Prelude methods are rejected even in unused templates; parameter names remain legal.
#[test]
fn supported_receiver_surface_includes_standard_prelude_blankets() {
    for name in ["into", "try_into", "to_owned", "clone_into"] {
        for generic in ["", "<T>"] {
            let error = resolve(&format!("#[version(1)] interface Invalid{generic} {{\n fn {name}(&self) -> Result<u32>;\n}}")).unwrap_err();
            let text = error.to_string();
            assert!(text.contains("EIDL100") && text.contains("proof/names.ncidl:2:"), "{text}");
        }
        resolve(&format!("#[version(1)] interface Valid {{ fn value(&self, {name}: u32) -> Result<u32>; }}")).unwrap();
    }
}

/// Common metadata must be admitted independently of the optional dense helpers.
#[test]
fn flag_metadata_conflict_is_source_local_and_other_public_names_stay_legal() {
    {
        let error = resolve("#[version(1)] bitenum Flags: u8 {\n IDL_TYPE = 1\n}").unwrap_err();
        let text = error.to_string();
        assert!(
            text.contains("EIDL021") && text.contains("proof/names.ncidl:2:") && text.contains("per-value metadata"),
            "{text}"
        );
    }
    let graph = resolve("#[version(1)] struct Data { idl_type: u32 } #[version(1)] enum Plain: u8 { COUNT = 0, ALL = 1, NAMES = 2 } #[version(1)] bitenum Flags: u8 { KNOWN_BITS = 1, USER_BITS = 2 }").unwrap();
    let rust = crate::generate_rust(&graph, "proof.names").unwrap();
    let file = syn::parse_file(&rust).unwrap();
    for name in ["Data", "Plain", "Flags"] {
        let count = file
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Impl(item) if item.trait_.is_none() => Some(item),
                _ => None,
            })
            .filter(|item| matches!(&*item.self_ty, syn::Type::Path(ty) if ty.path.is_ident(name)))
            .flat_map(|item| &item.items)
            .filter(|item| matches!(item, syn::ImplItem::Const(item) if item.ident == "IDL_TYPE"))
            .count();
        assert_eq!(count, 1, "{name}: common metadata exists exactly once");
    }
}

/// One real SDK consumer exercises legal prefixes, metadata, provider statuses and
/// standard blanket operations on the outer handle. All four ownership views call
/// accepted application methods; parameter spellings cannot affect dispatch.
#[test]
fn accepted_names_execute_through_shared_borrowed_exclusive_and_weak_clients() {
    let bindings = super::name_ownership_tests::contract(
        r#"
        #[version(1)] interface Parent {}
        #[version(1)] interface Root { fn base(&self, into: u32, try_into: u32, to_owned: u32, clone_into: u32) -> Result<u32>; }
        #[version(1)] interface Child: Parent { fn unknown(&self) -> Result<u32>; }
        #[version(1)] bitenum Flags: u8 { KNOWN_BITS = 1, USER_BITS = 2 }
    "#,
    );
    let source = r#"
        #![allow(dead_code)]
        use neocogi_com::{ComPtr, NcStatus, NC_E_CLOSED};
        use std::sync::atomic::{AtomicUsize, Ordering};
        $BINDINGS
        static ENTRIES: AtomicUsize = AtomicUsize::new(0);
        #[neocogi_com::com_object(interfaces(Root, Child))] struct Shared;
        #[neocogi_com::com_object(interfaces(Root, Child), ownership = exclusive)] struct Exclusive;
        macro_rules! implement {
            ($ty:ty) => {
                impl Parent for $ty {}
                impl Root for $ty {
                    fn base(&self, into: u32, try_into: u32, to_owned: u32, clone_into: u32) -> Result<u32, NcStatus> {
                        ENTRIES.fetch_add(1, Ordering::SeqCst);
                        Ok(into + try_into + to_owned + clone_into)
                    }
                }
                impl Child for $ty {
                    fn unknown(&self) -> Result<u32, NcStatus> { ENTRIES.fetch_add(1, Ordering::SeqCst); Err(NC_E_CLOSED) }
                }
            }
        }
        implement!(Shared); implement!(Exclusive);
        #[test] fn calls_and_metadata_keep_their_owners() {
            let owner = Shared::from_value(Shared);
            let root = owner.query::<RootAbi>().unwrap();
            assert_eq!(root.base(1, 2, 3, 67), Ok(73));
            assert_eq!(root.as_com_ref().base(1, 2, 3, 67), Ok(73));
            assert_eq!(owner.query::<ChildAbi>().unwrap().unknown(), Err(NC_E_CLOSED));
            let _: ComPtr<RootAbi> = root.clone().into();
            let _: Result<ComPtr<RootAbi>, std::convert::Infallible> = root.clone().try_into();
            let mut copy = root.to_owned(); root.clone_into(&mut copy);
            assert_eq!(ENTRIES.load(Ordering::SeqCst), 3, "blanket operations are handle operations");
            let exclusive = Exclusive::into_box(Exclusive);
            let weak = exclusive.query_weak::<RootAbi>().unwrap();
            assert_eq!(weak.base(1, 2, 3, 67), Ok(73));
            let boxed = exclusive.query::<RootAbi>().unwrap();
            assert_eq!(boxed.base(1, 2, 3, 67), Ok(73));
            assert_eq!(boxed.query_ref::<ChildAbi>().unwrap().unknown(), Err(NC_E_CLOSED));
            assert_eq!(ENTRIES.load(Ordering::SeqCst), 6);
            assert_eq!((Flags::KNOWN_BITS | Flags::USER_BITS).raw(), 3);
            let _ = Flags::IDL_TYPE;
            let _ = RootAbi::IDL_INTERFACE;
        }
    "#
    .replace("$BINDINGS", &bindings);
    super::name_ownership_tests::run_consumers(
        "public-name-controls",
        [
            ("ordinary".into(), source.clone()),
            ("printed".into(), prettyplease::unparse(&syn::parse_file(&source).unwrap())),
        ],
    );
}

/// Normalized input names cannot occupy the public accessor's module namespace.
#[test]
fn plugin_accessor_checks_rust_identity_at_input() {
    for name in ["plugin_descriptor", "r#plugin_descriptor"] {
        let function = syn::parse_str(&format!("fn {name}() -> Result<ComPtr<RootAbi>, NcStatus> {{ todo!() }}")).unwrap();
        let error = crate::codegen::expand_plugin(syn::parse_str("name = \"proof.names\"").unwrap(), function).unwrap_err();
        assert!(error.to_string().contains("generated plugin_descriptor accessor"));
    }
}

/// The scoped dynamic C export and the ordinary Rust acquisition may share spelling.
#[test]
fn plugin_export_spelling_is_legal_with_dynamic_feature_off_and_on() {
    let consumer = super::test_consumer::Consumer::new("plugin-public-name-controls").with_host_owner();
    let manifest = consumer.directory.path().join("Cargo.toml");
    let mut cargo = fs::read_to_string(&manifest).unwrap();
    cargo += "\n[features]\ndynamic-plugin=[]\n";
    fs::write(&manifest, cargo).unwrap();
    let bindings = super::name_ownership_tests::contract("#[version(1)] interface Root { fn value(&self) -> Result<u32>; }");
    for printed in [false, true] {
        let function: syn::ItemFn = syn::parse_quote! {
            fn nc_get_plugin_descriptor_v1(_runtime: neocogi_com::Runtime) -> Result<neocogi_com::ComPtr<RootAbi>, neocogi_com::NcStatus> {
                Provider::from_value(Provider).query()
            }
        };
        let args = quote::quote! { name = "proof.public_names" };
        let plugin = if printed {
            crate::codegen::expand_plugin(syn::parse2(args).unwrap(), function).unwrap()
        } else {
            quote::quote! { #[neocogi_com::plugin(#args)] #function }
        };
        let source = format!(
            r#"
            #![allow(dead_code)]
            {bindings}
            #[neocogi_com::com_object(interfaces(Root))] struct Provider;
            impl Root for Provider {{ fn value(&self) -> Result<u32, neocogi_com::NcStatus> {{ Ok(73) }} }}
            {plugin}
            #[test] fn acquire_through_actual_descriptor() {{
                let descriptor = plugin_descriptor();
                let mut raw = neocogi_com::ComRaw::null();
                // SAFETY: live writable slot receives one owner from the actual adapter.
                assert!(unsafe {{ descriptor.acquire.unwrap()(neocogi_com_host::initialize_process(), &mut raw) }}.is_ok());
                // SAFETY: successful adapter transferred the initialized owner.
                let owner = unsafe {{ neocogi_com::ComPtr::<neocogi_com::IUnknown>::from_raw_owned(raw) }}.unwrap();
                assert_eq!(owner.query::<RootAbi>().unwrap().value(), Ok(73));
            }}
        "#
        );
        fs::write(consumer.directory.path().join("src/lib.rs"), source).unwrap();
        for features in [vec![], vec!["--features", "dynamic-plugin"]] {
            let output = Command::new("cargo")
                .args(["test", "--offline", "--manifest-path"])
                .arg(&manifest)
                .args(features)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "printed={printed}: {}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
        }
    }
}

/// Imported templates retain their authored diagnostic owner before closing.
#[test]
fn imported_template_conflicts_keep_the_definition_span() {
    for name in ["unknown", "into", "try_into", "to_owned", "clone_into"] {
        let definition = crate::parse_module(
            Path::new("proof/definitions.ncidl"),
            &format!("#[version(1)] interface Template<T> {{\n fn {name}(&self) -> Result<T>;\n}}"),
        )
        .unwrap();
        let consumer = crate::parse_module(
            Path::new("proof/consumer.ncidl"),
            "extern proof.definitions; use proof.definitions::Template; #[version(1)] interface Closed: Template<u32> {}",
        )
        .unwrap();
        let error = crate::resolve_modules(vec![consumer, definition]).unwrap_err().to_string();
        assert!(error.contains("proof/definitions.ncidl:2:") && error.contains("EIDL100"), "{error}");
    }
}

/// no_std clients use the same method policy and keep ordinary conversion controls.
#[test]
fn accepted_client_and_prelude_controls_compile_without_std_or_alloc() {
    let consumer = super::test_consumer::Consumer::new("public-name-no-std-controls");
    let manifest = consumer.directory.path().join("Cargo.toml");
    fs::write(&manifest, format!("[package]\nname='public-name-no-std-controls'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?},default-features=false}}\n", consumer.facade)).unwrap();
    let bindings = super::name_ownership_tests::contract(
        "#[version(1)] interface Root { fn base(&self, into: u32, try_into: u32, to_owned: u32, clone_into: u32) -> Result<u32>; }",
    );
    fs::write(
        consumer.directory.path().join("src/lib.rs"),
        format!(
            r#"
        #![no_std]
        #![allow(dead_code)]
        {bindings}
        pub fn call(value: neocogi_com::ComRef<'_, RootAbi>) -> Result<u32, neocogi_com::NcStatus> {{
            value.base(1, 2, 3, 67)
        }}
        pub fn convert(value: neocogi_com::ComPtr<RootAbi>) -> neocogi_com::ComPtr<RootAbi> {{ value.into() }}
        pub fn try_convert(value: neocogi_com::ComPtr<RootAbi>) -> Result<neocogi_com::ComPtr<RootAbi>, core::convert::Infallible> {{ value.try_into() }}
    "#
        ),
    )
    .unwrap();
    let output = Command::new("cargo")
        .args(["check", "--offline", "--manifest-path"])
        .arg(&manifest)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
