//! Imported Rust declarations must prove the exact resolved IDL contract.
//!
//! All unsafe codecs, tables and clients in these fixtures come from the real
//! generator. Safe facade reexports reproduce ordinary stale/misplaced types;
//! compile rejection and independently generated COM calls test the boundary.

use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use super::test_consumer::Consumer;
use crate::{
    ResolvedModules,
    generate::{generate_rust_with_paths, RustModulePaths},
};

/// Physically equal records with exchanged meaning reproduce the original bug.
const OLD_VALUES: &str = "#[version(1)] struct Value { width: u32, height: u32 }";
/// A second same-layout name makes nominal admission independent of layout.
const CURRENT_VALUES: &str = "#[version(1)] struct Value { height: u32, width: u32 } #[version(1)] struct Other { height: u32, width: u32 }";
/// Independent provider and host must agree on the actual copied codec.
const SERVICE: &str = "extern proof.value; use proof.value::Value; #[version(1)] interface Service { fn increment(&self, value: Value) -> Result<Value>; }";

/// Both bases have the same physical slot types but opposite method meanings.
const OLD_BASE: &str = "#[version(1)] interface Base { fn width(&self) -> Result<u32>; fn height(&self) -> Result<u32>; }";
/// The actual checked marker must determine this height/width ordering.
const CURRENT_BASE: &str = "#[version(1)] interface Base { fn height(&self) -> Result<u32>; fn width(&self) -> Result<u32>; }";

/// Owns one real helper/provider/host consumer graph for successive facade cases.
struct Fixture {
    /// Cargo-resolved SDK locations and temporary consumer lifetime.
    consumer: Consumer,
}

impl Fixture {
    /// Writes ordinary package dependencies without inventing SDK source paths.
    fn new(name: &str, old: &str, current: &str) -> Self {
        let consumer = Consumer::new(name);
        let root = consumer.directory.path();
        for package in ["helper", "provider"] {
            fs::create_dir_all(root.join(package).join("src")).unwrap();
            let extra = if package == "provider" { "binding-helper={path='../helper'}\n" } else { "" };
            fs::write(
                root.join(package).join("Cargo.toml"),
                format!(
                    "[package]\nname='binding-{package}'\nversion='0.0.0'\nedition='2024'\n[dependencies]\nneocogi-com={{path={:?}}}\n{extra}",
                    consumer.facade
                ),
            )
            .unwrap();
        }
        let manifest = root.join("Cargo.toml");
        let mut source = fs::read_to_string(&manifest).unwrap();
        source.push_str("binding-provider={path='provider'}\n");
        fs::write(manifest, source).unwrap();
        for (file, source) in [("old", old), ("current", current)] {
            let graph = graph(source, None);
            let mut paths = RustModulePaths::default();
            paths.insert("proof.value", &format!("binding_helper::{file}")).unwrap();
            fs::write(
                root.join(format!("helper/src/{file}.rs")),
                generate_rust_with_paths(&graph, "proof.value", &paths).unwrap(),
            )
            .unwrap();
        }
        Self { consumer }
    }

    /// Changes only safe facade exports. Current module constants may coexist
    /// with an older explicit declaration, exactly as in an ordinary library.
    fn select(&self, export: &str) {
        self.write(
            "helper/src/lib.rs",
            &format!(
                r#"
            #![allow(dead_code)]
            pub mod old; pub mod current;
            pub mod selected {{ pub use super::current::*; {export} }}
            pub mod r#mod {{ pub mod r#type {{ pub use crate::selected::*; }} }}
        "#
            ),
        );
    }

    /// Emits the provider's real source selection, optionally through raw nested
    /// identifiers. The same emitter is used by both Cargo source integrations.
    fn provider(&self, values: &str, service: &str, path: &str, implementation: &str) {
        let graph = graph(values, Some(service));
        let mut paths = RustModulePaths::default();
        paths.insert("proof.value", path).unwrap();
        paths.insert("proof.service", "binding_provider::service").unwrap();
        let generated = generate_rust_with_paths(&graph, "proof.service", &paths).unwrap();
        self.write("provider/src/service.rs", &generated);
        self.write(
            "provider/src/lib.rs",
            &format!("extern crate self as binding_provider; pub mod service; {implementation}"),
        );
    }

