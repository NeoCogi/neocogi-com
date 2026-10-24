//! Two real consumers independently generate one shared IDL source and call across COM.

use std::{fs, process::Command};
use super::test_consumer::Consumer;

/// Distinct generated Rust types retain identical ABI identities without any
/// Cargo links owner or separately built contract library. Multipart imports and
/// actual copied arguments/outputs exercise the same transitive source graph.
#[test]
fn independent_shared_source_consumers_call_through_their_own_bindings() {
    let consumer = Consumer::new("shared-source-host");
    let root = consumer.directory.path();
    fs::create_dir_all(root.join("idl/proof/service")).unwrap();
    fs::create_dir_all(root.join("provider/src")).unwrap();
    fs::write(root.join("idl/proof/value.ncidl"), "#[version(1)] struct Value { count: u32 }").unwrap();
    fs::write(
        root.join("idl/proof/service/main.ncidl"),
        "extern proof.value; use proof.value::Value; #[version(1)] interface Service { fn increment(&self, value: Value) -> Result<Value>; }",
    )
    .unwrap();
    let idl = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = root.join("Cargo.toml");
    let mut source = fs::read_to_string(&manifest).unwrap();
    source.push_str(&format!("provider={{path='provider'}}\n[build-dependencies]\nneocogi-idl={{path={idl:?}}}\n"));
    fs::write(manifest, source).unwrap();
    fs::write(root.join("provider/Cargo.toml"), format!("[package]\nname='provider'\nversion='0.0.0'\nedition='2024'\n[dependencies]\nneocogi-com={{path={:?}}}\n[build-dependencies]\nneocogi-idl={{path={idl:?}}}\n", consumer.facade)).unwrap();
    for (path, shared, owner) in [(root.to_owned(), "idl", "shared_source_host"), (root.join("provider"), "../idl", "provider")] {
        fs::write(
            path.join("build.rs"),
            format!(
                r#"
            fn main() {{
                neocogi_idl::cargo::SharedSources::new({shared:?})
                    .module("proof.value", "{owner}::value")
                    .module("proof.service", "{owner}::service")
                    .build().unwrap().into_iter().for_each(|module| {{
                        let key = if module.module_name == "proof.value" {{ "VALUE_BINDINGS" }} else {{ "SERVICE_BINDINGS" }};
                        println!("cargo::rustc-env={{key}}={{}}", module.include.display());
                    }});
            }}
        "#
            ),
        )
        .unwrap();
    }
    let modules = r#"
        pub mod value { include!(env!("VALUE_BINDINGS")); }
        pub mod service { include!(env!("SERVICE_BINDINGS")); }
    "#;
    fs::write(
        root.join("provider/src/lib.rs"),
        format!(
            r#"
        extern crate self as provider;
        {modules}
        #[neocogi_com::com_object(service::Service)] struct Counter;
        impl service::Service for Counter {{
            fn increment(&self, value: value::Value) -> Result<value::Value, neocogi_com::NcStatus> {{
                Ok(value::Value {{ count: value.count + 1 }})
            }}
        }}
        pub fn root() -> neocogi_com::ComPtr<neocogi_com::IUnknown> {{ Counter::from_value(Counter) }}
    "#
        ),
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        format!(
            r#"
        extern crate self as shared_source_host;
        {modules}
        trait Independent {{}}
        impl Independent for value::Value {{}}
        impl Independent for provider::value::Value {{}}
        #[test] fn real_call() {{
            assert_eq!(service::MODULE_ID, provider::service::MODULE_ID);
            assert_eq!(service::ABI_FINGERPRINT, provider::service::ABI_FINGERPRINT);
            let local = provider::root().query::<service::ServiceAbi>().unwrap();
            assert_eq!(local.increment(value::Value {{ count: 41 }}).unwrap().count, 42);
        }}
    "#
        ),
    )
    .unwrap();
    let output = Command::new("cargo")
        .args(["test", "--offline", "--manifest-path"])
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
