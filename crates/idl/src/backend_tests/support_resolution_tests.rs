//! Executable normal/printed consumers cover support lookup in plugin and decoder emission.
use std::{fs, process::Command};
use proc_macro2::TokenStream;
use quote::quote;

/// Every record entrypoint delegates to the same default-decoding policy.
#[derive(Clone, Copy, Debug)]
enum Decoder {
    /// Rust-local persistent data derives the decoder without a COM value layout.
    Derive,
    /// Copied ABI records emit a decoder alongside their checked value codec.
    Copied,
    /// Owned ABI records use that same decoder with owned value storage.
    Owned,
}

/// Renders the actual expander output as source, deliberately losing syntax contexts.
fn printed(tokens: TokenStream) -> String {
    prettyplease::unparse(&syn::parse2(tokens).unwrap())
}

/// Emits the target record normally or directly through its production expander.
fn decoder_source(style: Decoder, unrelated_return: bool, expand: bool) -> String {
    let record: syn::ItemStruct = syn::parse_quote! {
        struct Settings { value: u32, enabled: bool }
    };
    let declaration = match (style, expand) {
        (Decoder::Derive, false) => quote! { #[derive(neocogi_com::ComDeserialize)] #[com_deserialize(default)] #record }.to_string(),
        (Decoder::Derive, true) => {
            let input = syn::parse2(quote! { #[com_deserialize(default)] #record }).unwrap();
            let implementation = crate::codegen::expand_com_deserialize_derive(input).unwrap();
            printed(quote! { #record #implementation })
        }
        (Decoder::Copied | Decoder::Owned, _) => {
            let retained = if matches!(style, Decoder::Owned) { ", retained: Vec<u8>" } else { "" };
            super::name_ownership_tests::contract(&format!(
                "#[version(1)] #[persistence(default)] struct Settings {{ value: u32, enabled: bool{retained} }}"
            ))
        }
    };
    let retained = if matches!(style, Decoder::Owned) { ", retained: vec![]" } else { "" };
    let inherent = if unrelated_return {
        "fn default() -> u32 { 123 }"
    } else {
        &format!("fn default() -> Self {{ Self {{ value: 123, enabled: false{retained} }} }}")
    };
    format!(
        r#"
        #![allow(dead_code)]
        {declaration}
        impl ::core::default::Default for Settings {{
            fn default() -> Self {{ Self {{ value: 7, enabled: true{retained} }} }}
        }}
        impl Settings {{ {inherent} }}
        #[derive(neocogi_com::ComSerialize)]
        struct Prefix {{ value: u32 }}
        #[derive(neocogi_com::ComSerialize)]
        struct Complete {{ value: u32, enabled: bool }}
        #[test]
        fn trait_defaults_apply_only_to_absent_fields() {{
            let empty = neocogi_com::binary::from_slice::<Settings>(b"NCB\x01\0\0\0\0").unwrap();
            assert_eq!((empty.value, empty.enabled), (7, true));
            let prefix = neocogi_com::binary::to_vec(&Prefix {{ value: 73 }}).unwrap();
            let partial = neocogi_com::binary::from_slice::<Settings>(&prefix).unwrap();
            assert_eq!((partial.value, partial.enabled), (73, true));
            let mut complete = neocogi_com::binary::to_vec(&Complete {{ value: 91, enabled: false }}).unwrap();
            let full = neocogi_com::binary::from_slice::<Settings>(&complete).unwrap();
            assert_eq!((full.value, full.enabled), (91, false));
            *complete.last_mut().unwrap() = 2; // A present malformed bool cannot become its default.
            assert!(neocogi_com::binary::from_slice::<Settings>(&complete).is_err());
            assert!(neocogi_com::binary::from_slice::<Settings>(&prefix[..prefix.len()-1]).is_err());
        }}
    "#
    )
}

/// Emits one independent plugin case so one namespace failure cannot mask another.
fn plugin_source(shadow: &str, explicit_version: bool, expand: bool) -> String {
    let version = explicit_version.then(|| quote! { , version = "4.5.6" });
    let args = quote! { name = "nc.tests.support_resolution" #version };
    let function: syn::ItemFn = syn::parse_quote! {
        fn acquire(_runtime: neocogi_com::Runtime) -> Result<neocogi_com::ComPtr<IRootAbi>, neocogi_com::NcStatus> {
            Root::from_value(Root).query()
        }
    };
    let plugin = if expand {
        printed(crate::codegen::expand_plugin(syn::parse2(args).unwrap(), function).unwrap())
    } else {
        quote! { #[neocogi_com::plugin(#args)] #function }.to_string()
    };
    let bindings = super::name_ownership_tests::contract("#[version(1)] interface IRoot { fn value(&self) -> Result<u32>; }");
    let expected = if explicit_version { "4.5.6" } else { "0.0.0" };
    format!(
        r#"
        #![allow(dead_code, non_snake_case, unused_macros)]
        {bindings}
        #[neocogi_com::com_object(interfaces(IRoot))]
        struct Root;
        impl IRoot for Root {{ fn value(&self) -> Result<u32, neocogi_com::NcStatus> {{ Ok(73) }} }}
        {shadow}
        {plugin}
        #[test]
        fn descriptor_and_acquisition_use_generator_support() {{
            let descriptor = plugin_descriptor();
            // SAFETY: the generated descriptor's text is static in this executable.
            assert_eq!(unsafe {{ descriptor.plugin_version.as_str() }}.unwrap(), {expected:?});
            let mut raw = neocogi_com::ComRaw::<neocogi_com::IUnknown>::null();
            // SAFETY: live output storage receives one owner from the generated C adapter.
            assert!(unsafe {{ descriptor.acquire.unwrap()(nc_com::initialize_process(), &mut raw) }}.is_ok());
            // SAFETY: successful acquisition returned that one shared owner.
            let root = unsafe {{ neocogi_com::ComPtr::<neocogi_com::IUnknown>::from_raw_owned(raw) }}.unwrap();
            assert_eq!(root.query::<IRootAbi>().unwrap().value().unwrap(), 73);
        }}
    "#
    )
}

/// Executes each real consumer and accumulates independent failures for useful baseline evidence.
fn run_cases(consumer: super::test_consumer::Consumer, cases: impl IntoIterator<Item = (String, String)>) {
    let mut failures = Vec::new();
    for (name, source) in cases {
        fs::write(consumer.directory.path().join("src/lib.rs"), source).unwrap();
        let output = Command::new("cargo")
            .args(["test", "--offline", "--manifest-path"])
            .arg(consumer.directory.path().join("Cargo.toml"))
            .output()
            .unwrap();
        if !output.status.success() {
            failures.push(format!(
                "{name}:\n{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        } else {
            assert!(
                String::from_utf8_lossy(&output.stdout).contains("1 passed"),
                "{name}: no semantic test executed"
            );
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Both a wrong-return-type inherent method and a plausible wrong value must be harmless.
#[test]
fn every_decoder_entrypoint_selects_trait_defaults_in_normal_and_printed_rust() {
    run_cases(
        super::test_consumer::Consumer::new("support-resolution-proof"),
        [Decoder::Derive, Decoder::Copied, Decoder::Owned].into_iter().flat_map(|style| {
            [false, true].into_iter().flat_map(move |unrelated| {
                // IDL already emits complete printed Rust; only standalone derives
                // have distinct ordinary procedural and explicit printed paths.
                let expansions = if matches!(style, Decoder::Derive) { vec![false, true] } else { vec![true] };
                expansions.into_iter().map(move |expand| {
                    (
                        format!("{style:?}, unrelated return={unrelated}, printed={expand}"),
                        decoder_source(style, unrelated, expand),
                    )
                })
            })
        }),
    );
}

/// Value and macro namespaces have separate adversarial controls, including silent wrong metadata.
#[test]
fn plugin_support_lookup_is_explicit_in_normal_and_printed_rust() {
    let cases = [
        ("control", "", false),
        ("Some wrong arity", "fn Some() {}", false),
        (
            "Some wrong value",
            r#"const fn Some(_: unsafe extern "C" fn(*const neocogi_com::NcHostApiV1, *mut neocogi_com::ComRaw<neocogi_com::IUnknown>) -> neocogi_com::NcStatus) -> Option<unsafe extern "C" fn(*const neocogi_com::NcHostApiV1, *mut neocogi_com::ComRaw<neocogi_com::IUnknown>) -> neocogi_com::NcStatus> { None }"#,
            false,
        ),
        ("env wrong type", "macro_rules! env { ($key:literal) => { 99_u32 }; }", false),
        ("env wrong value", "macro_rules! env { ($key:literal) => { \"99.0.0\" }; }", false),
        ("explicit version", "macro_rules! env { ($key:literal) => { 99_u32 }; }", true),
    ];
    run_cases(
        super::test_consumer::Consumer::new("plugin-support-resolution-proof").with_runtime_owner(),
        cases
            .into_iter()
            .flat_map(|(name, shadow, version)| [false, true].map(move |expand| (format!("{name}, printed={expand}"), plugin_source(shadow, version, expand)))),
    );
}