    /// Installs independent current host bindings, so success proves the ABI
    /// call rather than merely comparing signatures or compiling one type.
    fn host(&self, values: &str) {
        let graph = graph(values, Some(SERVICE));
        let mut paths = RustModulePaths::default();
        paths.insert("proof.value", "binding_admission_host::value").unwrap();
        self.write("src/value.rs", &generate_rust_with_paths(&graph, "proof.value", &paths).unwrap());
        self.write("src/service.rs", &generate_rust_with_paths(&graph, "proof.service", &paths).unwrap());
        self.write(
            "src/lib.rs",
            r#"
            extern crate self as binding_admission_host;
            pub mod value; pub mod service;
            #[test] fn real_call() {
                let service = binding_provider::root().query::<service::ServiceAbi>().unwrap();
                let output = service.increment(value::Value { height: 3, width: 4 }).unwrap();
                assert_eq!((output.height, output.width), (3, 5));
            }
        "#,
        );
    }

    /// Writes only fixture-owned sources; production SDK files are never edited.
    fn write(&self, file: &str, source: &str) {
        fs::write(self.consumer.directory.path().join(file), source).unwrap();
    }

    /// Uses Cargo's selected compiler and inherited one-job/target configuration.
    /// Each child completes before the next case or fixture teardown.
    fn cargo(&self, verb: &str) -> Output {
        Command::new("cargo")
            .args([verb, "--offline", "--lib", "--manifest-path"])
            .arg(self.consumer.directory.path().join("Cargo.toml"))
            .output()
            .unwrap()
    }

    /// A semantic type mismatch must fail in the generated identity obligation,
    /// not merely because an unrelated fixture name or trait failed to compile.
    /// Returns those diagnostics for cases that also constrain the supplying path.
    fn rejected(&self, label: &str, declaration: &str) -> Output {
        let output = self.cargo("check");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{label}: stale binding compiled");
        assert!(
            stderr.contains("E0080") && stderr.contains("IDL binding mismatch") && stderr.contains(declaration),
            "{label}: {stderr}"
        );
        output
    }
}

/// Resolves exact source revisions once; generation must consume their completed
/// declaration signatures rather than recalculate identities from Rust syntax.
fn graph(values: &str, service: Option<&str>) -> ResolvedModules {
    let mut modules = vec![crate::parse_module(Path::new("proof/value.ncidl"), values).unwrap()];
    if let Some(service) = service {
        modules.push(crate::parse_module(Path::new("proof/service.ncidl"), service).unwrap());
    }
    crate::resolve_modules(modules).unwrap()
}

/// Matching module constants do not establish the identity of an explicit type
/// reexport. Both stale revision and same-revision nominal substitution reject.
#[test]
fn imported_declaration_identity_rejects_stale_and_nominal_facades() {
    let fixture = Fixture::new("binding-admission-host", OLD_VALUES, CURRENT_VALUES);
    fixture.provider(CURRENT_VALUES, SERVICE, "binding_helper::selected", "");
    fixture.select("pub use super::old::Value;");
    fixture.rejected("current module, old Value", "proof.value.Value");
    fixture.select("pub use super::current::Other as Value;");
    fixture.rejected("current module, wrong nominal Value", "proof.value.Value");
    fixture.select("");
    fixture.provider(
        CURRENT_VALUES,
        SERVICE,
        "binding_helper::selected",
        r#"
        #[neocogi_com::com_object(service::Service)] struct Counter;
        impl service::Service for Counter {
            fn increment(&self, value: binding_helper::selected::Value)
                -> Result<binding_helper::selected::Value, neocogi_com::NcStatus> {
                Ok(binding_helper::selected::Value { height: value.height, width: value.width + 1 })
            }
        }
        pub fn root() -> neocogi_com::ComPtr<neocogi_com::IUnknown> { Counter::from_value(Counter) }
    "#,
    );
    fixture.host(CURRENT_VALUES);
    for export in ["", "pub use super::old::{MODULE_ID, ABI_FINGERPRINT};"] {
        fixture.select(export);
        let output = fixture.cargo("test");
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(String::from_utf8_lossy(&output.stdout).contains("test real_call ... ok"));
    }
}

