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
    fs::create_dir_all(root.join("provider-schemas/proof/service")).unwrap();
    fs::create_dir_all(root.join("schemas/proof")).unwrap();
    fs::create_dir_all(root.join("provider/src")).unwrap();
    fs::write(root.join("schemas/proof/value.ncidl"), "#[version(1)] struct Value { count: u32 }").unwrap();
    fs::write(
        root.join("provider-schemas/proof/service/main.ncidl"),
        "extern proof.value; use proof.value::Value; #[version(1)] interface Service { fn increment(&self, value: Value) -> Result<Value>; }",
    )
    .unwrap();
    let idl = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = root.join("Cargo.toml");
    let mut source = fs::read_to_string(&manifest).unwrap();
    source.push_str(&format!("provider={{path='provider'}}\n[build-dependencies]\nneocogi-idl={{path={idl:?}}}\n"));
    fs::write(manifest, source).unwrap();
    fs::write(root.join("provider/Cargo.toml"), format!("[package]\nname='provider'\nversion='0.0.0'\nedition='2024'\n[dependencies]\nneocogi-com={{path={:?}}}\n[build-dependencies]\nneocogi-idl={{path={idl:?}}}\n", consumer.facade)).unwrap();
    for (path, shared, service, owner) in [
        (root.to_owned(), "schemas", "provider-schemas", "shared_source_host"),
        (root.join("provider"), "../schemas", "../provider-schemas", "provider"),
    ] {
        fs::write(
            path.join("build.rs"),
            format!(
                r#"
            fn main() {{
                println!("cargo::rerun-if-changed=build.rs");
                let stamp = std::path::Path::new("build-runs");
                let runs = std::fs::read_to_string(stamp).map(|value| value.parse::<u32>().unwrap()).unwrap_or(0);
                std::fs::write(stamp, (runs + 1).to_string()).unwrap();
                neocogi_idl::cargo::SharedSources::new({shared:?})
                    .include_root({service:?})
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
    let runs = || [root.join("build-runs"), root.join("provider/build-runs")].map(|path| fs::read_to_string(path).unwrap().parse::<u32>().unwrap());
    let first = runs();
    let run = || {
        Command::new("cargo")
            .args(["test", "--offline", "--manifest-path"])
            .arg(root.join("Cargo.toml"))
            .output()
            .unwrap()
    };
    let cached = run();
    assert!(cached.status.success(), "{}", String::from_utf8_lossy(&cached.stderr));
    assert_eq!(runs(), first, "unchanged source roots must not rerun generation");
    // New source membership must invalidate both consumers even when it has no
    // ABI declarations. This also preserves both consumers' exact identities.
    fs::write(root.join("provider-schemas/proof/service/additional.ncidl"), "// Added source part.\n").unwrap();
    let added = run();
    assert!(added.status.success(), "{}", String::from_utf8_lossy(&added.stderr));
    assert_eq!(runs(), first.map(|value| value + 1));
    // A new competing owner must invalidate the previously successful cached
    // build. Root order must never silently choose the old directory owner.
    fs::write(root.join("schemas/proof/service.ncidl"), "#[version(1)] interface Service {}").unwrap();
    let conflict = run();
    assert!(!conflict.status.success());
    assert!(String::from_utf8_lossy(&conflict.stderr).contains("ambiguous source owners"));
}
