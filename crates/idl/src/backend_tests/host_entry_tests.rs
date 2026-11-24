//! Actual executable startup through normal and printed macro expansions.

use std::{fs, process::Command};
use quote::quote;

use super::test_consumer::Consumer;

/// Renamed Cargo dependencies and ambient binding names must survive both ways
/// of consuming generated Rust, including ordinary Result/? entry semantics.
#[test]
fn host_entry_runs_with_normal_printed_and_renamed_dependencies() {
    for (renamed, forwarded) in [(false, false), (true, false), (false, true), (true, true)] {
        let consumer = Consumer::new("host-entry-proof").with_host_owner();
        let manifest = consumer.directory.path().join("Cargo.toml");
        let (facade, implementation, args) = if renamed {
            let source = fs::read_to_string(&manifest)
                .unwrap()
                .lines()
                .map(|line| {
                    for (package, alias) in [("neocogi-com-sdk", "sdk"), ("neocogi-com-host", "owner")] {
                        if let Some(dependency) = line.strip_prefix(&format!("{package}=")) {
                            return match dependency.strip_prefix('{') {
                                Some(fields) => format!("{alias}={{package={package:?},{fields}\n"),
                                None => format!("{alias}={{package={package:?},version={dependency}}}\n"),
                            };
                        }
                    }
                    format!("{line}\n")
                })
                .collect::<String>();
            fs::write(&manifest, source).unwrap();
            ("sdk", "owner", "neocogi_com = ::sdk, neocogi_com_host = ::owner")
        } else {
            ("neocogi_com_sdk", "neocogi_com_host", "")
        };
        // A custom safe module can forward admitted access, including through
        // renamed dependencies. Its path is not privileged by the expansion.
        let args = if forwarded {
            format!("neocogi_com = ::{facade}, neocogi_com_host = selected_owner")
        } else {
            args.to_owned()
        };
        let authored: syn::ItemFn = syn::parse_str(&format!(
            r#"
            /// Ordinary main remains a function with normal Result/? behavior.
            #[deny(unused_variables)]
            fn main(runtime: {facade}::RuntimeHandle) -> Result<(), &'static str> {{
                let copied = runtime;
                let admitted = Result::<_, &'static str>::Ok(copied.abi_version())?;
                assert_eq!(admitted, {facade}::NC_COM_ABI_VERSION);
                assert_eq!(CALLS.fetch_add(1, ::core::sync::atomic::Ordering::SeqCst), 0);
                println!("host-entry-completed");
                Ok(())
            }}
        "#
        ))
        .unwrap();
        let expanded = crate::codegen::expand_com_main(syn::parse_str(&args).unwrap(), authored.clone()).unwrap();
        let generated: syn::File = syn::parse2(expanded.clone()).unwrap();
        // Use the same adversarial scope construction as the other generator
        // tests, derived from the actual expansion rather than guessed suffixes.
        let ambient = super::name_ownership_tests::ambient_binding_constants(&quote!(#authored).to_string(), &generated);
        for printed in [false, true] {
            let entry = if printed {
                prettyplease::unparse(&generated)
            } else {
                format!("#[{facade}::com_main({args})] {authored}", authored = quote!(#authored))
            };
            let source = format!(
                r#"
                #![deny(unsafe_code)]
                #![allow(dead_code, non_upper_case_globals)]
                {ambient}
                static CALLS: ::core::sync::atomic::AtomicUsize = ::core::sync::atomic::AtomicUsize::new(0);
                mod selected_owner {{
                    pub fn initialize_runtime() -> {facade}::RuntimeHandle {{
                        {implementation}::initialize_runtime()
                    }}
                }}
                {entry}
            "#
            );
            fs::write(consumer.directory.path().join("src/main.rs"), source).unwrap();
            let output = Command::new("cargo")
                .args(["run", "--offline", "--quiet", "--manifest-path"])
                .arg(&manifest)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "renamed={renamed} printed={printed} forwarded={forwarded} owner={implementation}:\n{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "host-entry-completed");
        }
    }
}

/// A static outer record does not prove backing for its child pointers. Compile
/// only: deliberately invalid children must never be admitted or executed by a
/// safe macro, with either the retired raw entry name or the typed entry name.
#[test]
fn host_entry_rejects_safe_modules_returning_unverified_raw_tables() {
    let consumer = Consumer::new("host-entry-raw-rejection");
    for initializer in ["initialize_process", "initialize_runtime"] {
        let source = format!(
            r#"
            #![deny(unsafe_code)]
            mod safe_local_host {{
                use neocogi_com_sdk::{{NcHostApi, NcPluginApi}};
                static HOST: NcHostApi = NcHostApi {{
                    size: core::mem::size_of::<NcHostApi>(),
                    abi_magic: neocogi_com_sdk::NC_COM_ABI_MAGIC,
                    abi_version: neocogi_com_sdk::NC_COM_ABI_VERSION,
                    runtime: core::ptr::NonNull::<NcPluginApi>::dangling().as_ptr(),
                    debug: core::ptr::null(),
                    persistence: core::ptr::null(),
                }};
                pub fn {initializer}() -> &'static NcHostApi {{ &HOST }}
            }}
            #[neocogi_com_sdk::com_main(neocogi_com_host = safe_local_host)]
            fn main(_runtime: neocogi_com_sdk::RuntimeHandle) {{}}
        "#
        );
        fs::write(consumer.directory.path().join("src/main.rs"), source).unwrap();
        let output = Command::new("cargo")
            .args(["check", "--offline", "--manifest-path"])
            .arg(consumer.directory.path().join("Cargo.toml"))
            .output()
            .unwrap();
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{initializer} must not admit a raw table");
        if initializer == "initialize_process" {
            assert!(diagnostic.contains("E0425") && diagnostic.contains("initialize_runtime"), "{diagnostic}");
        } else {
            assert!(
                diagnostic.contains("E0308") && diagnostic.contains("RuntimeHandle") && diagnostic.contains("NcHostApi"),
                "{diagnostic}"
            );
        }
    }
}

/// The ordinary Rust call checks the parameter's actual type, allowing aliases
/// while rejecting a same-arity function that expects unrelated application data.
#[test]
fn plugin_bootstrap_requires_the_actual_supplied_runtime_type() {
    // A pure provider needs no implementation dependency to compile its adapter.
    let consumer = Consumer::new("plugin-runtime-type-proof");
    fs::write(
        consumer.directory.path().join("src/lib.rs"),
        r#"
        #[neocogi_com_sdk::com_object(interfaces())]
        struct Root;
        #[neocogi_com_sdk::plugin(name = "nc.tests.wrong-runtime-argument")]
        fn acquire(_application_value: u32) -> Result<neocogi_com_sdk::ComPtr<neocogi_com_sdk::IUnknown>, neocogi_com_sdk::NcStatus> {
            Ok(Root::from_value(Root))
        }
    "#,
    )
    .unwrap();
    let output = Command::new("cargo")
        .args(["check", "--offline", "--manifest-path"])
        .arg(consumer.directory.path().join("Cargo.toml"))
        .output()
        .unwrap();
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "an unrelated argument type must not compile");
    assert!(
        diagnostic.contains("E0308") && diagnostic.contains("RuntimeHandle") && diagnostic.contains("u32"),
        "{diagnostic}"
    );
}