/// Obligations follow actual emission for imports even without calls, qualified
/// references without use, nested fields, borrowed values and both buffer directions.
#[test]
fn imported_declaration_checks_cover_each_lowering_route() {
    let fixture = Fixture::new("binding-route-proof", OLD_VALUES, CURRENT_VALUES);
    fixture.select("pub use super::old::Value;");
    for (label, service) in [
        ("unused emitted import", "use proof.value::Value; #[version(1)] interface Service {}"),
        (
            "fully qualified",
            "#[version(1)] interface Service { fn value(&self, value: proof::value::Value) -> Result<proof::value::Value>; }",
        ),
        (
            "nested array field",
            "use proof.value::Value; #[version(1)] struct Nested { values: [Value; 2] }",
        ),
        (
            "borrowed value",
            "use proof.value::Value; #[version(1)] interface Service { fn value(&self, value: &Value) -> Result<()>; }",
        ),
        (
            "read buffer",
            "use proof.value::Value; #[version(1)] interface Service { fn value(&self, value: &[Value]) -> Result<()>; }",
        ),
        (
            "output buffer",
            "use proof.value::Value; #[version(1)] interface Service { fn value(&self, value: &mut [Value]) -> Result<()>; }",
        ),
    ] {
        fixture.provider(CURRENT_VALUES, &format!("extern proof.value; {service}"), "binding_helper::r#mod::r#type", "");
        fixture.rejected(label, "proof.value.Value");
    }
}

/// Interface root arguments, results and physical inheritance all certify the
/// selected Abi marker, even when only its public trait is emitted as an import.
#[test]
fn imported_interface_identity_rejects_stale_markers() {
    let fixture = Fixture::new("binding-interface-proof", OLD_BASE, CURRENT_BASE);
    fixture.select("pub use super::old::{Base, BaseAbi, BaseVTable, BaseVtblFor, BaseClient};");
    for (label, source) in [
        ("base", "use proof.value::Base; #[version(1)] interface Service: Base {}"),
        (
            "borrowed",
            "#[version(1)] interface Service { fn value(&self, value: &proof::value::Base) -> Result<()>; }",
        ),
        (
            "strong result",
            "#[version(1)] interface Service { fn value(&self) -> Result<ComPtr<proof::value::Base>>; }",
        ),
        (
            "weak result",
            "#[version(1)] interface Service { fn value(&self) -> Result<WeakComPtr<proof::value::Base>>; }",
        ),
        (
            "exclusive transfer",
            "#[version(1)] interface Service { fn value(&self, value: ComBox<proof::value::Base>) -> Result<ComBox<proof::value::Base>>; }",
        ),
        (
            "exclusive weak result",
            "#[version(1)] interface Service { fn value(&self) -> Result<WeakComBox<proof::value::Base>>; }",
        ),
    ] {
        fixture.provider(CURRENT_BASE, &format!("extern proof.value; {source}"), "binding_helper::selected", "");
        fixture.rejected(label, "proof.value.Base");
    }
}

/// Scalar, owning and payload codecs all use the selected type's identity, not
/// only copied record layout. These are separate generated consumers so one
/// rejected scalar cannot conceal a missing obligation for another value family.
#[test]
fn imported_value_families_all_require_their_own_identity() {
    for (label, old, current) in [
        (
            "scalar",
            "#[version(1)] enum Value: u8 { Ready = 1 }",
            "#[version(1)] enum Value: u8 { Ready = 2 }",
        ),
        (
            "flags",
            "#[version(1)] bitenum Value: u8 { READY = 1 }",
            "#[version(1)] bitenum Value: u8 { READY = 2 }",
        ),
        (
            "owned",
            "#[version(1)] struct Value { width: String, height: String }",
            "#[version(1)] struct Value { height: String, width: String }",
        ),
        (
            "payload",
            "#[version(1)] enum Value: u8 { Pair { width: u32, height: u32 }, Empty }",
            "#[version(1)] enum Value: u8 { Pair { height: u32, width: u32 }, Empty }",
        ),
    ] {
        let fixture = Fixture::new(&format!("binding-{label}-proof"), old, current);
        fixture.select("pub use super::old::Value;");
        fixture.provider(
            current,
            "extern proof.value; use proof.value::Value; #[version(1)] interface Service { fn value(&self, value: &Value) -> Result<()>; }",
            "binding_helper::selected",
            "",
        );
        fixture.rejected(label, "proof.value.Value");
    }
}

/// Imported open templates are semantic inputs, not phantom Rust declarations.
/// Their local closed projections still select/check actual external free types.
#[test]
fn closed_specializations_check_free_types_without_requiring_open_rust_types() {
    let template = " #[version(1)] struct Envelope<T> { supplied: T, free: Value }";
    let old = format!("{OLD_VALUES}{template}");
    let current = format!("{CURRENT_VALUES}{template}");
    let service =
        "extern proof.value; use proof.value::Envelope; #[version(1)] interface Service { fn value(&self, value: Envelope<u32>) -> Result<Envelope<u32>>; }";
    let fixture = Fixture::new("binding-specialization-proof", &old, &current);
    fixture.provider(&current, service, "binding_helper::selected", "");
    fixture.select("pub use super::old::Value;");
    fixture.rejected("closed template free type", "proof.value.Value");
    fixture.select("");
    let output = fixture.cargo("check");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

/// Checking BaseAbi alone is insufficient if a facade's unrelated BaseClient or
/// BaseVTable name still controls inherited unsafe casts/physical prefixes.
/// Both normal and printed object expansion must use the checked associations.
#[test]
fn inherited_calls_use_checked_marker_associations_instead_of_stale_names() {
    let fixture = Fixture::new("binding-client-proof", OLD_BASE, CURRENT_BASE);
    let manifest = fixture.consumer.directory.path().join("Cargo.toml");
    let mut source = fs::read_to_string(&manifest).unwrap();
    source.push_str("binding-helper={path='helper'}\n");
    fs::write(manifest, source).unwrap();
    let service = "extern proof.value; use proof.value::Base; #[version(1)] interface Service: Base {}";
    let graph = graph(CURRENT_BASE, Some(service));
    let mut paths = RustModulePaths::default();
    paths.insert("proof.value", "binding_helper::selected").unwrap();
    fixture.write("src/service.rs", &generate_rust_with_paths(&graph, "proof.service", &paths).unwrap());
    fixture.write(
        "src/lib.rs",
        r#"
        pub mod service;
        #[test] fn inherited_and_queried_calls_agree() {
            let root = binding_provider::root();
            let derived = root.query::<service::ServiceAbi>().unwrap();
            let base = root.query::<binding_helper::selected::BaseAbi>().unwrap();
            assert_eq!((derived.height().unwrap(), derived.width().unwrap()), (3, 4));
            assert_eq!((base.height().unwrap(), base.width().unwrap()), (3, 4));
        }
    "#,
    );
    for printed in [false, true] {
        let object = if printed {
            let expanded = crate::codegen::expand_com_object(
                syn::parse_quote!(service::Service),
                syn::parse_quote!(
                    struct Counter;
                ),
            );
            prettyplease::unparse(&syn::parse2(expanded).unwrap())
        } else {
            "#[neocogi_com::com_object(service::Service)] struct Counter;".to_owned()
        };
        fixture.provider(
            CURRENT_BASE,
            service,
            "binding_helper::selected",
            &format!(
                r#"
            {object}
            impl binding_helper::selected::Base for Counter {{
                fn height(&self) -> Result<u32, neocogi_com::NcStatus> {{ Ok(3) }}
                fn width(&self) -> Result<u32, neocogi_com::NcStatus> {{ Ok(4) }}
            }}
            impl service::Service for Counter {{}}
            pub fn root() -> neocogi_com::ComPtr<neocogi_com::IUnknown> {{ Counter::from_value(Counter) }}
        "#
            ),
        );
        for export in [
            "pub use super::old::BaseClient;",
            "pub use super::old::BaseVTable;",
            "pub use super::old::{BaseClient, BaseVTable};",
        ] {
            fixture.select(export);
            let output = fixture.cargo("test");
            assert!(
                output.status.success(),
                "printed={printed}, {export}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(String::from_utf8_lossy(&output.stdout).contains("test inherited_and_queried_calls_agree ... ok"));
        }
    }
    // A stale constructor cannot supply the current marker's physical table.
    // The generic provider body must reject its incompatible type/trait bounds.
    fixture.select("pub use super::old::{BaseVTable, BaseVtblFor};");
    let output = fixture.cargo("check");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("E0308") || stderr.contains("E0599") || stderr.contains("E0277"), "{stderr}");
}

/// Both source integrations must reject the same compiled facade mismatch.
/// The helper is an ordinary Rust library that optionally publishes its source
/// graph; no standalone production contract package is introduced by the test.
#[test]
fn shared_and_packaged_sources_certify_actual_imports_before_real_calls() {
    let fixture = Fixture::new("binding-admission-host", OLD_VALUES, CURRENT_VALUES);
    let root = fixture.consumer.directory.path();
    fs::create_dir_all(root.join("schemas/proof")).unwrap();
    fixture.write("schemas/proof/value.ncidl", CURRENT_VALUES);
    fixture.write("schemas/proof/service.ncidl", SERVICE);
    let idl = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (package, module) in [("helper", "proof.value"), ("provider", "proof.service")] {
        let extra = if package == "provider" { "binding-helper={path='../helper'}\n" } else { "" };
        fixture.write(&format!("{package}/Cargo.toml"), &format!(
            "[package]\nname='binding-{package}'\nversion='0.0.0'\nedition='2024'\nlinks={:?}\n[dependencies]\nneocogi-com={{path={:?}}}\n{extra}[build-dependencies]\nneocogi-idl={{path={idl:?}}}\n",
            crate::cargo::contract_links_name(module), fixture.consumer.facade
        ));
    }
    fixture.write(
        "helper/build.rs",
        r#"
        fn main() {
            neocogi_idl::cargo::Module::new("../schemas", "proof/value.ncidl")
                .rust_path("binding_helper::selected").build().unwrap();
        }
    "#,
    );
    fixture.write(
        "provider/src/lib.rs",
        r#"
        extern crate self as binding_provider;
        pub mod service { include!(env!("SERVICE_BINDINGS")); }
        #[neocogi_com::com_object(service::Service)] struct Counter;
        impl service::Service for Counter {
            fn increment(&self, value: binding_helper::selected::Value)
                -> Result<binding_helper::selected::Value, neocogi_com::NcStatus> {
                Ok(binding_helper::selected::Value { height: value.height, width: value.width + 1 })
            }
        }
        pub fn root() -> neocogi_com::ComPtr<neocogi_com::IUnknown> { Counter::from_value(Counter) }
    "#,
    );
    fixture.host(CURRENT_VALUES);
    for (label, selection) in [
        (
            "shared",
            r#"neocogi_idl::cargo::SharedSources::new("../schemas")
            .module("proof.service", "binding_provider::service")
            .import("proof.value", "binding_helper::selected").build().unwrap().remove(0)"#,
        ),
        (
            "packaged",
            r#"neocogi_idl::cargo::Module::new("../schemas", "proof/service.ncidl")
            .rust_path("binding_provider::service").build().unwrap()"#,
        ),
    ] {
        fixture.write(
            "provider/build.rs",
            &format!(
                r#"
            fn main() {{
                let output = {selection};
                println!("cargo::rustc-env=SERVICE_BINDINGS={{}}", output.include.display());
            }}
        "#
            ),
        );
        fixture.select("pub use super::old::Value;");
        fixture.rejected(label, "proof.value.Value");
        fixture.select("");
        let output = fixture.cargo("test");
        assert!(output.status.success(), "{label}: {}", String::from_utf8_lossy(&output.stderr));
        assert!(String::from_utf8_lossy(&output.stdout).contains("test real_call ... ok"));
    }
}

/// A closed template must check its free value through the actual immediate
/// provider's UUID route, even when the consumer cannot name the leaf crate.
/// The host generates its own coherent types and executes the inherited call.
#[test]
fn transitive_template_free_types_require_the_selected_leaf_identity() {
    let fixture = Fixture::new("binding-transitive-host", OLD_VALUES, CURRENT_VALUES);
    let root = fixture.consumer.directory.path();
    fs::create_dir_all(root.join("schemas/proof")).unwrap();
    fs::create_dir_all(root.join("bridge/src")).unwrap();
    let bridge =
        "extern proof.value; #[version(1)] interface Forward<T> { fn increment(&self, value: proof::value::Value, amount: T) -> Result<proof::value::Value>; }";
    let service = "extern proof.bridge; #[version(1)] interface Service: proof::bridge::Forward<u32> {}";
    let graph = crate::resolve_modules(vec![
        crate::parse_module(Path::new("proof/value.ncidl"), CURRENT_VALUES).unwrap(),
        crate::parse_module(Path::new("proof/bridge.ncidl"), bridge).unwrap(),
        crate::parse_module(Path::new("proof/service.ncidl"), service).unwrap(),
    ])
    .unwrap();
    let forward = &graph
        .module("proof.service")
        .unwrap()
        .source
        .declarations
        .iter()
        .find(|value| value.specialization.is_some())
        .unwrap()
        .name;
    let idl = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (package, module, source, dependency) in [
        ("helper", "proof.value", CURRENT_VALUES, ""),
        ("bridge", "proof.bridge", bridge, "binding-helper={path='../helper'}\n"),
        ("provider", "proof.service", service, "binding-bridge={path='../bridge'}\n"),
    ] {
        fixture.write(&format!("schemas/{}.ncidl", module.replace('.', "/")), source);
        fixture.write(&format!("{package}/Cargo.toml"), &format!(
            "[package]\nname='binding-{package}'\nversion='0.0.0'\nedition='2024'\nlinks={:?}\n[dependencies]\nneocogi-com={{path={:?}}}\n{dependency}[build-dependencies]\nneocogi-idl={{path={idl:?}}}\n",
            crate::cargo::contract_links_name(module), fixture.consumer.facade
        ));
        let rust_path = match package {
            "helper" => "binding_helper::selected",
            "bridge" => "binding_bridge::bindings",
            _ => "binding_provider::service",
        };
        fixture.write(
            &format!("{package}/build.rs"),
            &format!(
                r#"fn main() {{
                    let output = neocogi_idl::cargo::Module::new("../schemas", {:?})
                        .rust_path({rust_path:?}).build().unwrap();
                    println!("cargo::rustc-env=BINDINGS={{}}", output.include.display());
                }}"#,
                format!("{}.ncidl", module.replace('.', "/"))
            ),
        );
    }
    fixture.write(
        "bridge/src/lib.rs",
        r#"
        extern crate self as binding_bridge;
        pub mod bindings { include!(env!("BINDINGS")); }
        pub use binding_helper::selected::Value;
    "#,
    );
    fixture.write(
        "provider/src/lib.rs",
        &format!(
            r#"
        extern crate self as binding_provider;
        pub mod service {{ include!(env!("BINDINGS")); }}
        #[neocogi_com::com_object(service::Service)] struct Counter;
        impl service::{forward} for Counter {{
            fn increment(&self, value: binding_bridge::Value, amount: u32)
                -> Result<binding_bridge::Value, neocogi_com::NcStatus> {{
                Ok(binding_bridge::Value {{ height: value.height, width: value.width + amount }})
            }}
        }}
        impl service::Service for Counter {{}}
        pub fn root() -> neocogi_com::ComPtr<neocogi_com::IUnknown> {{ Counter::from_value(Counter) }}
        "#
        ),
    );
    let mut paths = RustModulePaths::default();
    paths.insert("proof.value", "binding_transitive_host::value").unwrap();
    fixture.write("src/value.rs", &generate_rust_with_paths(&graph, "proof.value", &paths).unwrap());
    fixture.write("src/service.rs", &generate_rust_with_paths(&graph, "proof.service", &paths).unwrap());
    fixture.write(
        "src/lib.rs",
        r#"
        extern crate self as binding_transitive_host;
        pub mod value; pub mod service;
        #[test] fn transitive_real_call() {
            let service = binding_provider::root().query::<service::ServiceAbi>().unwrap();
            let output = service.increment(value::Value { height: 3, width: 4 }, 1).unwrap();
            assert_eq!((output.height, output.width), (3, 5));
        }
    "#,
    );
    fixture.select("pub use super::old::Value;");
    let rejected = fixture.rejected("transitive template free value", "proof.value.Value");
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("binding_bridge::bindings::__nc_dependencies::module_"),
        "{}",
        String::from_utf8_lossy(&rejected.stderr)
    );
    fixture.select("");
    let output = fixture.cargo("test");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("test transitive_real_call ... ok"));
}

/// A mapped module is not itself a selected compiled declaration. Neither an
/// unrelated mapping nor an open template needs a phantom Rust owner crate.
#[test]
fn semantic_templates_and_unused_mappings_do_not_require_rust_owners() {
    let fixture = Fixture::new("binding-unused-proof", OLD_VALUES, CURRENT_VALUES);
    fixture.select("");
    let graph = crate::resolve_modules(vec![
        crate::parse_module(Path::new("proof/open.ncidl"), "#[version(1)] struct Envelope<T> { item: T }").unwrap(),
        crate::parse_module(Path::new("proof/unused.ncidl"), "#[version(1)] struct NeverSelected { item: u32 }").unwrap(),
        crate::parse_module(
            Path::new("proof/service.ncidl"),
            "extern proof.open; use proof.open::Envelope; #[version(1)] interface Service { fn value(&self, value: Envelope<u32>) -> Result<Envelope<u32>>; }",
        )
        .unwrap(),
    ])
    .unwrap();
    let mut paths = RustModulePaths::default();
    paths.insert("proof.service", "binding_provider::service").unwrap();
    paths.insert("proof.open", "absent_open_owner::bindings").unwrap();
    paths.insert("proof.unused", "absent_unused_owner::bindings").unwrap();
    fixture.write("provider/src/service.rs", &generate_rust_with_paths(&graph, "proof.service", &paths).unwrap());
    fixture.write("provider/src/lib.rs", "extern crate self as binding_provider; pub mod service;");
    let output = fixture.cargo("check");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

/// The source marker's nominal interface identity must match even when a
/// different interface in that same module has indistinguishable slot types.
#[test]
fn nominal_interface_substitution_is_rejected() {
    let values = format!("{CURRENT_BASE} #[version(1)] interface Other {{ fn height(&self) -> Result<u32>; fn width(&self) -> Result<u32>; }}");
    let fixture = Fixture::new("binding-nominal-interface", &values, &values);
    fixture.select("pub use super::current::{Other as Base, OtherAbi as BaseAbi};");
    fixture.provider(
        &values,
        "extern proof.value; use proof.value::Base; #[version(1)] interface Service {}",
        "binding_helper::selected",
        "",
    );
    fixture.rejected("same-module nominal interface", "proof.value.Base");
}

/// com_object's existing typed projection proof must keep rejecting a stale
/// root provider even when the Rust implementation satisfies both old/new traits.
/// This is independent of IDL's external-use checks and inherited view selection.
#[test]
fn object_projection_identity_still_rejects_a_stale_root_provider() {
    let fixture = Fixture::new("binding-root-provider", OLD_BASE, CURRENT_BASE);
    fixture.select("pub use super::old::BaseVtblFor;");
    fixture.provider(
        CURRENT_BASE,
        "#[version(1)] interface Service {}",
        "binding_helper::selected",
        r#"
        #[neocogi_com::com_object(interfaces(binding_helper::selected::Base))]
        struct Counter;
        impl binding_helper::current::Base for Counter {
            fn height(&self) -> Result<u32, neocogi_com::NcStatus> { Ok(3) }
            fn width(&self) -> Result<u32, neocogi_com::NcStatus> { Ok(4) }
        }
        impl binding_helper::old::Base for Counter {
            fn height(&self) -> Result<u32, neocogi_com::NcStatus> { Ok(3) }
            fn width(&self) -> Result<u32, neocogi_com::NcStatus> { Ok(4) }
        }
        pub fn root() -> neocogi_com::ComPtr<neocogi_com::IUnknown> { Counter::from_value(Counter) }
    "#,
    );
    fixture.write("src/lib.rs", "#[test] fn construct() { drop(binding_provider::root()); }");
    let output = fixture.cargo("test");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        stderr.contains("E0080") && stderr.contains("conflicting inherited interface contracts") && stderr.contains("VTABLES"),
        "{stderr}"
    );
}
