//! Imported Rust declarations must prove the exact resolved IDL contract.
//!
//! All unsafe codecs, tables and callers in these fixtures come from the real
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
const OLD_VALUES: &str = "struct Value { width: u32, height: u32 }";
/// A second same-layout name makes nominal admission independent of layout.
const CURRENT_VALUES: &str = "struct Value { height: u32, width: u32 } struct Other { height: u32, width: u32 }";
/// Independent provider and host must agree on the actual copied codec.
const SERVICE: &str = "extern proof.value; use proof.value::Value; interface Service { fn increment(&self, value: Value) -> Result<Value>; }";

/// Both bases have the same physical slot types but opposite method meanings.
const OLD_BASE: &str = "interface Base { fn width(&self) -> Result<u32>; fn height(&self) -> Result<u32>; }";
/// The actual checked marker must determine this height/width ordering.
const CURRENT_BASE: &str = "interface Base { fn height(&self) -> Result<u32>; fn width(&self) -> Result<u32>; }";

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
                    "[package]\nname='binding-{package}'\nversion='0.0.0'\nedition='2024'\n[dependencies]\nneocogi-com-sdk={{path={:?}}}\n{extra}",
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

    /// Gives the same real helper/provider graph to either Cargo source API.
    /// The helper publishes graph ownership; its actual supplied Rust definitions
    /// remain the independent coherent (or deliberately overridden) fixture.
    fn source_packages(&self, values: &str, service: &str) {
        let root = self.consumer.directory.path();
        fs::create_dir_all(root.join("schemas/proof")).unwrap();
        self.write("schemas/proof/value.ncidl", values);
        self.write("schemas/proof/service.ncidl", service);
        let idl = Path::new(env!("CARGO_MANIFEST_DIR"));
        for (package, module) in [("helper", "proof.value"), ("provider", "proof.service")] {
            let extra = if package == "provider" { "binding-helper={path='../helper'}\n" } else { "" };
            self.write(&format!("{package}/Cargo.toml"), &format!(
            "[package]\nname='binding-{package}'\nversion='0.0.0'\nedition='2024'\nlinks={:?}\n[dependencies]\nneocogi-com-sdk={{path={:?}}}\n{extra}[build-dependencies]\nneocogi-idl={{path={idl:?}}}\n",
            crate::cargo::contract_links_name(module), self.consumer.facade
        ));
        }
        self.write(
            "helper/build.rs",
            r#"
        fn main() {
            neocogi_idl::cargo::Module::new("../schemas", "proof/value.ncidl")
                .rust_path("binding_helper::selected").build().unwrap();
        }
    "#,
        );
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

    /// Each fixture owns its build artifacts: these graphs deliberately reuse
    /// helper/provider package names while changing their generated contracts.
    /// Sharing a target between parallel fixtures can reuse another graph's
    /// metadata or executable and invalidate both acceptance and rejection tests.
    fn cargo(&self, verb: &str) -> Output {
        Command::new("cargo")
            .args([verb, "--offline", "--lib", "--manifest-path"])
            .arg(self.consumer.directory.path().join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", self.consumer.directory.path().join("target"))
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
        #[neocogi_com_sdk::com_object(service::Service)] struct Counter;
        impl service::Service for Counter {
            fn increment(&self, value: binding_helper::selected::Value)
                -> Result<binding_helper::selected::Value, neocogi_com_sdk::NcStatus> {
                Ok(binding_helper::selected::Value { height: value.height, width: value.width + 1 })
            }
        }
        pub fn root() -> neocogi_com_sdk::ComPtr<neocogi_com_sdk::IUnknown> { Counter::from_value(Counter) }
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

/// A selected generated type must prove its Rust projection even when its
/// native value identity independently matches the resolved module.
#[test]
fn imported_declaration_checks_rust_proof_separately_from_native_signature() {
    let fixture = Fixture::new("binding-rust-proof", CURRENT_VALUES, CURRENT_VALUES);
    let helper = fixture.consumer.directory.path().join("helper/src/old.rs");
    let old = fs::read_to_string(helper).unwrap();
    // Leave the real value, codec, native identity, and module constants intact.
    // Change only Value's hidden Rust proof in the selected fixture module.
    let proof = "impl Value {\n    #[doc(hidden)]\n    pub const __NC_RUST_BINDING: [::core::primitive::u8; 32] = [";
    let (before, generated_digest) = old.split_once(proof).unwrap();
    let (_, after) = generated_digest.split_once("];").unwrap();
    fixture.write("helper/src/old.rs", &format!("{before}{proof}0; 32];{after}"));
    fixture.provider(CURRENT_VALUES, SERVICE, "binding_helper::selected", "");
    fixture.select("pub use super::old::Value;");
    fixture.rejected("same native signature, stale Rust projection", "proof.value.Value");
    fixture.select("");
    let result = fixture.cargo("check");
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
}

/// Native peers agree on status-only calls. A stale Rust binding with a different
/// return projection must still fail when selected through a facade.
#[test]
fn native_compatible_result_projections_require_current_rust_bindings() {
    let old = "interface Base { fn ready(&self) -> Result<()>; }";
    let direct = "interface Base { fn ready(&self); }";
    let first = graph(old, None);
    let second = graph(direct, None);
    let first = first.module("proof.value").unwrap().declaration("Base").unwrap();
    let second = second.module("proof.value").unwrap().declaration("Base").unwrap();
    assert_eq!(first.abi_signature, second.abi_signature);
    assert_ne!(first.rust_binding, second.rust_binding);

    let fixture = Fixture::new("binding-return-projection", old, direct);
    fixture.provider(
        direct,
        "extern proof.value; use proof.value::Base; interface Service: Base {}",
        "binding_helper::selected",
        "",
    );
    fixture.select("pub use super::old::BaseAbi;");
    fixture.rejected("fallible marker selected when direct was expected", "proof.value.Base");
    fixture.select("");
    let result = fixture.cargo("check");
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
}

/// A native-equivalent leaf change invalidates transitive Rust declarations
/// exposed through a copied method boundary and a closed generic aggregate.
#[test]
fn native_compatible_projection_changes_reach_transitive_rust_imports() {
    let old = "interface Base { fn ready(&self) -> Result<()>; }";
    let direct = "interface Base { fn ready(&self); }";
    let fixture = Fixture::new("binding-transitive-return-projection", old, direct);
    let carrier = "
        extern proof.value;
        use proof.value::Base;
        struct Bundle<T> { item: T }
        struct Ticket { item: Bundle<ComPtr<Base>> }
        interface Bridge {
            fn forward(&self, source: &Base) -> Result<ComPtr<Base>>;
            fn receive(&self, ticket: &Ticket) -> Result<()>;
        }
    ";
    let service = "
        extern proof.carrier;
        struct Parcel { ticket: proof::carrier::Ticket }
        interface Service {
            fn relay(&self, bridge: &proof::carrier::Bridge, parcel: &Parcel) -> Result<()>;
        }
    ";
    for (version, leaf) in [("old", old), ("current", direct)] {
        let modules = [("proof/value.ncidl", leaf), ("proof/carrier.ncidl", carrier), ("proof/service.ncidl", service)]
            .map(|(path, input)| crate::parse_module(Path::new(path), input).unwrap())
            .to_vec();
        let graph = crate::resolve_modules(modules).unwrap();
        let mut paths = RustModulePaths::default();
        paths.insert("proof.value", &format!("binding_helper::{version}")).unwrap();
        paths.insert("proof.carrier", "binding_helper::selected").unwrap();
        fixture.write(
            &format!("helper/src/{version}_carrier.rs"),
            &generate_rust_with_paths(&graph, "proof.carrier", &paths).unwrap(),
        );
        if version == "current" {
            fixture.write("provider/src/lib.rs", &generate_rust_with_paths(&graph, "proof.service", &paths).unwrap());
        }
    }
    for (override_export, expected) in [
        ("pub use super::old_carrier::Ticket;", "proof.carrier.Ticket"),
        ("pub use super::old_carrier::BridgeAbi;", "proof.carrier.Bridge"),
        ("", ""),
    ] {
        fixture.write(
            "helper/src/lib.rs",
            &format!(
                "extern crate self as binding_helper;
                 pub mod old; pub mod current;
                 pub mod old_carrier; pub mod current_carrier;
                 pub mod selected {{ pub use super::current_carrier::*; {override_export} }}"
            ),
        );
        if expected.is_empty() {
            let result = fixture.cargo("check");
            assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
        } else {
            fixture.rejected("stale projection reached through a dependency", expected);
        }
    }
}

/// Independent bindings exercise scoped dispatch through live first-base loans.
/// Unrelated or secondary projections, unborrowed sources and escaping loans fail.
#[test]
fn scoped_marker_requires_a_live_first_base_lender_and_a_scoped_visitor() {
    let contract = "
        interface Loan { fn value(&self) -> Result<u32>; }
        interface Base {
            #[scoped] fn tooltip(&self, visible: bool) -> Option<&Loan>;
            #[scoped] fn checked(&self, state: u32) -> Result<Option<&Loan>>;
            fn update(&self) -> Result<()>;
        }
        interface Derived: Base { fn with_tooltip(&self) -> Result<u32>; }
        interface Unrelated {}
        interface Middle: Unrelated + Base {}
        interface Secondary: Middle {}
    ";
    let fixture = Fixture::new("binding-scoped-live-source", contract, contract);
    fixture.select("");
    fixture.provider(
        contract,
        "interface Service {}",
        "binding_helper::selected",
        r#"
        use binding_helper::selected::*;
        pub use binding_helper::selected::DerivedAbi;
        use neocogi_com_sdk::{com_object, ComPtr, ComRef, LocalCom, NcStatus, NC_E_BUSY, NC_E_CLOSED};
        use std::cell::RefCell;

        #[com_object(Loan)] struct Tip;
        impl Loan for Tip {
            fn value(&self) -> Result<u32, NcStatus> { Ok(23) }
        }
        #[com_object(Derived)] pub struct Probe(RefCell<ComPtr<LoanAbi>>);
        impl Base for Probe {
            fn tooltip<R>(&self, visible: bool, lend: impl for<'a> FnOnce(Option<ComRef<'a, LoanAbi>>) -> R) -> R {
                match self.0.try_borrow() {
                    Ok(tip) => lend(visible.then(|| tip.as_com_ref())),
                    Err(_) => lend(None),
                }
            }
            fn checked<R>(&self, state: u32, lend: impl for<'a> FnOnce(Result<Option<ComRef<'a, LoanAbi>>, NcStatus>) -> R) -> R {
                if state == 2 { return lend(Err(NC_E_CLOSED)); }
                match self.0.try_borrow() {
                    Ok(tip) => lend(Ok((state == 1).then(|| tip.as_com_ref()))),
                    Err(_) => lend(Err(NC_E_BUSY)),
                }
            }
            fn update(&self) -> Result<(), NcStatus> {
                let _tip = self.0.try_borrow_mut().map_err(|_| NC_E_BUSY)?;
                Ok(())
            }
        }
        impl Derived for Probe {
            fn with_tooltip(&self) -> Result<u32, NcStatus> { Ok(47) }
        }
        pub fn local() -> LocalCom<Probe> {
            Probe::into_local(Probe(RefCell::new(Tip::from_value(Tip).query().unwrap())))
        }
        pub fn with_writer<R>(local: &LocalCom<Probe>, run: impl FnOnce() -> R) -> R {
            let _tip = local.get().0.borrow_mut();
            run()
        }
        "#,
    );
    // The caller binds the same contracts independently from the provider crate.
    fixture.write("src/value.rs", &crate::generate_rust(&graph(contract, None), "proof.value").unwrap());
    fixture.write(
        "src/lib.rs",
        r#"
        mod value;
        use value::{BaseAbi, DerivedAbi};
        use std::{ffi::c_void, sync::atomic::{AtomicUsize, Ordering::Relaxed}};
        use neocogi_com_sdk::{ComRaw, ComRef, IUnknown, NcStatus, NcInterfaceId, NC_E_BUSY, NC_E_CLOSED, lifetime::SHARED_UNKNOWN_VTABLE};

        static QUERIES: AtomicUsize = AtomicUsize::new(0);
        static RETAINS: AtomicUsize = AtomicUsize::new(0);
        unsafe extern "C" fn query(object: *mut c_void, id: *const NcInterfaceId, out: *mut ComRaw<IUnknown>) -> NcStatus {
            QUERIES.fetch_add(1, Relaxed);
            // SAFETY: The copied prefix preserves the real shared allocation and call contract.
            unsafe { (SHARED_UNKNOWN_VTABLE.query_interface)(object, id, out) }
        }
        unsafe extern "C" fn retain(object: *mut c_void) -> u32 {
            RETAINS.fetch_add(1, Relaxed);
            // SAFETY: The copied prefix preserves the real shared allocation.
            unsafe { (SHARED_UNKNOWN_VTABLE.add_ref)(object) }
        }

        #[test] fn scoped_calls() {
            let local = binding_provider::local();
            let base = local.query::<BaseAbi>().unwrap();
            let derived = local.query::<DerivedAbi>().unwrap();
            let writer = derived.clone();
            assert_eq!(BaseAbi::with_tooltip::<neocogi_com_sdk::ThreadLocal, _>(base.as_com_ref(), true, |tip| tip.value()), Some(Ok(23)));

            let raw = derived.as_raw();
            // SAFETY: The live exact table contains only copyable C slot data.
            let mut table = unsafe { raw.vtable.read() };
            table.base.unknown.query_interface = query;
            table.base.unknown.add_ref = retain;
            // SAFETY: The original owner and copied exact table outlive this live loan.
            let source = unsafe { ComRef::<DerivedAbi>::from_raw_borrowed(ComRaw { object: raw.object, vtable: &table }) }.unwrap();

            assert_eq!(BaseAbi::with_tooltip(source, true, |tip| {
                assert_eq!(source.with_tooltip(), Ok(47));
                assert_eq!(writer.update(), Err(NC_E_BUSY));
                tip.value()
            }), Some(Ok(23)));
            assert_eq!(writer.update(), Ok(()));
            assert_eq!(BaseAbi::with_tooltip(source, false, |_| panic!("unexpected direct delivery")), None::<()>);
            assert_eq!(BaseAbi::with_checked(source, 0, |_| panic!("unexpected checked delivery")), Ok(None::<()>));
            assert_eq!(BaseAbi::with_checked(source, 1, |tip| tip.value()), Ok(Some(Ok(23))));
            assert_eq!(BaseAbi::with_checked(source, 2, |_| panic!("unexpected failed delivery")), Err::<Option<()>, _>(NC_E_CLOSED));

            binding_provider::with_writer(&local, || {
                assert_eq!(BaseAbi::with_tooltip(source, true, |_| panic!("busy direct delivery")), None::<()>);
                assert_eq!(BaseAbi::with_checked(source, 1, |_| panic!("busy checked delivery")), Err::<Option<()>, _>(NC_E_BUSY));
            });
            assert_eq!(QUERIES.load(Relaxed), 0);
            assert_eq!(RETAINS.load(Relaxed), 0);
        }
        "#,
    );
    let result = fixture.cargo("test");
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    assert!(String::from_utf8_lossy(&result.stdout).contains("scoped_calls ... ok"));
    let imports = "
        mod value;
        use value::{BaseAbi, LoanAbi, UnrelatedAbi, SecondaryAbi};
        use neocogi_com_sdk::{ComRef, ComPtr};
    ";
    for (declaration, error) in [
        (
            "pub fn reject(source: ComRef<'_, SecondaryAbi>) { BaseAbi::with_tooltip(source, true, |_| ()); }",
            "__NcScopedPrefix",
        ),
        (
            "pub fn reject(source: ComRef<'_, UnrelatedAbi>) { BaseAbi::with_tooltip(source, true, |_| ()); }",
            "__NcScopedPrefix",
        ),
        (
            "pub fn reject(source: ComRef<'_, binding_provider::DerivedAbi>) { BaseAbi::with_tooltip(source, true, |_| ()); }",
            "E0271",
        ),
        (
            "pub fn reject(source: ComPtr<BaseAbi>) { BaseAbi::with_tooltip(source, true, |_| ()); }",
            "E0308",
        ),
        (
            "pub fn escape(source: ComRef<'_, BaseAbi>) -> Option<ComRef<'static, LoanAbi>> {
             BaseAbi::with_tooltip(source, true, |loan| loan)
         }",
            "lifetime may not live long enough",
        ),
    ] {
        fixture.write("src/lib.rs", &format!("{imports}{declaration}"));
        let result = fixture.cargo("check");
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(!result.status.success() && stderr.contains(error), "{stderr}");
    }
}

/// Obligations follow actual emission for imports even without calls, qualified
/// references without use, nested fields, borrowed values and both buffer directions.
#[test]
fn imported_declaration_checks_cover_each_lowering_route() {
    let fixture = Fixture::new("binding-route-proof", OLD_VALUES, CURRENT_VALUES);
    fixture.select("pub use super::old::Value;");
    for (label, service) in [
        ("unused emitted import", "use proof.value::Value; interface Service {}"),
        (
            "fully qualified",
            "interface Service { fn value(&self, value: proof::value::Value) -> Result<proof::value::Value>; }",
        ),
        ("nested array field", "use proof.value::Value; struct Nested { values: [Value; 2] }"),
        (
            "borrowed value",
            "use proof.value::Value; interface Service { fn value(&self, value: &Value) -> Result<()>; }",
        ),
        (
            "read buffer",
            "use proof.value::Value; interface Service { fn value(&self, value: &[Value]) -> Result<()>; }",
        ),
        (
            "output buffer",
            "use proof.value::Value; interface Service { fn value(&self, value: &mut [Value]) -> Result<()>; }",
        ),
    ] {
        fixture.provider(CURRENT_VALUES, &format!("extern proof.value; {service}"), "binding_helper::r#mod::r#type", "");
        fixture.rejected(label, "proof.value.Value");
    }
}

/// Interface root arguments, results and physical inheritance all certify the
/// selected Abi marker. Forwarding the full imported family must not replace
/// that proof with the identity of an independently named sibling alias.
#[test]
fn imported_interface_identity_rejects_stale_markers() {
    let fixture = Fixture::new("binding-interface-proof", OLD_BASE, CURRENT_BASE);
    fixture.select("pub use super::old::{Base, BaseAbi, BaseVTable, BaseVtblFor, BaseWrapper};");
    for (label, source) in [
        ("base", "use proof.value::Base; interface Service: Base {}"),
        ("borrowed", "interface Service { fn value(&self, value: &proof::value::Base) -> Result<()>; }"),
        ("strong result", "interface Service { fn value(&self) -> Result<ComPtr<proof::value::Base>>; }"),
        (
            "weak result",
            "interface Service { fn value(&self) -> Result<WeakComPtr<proof::value::Base>>; }",
        ),
        (
            "exclusive transfer",
            "interface Service { fn value(&self, value: ComBox<proof::value::Base>) -> Result<ComBox<proof::value::Base>>; }",
        ),
        ("receiver-bound result", "interface Service { fn value(&self) -> Result<&proof::value::Base>; }"),
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
        ("scalar", "enum Value: u8 { Ready = 1 }", "enum Value: u8 { Ready = 2 }"),
        ("flags", "bitenum Value: u8 { READY = 1 }", "bitenum Value: u8 { READY = 2 }"),
        (
            "owned",
            "struct Value { width: String, height: String }",
            "struct Value { height: String, width: String }",
        ),
        (
            "payload",
            "enum Value: u8 { Pair { width: u32, height: u32 }, Empty }",
            "enum Value: u8 { Pair { height: u32, width: u32 }, Empty }",
        ),
    ] {
        let fixture = Fixture::new(&format!("binding-{label}-proof"), old, current);
        fixture.select("pub use super::old::Value;");
        fixture.provider(
            current,
            "extern proof.value; use proof.value::Value; interface Service { fn value(&self, value: &Value) -> Result<()>; }",
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
    let template = " struct Envelope<T> { supplied: T, free: Value }";
    let old = format!("{OLD_VALUES}{template}");
    let current = format!("{CURRENT_VALUES}{template}");
    let service = "extern proof.value; use proof.value::Envelope; interface Service { fn value(&self, value: Envelope<u32>) -> Result<Envelope<u32>>; }";
    let fixture = Fixture::new("binding-specialization-proof", &old, &current);
    fixture.provider(&current, service, "binding_helper::selected", "");
    fixture.select("pub use super::old::Value;");
    fixture.rejected("closed template free type", "proof.value.Value");
    fixture.select("");
    let output = fixture.cargo("check");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

/// Checking BaseAbi alone is insufficient if a facade's unrelated BaseWrapper or
/// BaseVTable name still controls inherited unsafe casts/physical prefixes.
/// Both normal and printed object expansion must use the checked associations.
#[test]
fn inherited_calls_use_checked_marker_associations_instead_of_stale_names() {
    let old = OLD_BASE.replace("fn width", "fn set(&mut self, value: u32) -> Result<()>; fn width");
    let current = CURRENT_BASE.replace("fn width", "fn set(&mut self, value: u32) -> Result<()>; fn width");
    let fixture = Fixture::new("binding-wrapper-proof", &old, &current);
    let manifest = fixture.consumer.directory.path().join("Cargo.toml");
    let mut source = fs::read_to_string(&manifest).unwrap();
    source.push_str("binding-helper={path='helper'}\n");
    fs::write(manifest, source).unwrap();
    let service = "extern proof.value; use proof.value::Base; interface Service: Base {}";
    let graph = graph(&current, Some(service));
    let mut paths = RustModulePaths::default();
    paths.insert("proof.value", "binding_helper::selected").unwrap();
    fixture.write("src/service.rs", &generate_rust_with_paths(&graph, "proof.service", &paths).unwrap());
    fixture.write(
        "src/lib.rs",
        r#"
        pub mod service;
        #[test] fn inherited_and_queried_calls_agree() {
            use binding_helper::selected::BaseAbi;
            use neocogi_com_sdk::{ComBox, IUnknown};
            use service::ServiceAbi;

            let mut root = binding_provider::root();
            let dimensions = |root: &ComBox<IUnknown>| {
                let derived = root.query_ref::<ServiceAbi>().unwrap();
                let base = root.query_ref::<BaseAbi>().unwrap();
                ((derived.height().unwrap(), derived.width().unwrap()), (base.height().unwrap(), base.width().unwrap()))
            };
            assert_eq!(dimensions(&root), ((3, 4), (3, 4)));
            root.query_mut::<ServiceAbi>().unwrap().set(19).unwrap();
            assert_eq!(dimensions(&root), ((19, 4), (19, 4)));
            root.query_mut::<BaseAbi>().unwrap().set(23).unwrap();
            assert_eq!(dimensions(&root), ((23, 4), (23, 4)));
        }
    "#,
    );
    for printed in [false, true] {
        let object = if printed {
            let expanded = crate::codegen::expand_com_object(
                syn::parse_quote!(service::Service, ownership = exclusive),
                syn::parse_quote!(
                    struct Counter {
                        height: u32,
                    }
                ),
            );
            prettyplease::unparse(&syn::parse2(expanded).unwrap())
        } else {
            "#[neocogi_com_sdk::com_object(service::Service, ownership = exclusive)] struct Counter { height: u32 }".to_owned()
        };
        fixture.provider(
            &current,
            service,
            "binding_helper::selected",
            &format!(
                r#"
            {object}
            impl binding_helper::selected::Base for Counter {{
                fn height(&self) -> Result<u32, neocogi_com_sdk::NcStatus> {{ Ok(self.height) }}
                fn set(&mut self, value: u32) -> Result<(), neocogi_com_sdk::NcStatus> {{ self.height = value; Ok(()) }}
                fn width(&self) -> Result<u32, neocogi_com_sdk::NcStatus> {{ Ok(4) }}
            }}
            impl service::Service for Counter {{}}
            pub fn root() -> neocogi_com_sdk::ComBox<neocogi_com_sdk::IUnknown> {{ Counter::into_box(Counter {{ height: 3 }}) }}
        "#
            ),
        );
        for export in [
            "pub use super::old::BaseWrapper;",
            "pub use super::old::BaseVTable;",
            "pub use super::old::{BaseWrapper, BaseVTable};",
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
    fixture.source_packages(CURRENT_VALUES, SERVICE);
    fixture.write(
        "provider/src/lib.rs",
        r#"
        extern crate self as binding_provider;
        pub mod service { include!(env!("SERVICE_BINDINGS")); }
        #[neocogi_com_sdk::com_object(service::Service)] struct Counter;
        impl service::Service for Counter {
            fn increment(&self, value: binding_helper::selected::Value)
                -> Result<binding_helper::selected::Value, neocogi_com_sdk::NcStatus> {
                Ok(binding_helper::selected::Value { height: value.height, width: value.width + 1 })
            }
        }
        pub fn root() -> neocogi_com_sdk::ComPtr<neocogi_com_sdk::IUnknown> { Counter::from_value(Counter) }
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
    let bridge = "extern proof.value; interface Forward<T> { fn increment(&self, value: proof::value::Value, amount: T) -> Result<proof::value::Value>; }";
    let service = "extern proof.bridge; interface Service: proof::bridge::Forward<u32> {}";
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
            "[package]\nname='binding-{package}'\nversion='0.0.0'\nedition='2024'\nlinks={:?}\n[dependencies]\nneocogi-com-sdk={{path={:?}}}\n{dependency}[build-dependencies]\nneocogi-idl={{path={idl:?}}}\n",
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
        #[neocogi_com_sdk::com_object(service::Service)] struct Counter;
        impl service::{forward} for Counter {{
            fn increment(&self, value: binding_bridge::Value, amount: u32)
                -> Result<binding_bridge::Value, neocogi_com_sdk::NcStatus> {{
                Ok(binding_bridge::Value {{ height: value.height, width: value.width + amount }})
            }}
        }}
        impl service::Service for Counter {{}}
        pub fn root() -> neocogi_com_sdk::ComPtr<neocogi_com_sdk::IUnknown> {{ Counter::from_value(Counter) }}
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
        crate::parse_module(Path::new("proof/open.ncidl"), "struct Envelope<T> { item: T }").unwrap(),
        crate::parse_module(Path::new("proof/unused.ncidl"), "struct NeverSelected { item: u32 }").unwrap(),
        crate::parse_module(
            Path::new("proof/service.ncidl"),
            "extern proof.open; use proof.open::Envelope; interface Service { fn value(&self, value: Envelope<u32>) -> Result<Envelope<u32>>; }",
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
    let values = format!("{CURRENT_BASE} interface Other {{ fn height(&self) -> Result<u32>; fn width(&self) -> Result<u32>; }}");
    let fixture = Fixture::new("binding-nominal-interface", &values, &values);
    fixture.select("pub use super::current::{Other as Base, OtherAbi as BaseAbi};");
    fixture.provider(
        &values,
        "extern proof.value; use proof.value::Base; interface Service {}",
        "binding_helper::selected",
        "",
    );
    fixture.rejected("same-module nominal interface", "proof.value.Base");
}

/// An explicit object's root marker and provider must select the same Rust
/// vtable type even when the old and new Rust slots have the same native ABI.
#[test]
fn object_projection_identity_still_rejects_a_stale_root_provider() {
    const DIRECT: &str = "interface Base { fn tick(&self); }";
    const FALLIBLE: &str = "interface Base { fn tick(&self) -> Result<()>; }";
    let fixture = Fixture::new("binding-root-provider", DIRECT, FALLIBLE);
    fixture.select("pub use super::old::BaseVtblFor;");
    fixture.provider(
        FALLIBLE,
        "interface Service {}",
        "binding_helper::selected",
        r#"
        #[neocogi_com_sdk::com_object(interfaces(binding_helper::selected::Base))]
        struct Counter;
        impl binding_helper::current::Base for Counter {
            fn tick(&self) -> Result<(), neocogi_com_sdk::NcStatus> { Ok(()) }
        }
        impl binding_helper::old::Base for Counter {
            fn tick(&self) {}
        }
        pub fn root() -> neocogi_com_sdk::ComPtr<neocogi_com_sdk::IUnknown> { Counter::from_value(Counter) }
    "#,
    );
    fixture.write("src/lib.rs", "#[test] fn construct() { drop(binding_provider::root()); }");
    let output = fixture.cargo("test");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("E0308") && stderr.contains("BaseVTable"), "{stderr}");
}

/// Source owner for the imported-provider regressions; no locally derived
/// interface may substitute for this imported trait as the object's root.
const IMPORTED_BASE: &str = "interface IBase { fn get(&self) -> Result<u32>; }";
/// An otherwise unrelated local value keeps the consuming module concrete.
const IMPORTING_MODULE: &str = "extern proof.value; use proof.value::IBase; struct Anchor { value: u32 }";

/// Selects the actual procedural macro or its printed expansion without changing
/// the provider's authored trait, state, imported bindings or method body.
fn imported_provider_source(path: &str, printed: bool, witnesses: bool) -> String {
    let object = if printed {
        let arguments = syn::parse_str(path).unwrap();
        let expanded = crate::codegen::expand_com_object(
            arguments,
            syn::parse_quote!(
                struct Provider;
            ),
        );
        prettyplease::unparse(&syn::parse2(expanded).unwrap())
    } else {
        format!("#[neocogi_com_sdk::com_object({path})] struct Provider;")
    };
    let mut source = format!(
        r#"
        {object}
        impl service::IBase for Provider {{
            fn get(&self) -> Result<u32, neocogi_com_sdk::NcStatus> {{ Ok(73) }}
        }}
        pub fn root() -> neocogi_com_sdk::ComPtr<neocogi_com_sdk::IUnknown> {{ Provider::from_value(Provider) }}
    "#
    );
    if witnesses {
        // PhantomData accepts unsized Wrapper views and requires nominal type
        // equality, without allowing a reference-returning deref coercion.
        for companion in neocogi_com::NcComInterfaceCompanion::ALL {
            let name = format!("IBase{}", companion.suffix());
            let argument = match companion {
                neocogi_com::NcComInterfaceCompanion::Provider => "<Provider>",
                _ => "",
            };
            source.push_str(&format!(
                "#[allow(dead_code)] fn same_{suffix}(x: core::marker::PhantomData<service::{name}{argument}>) -> core::marker::PhantomData<binding_helper::selected::{name}{argument}> {{ x }}\n",
                suffix = companion.suffix().to_ascii_lowercase(),
            ));
        }
    }
    source
}

/// Both root spellings query the same original Rust marker; the imported owner
/// and its borrowed view actually dispatch the generated ABI method.
fn imported_host_source() -> &'static str {
    r#"
    #[test] fn imported_call() {
        let root = binding_provider::root();
        let imported = root.query::<binding_provider::service::IBaseAbi>().unwrap();
        let original = root.query::<binding_helper::selected::IBaseAbi>().unwrap();
        assert_eq!(imported.get().unwrap(), 73);
        let borrowed: neocogi_com_sdk::ComRef<'_, binding_provider::service::IBaseAbi> = imported.as_com_ref();
        assert_eq!(borrowed.get().unwrap(), 73);
        assert_eq!(original.get().unwrap(), 73);
        assert!(imported.as_com_ref().same_object(original.as_com_ref()));
    }
    "#
}

/// Regression for the omitted operation: implement an authored imported interface
/// through its consuming module, not a local derived interface or owner alias.
#[test]
fn imported_interface_paths_construct_providers() {
    let fixture = Fixture::new("binding-import-surface-host", IMPORTED_BASE, IMPORTED_BASE);
    fixture.select("");
    let manifest = fixture.consumer.directory.path().join("Cargo.toml");
    let mut source = fs::read_to_string(&manifest).unwrap();
    source.push_str("binding-helper={path='helper'}\n");
    fs::write(manifest, source).unwrap();
    fixture.write("src/lib.rs", imported_host_source());
    // First validate identical source/setup with the owner path; it cannot count
    // as closure because both following consuming-path expansions must pass.
    for (path, printed, witnesses) in [
        ("binding_helper::selected::IBase", false, false),
        ("service::IBase", false, true),
        ("service::IBase", true, true),
    ] {
        fixture.provider(
            IMPORTED_BASE,
            IMPORTING_MODULE,
            "binding_helper::r#mod::r#type",
            &imported_provider_source(path, printed, witnesses),
        );
        // The owner-only control cannot query a nonexistent imported marker on
        // the unfixed compiler, so use the original marker solely for this row.
        if !witnesses {
            fixture.write(
                "src/lib.rs",
                &imported_host_source().replace("binding_provider::service::IBaseAbi", "binding_helper::selected::IBaseAbi"),
            );
        } else {
            fixture.write("src/lib.rs", imported_host_source());
        }
        let output = fixture.cargo("test");
        assert!(
            output.status.success(),
            "path={path}, printed={printed}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("test imported_call ... ok"));
    }
}

/// Same-name incompatible exports expose missing wrapper precedence even when
/// associated-type method calls never mention a named Wrapper or VTable.
fn imported_rival_source() -> String {
    let mut source = String::from(
        "#[allow(dead_code)] pub mod rival { pub const MODULE_NAME: &str = \"wrong\"; pub const MODULE_ID: () = (); pub const ABI_FINGERPRINT: () = ();\n",
    );
    for companion in neocogi_com::NcComInterfaceCompanion::ALL {
        let name = format!("IBase{}", companion.suffix());
        match companion {
            neocogi_com::NcComInterfaceCompanion::Provider => source.push_str(&format!("pub struct {name}<T>(core::marker::PhantomData<T>);\n")),
            neocogi_com::NcComInterfaceCompanion::Wrapper => {
                source.push_str(&format!("pub struct {name}<M = neocogi_com_sdk::ThreadLocal>(core::marker::PhantomData<M>);\n"))
            }
            neocogi_com::NcComInterfaceCompanion::Abi | neocogi_com::NcComInterfaceCompanion::VTable => source.push_str(&format!("pub struct {name};\n")),
        }
    }
    source.push_str("}\n");
    source
}

/// Both real build APIs must expose the imported provider's complete family.
/// One rival-glob case witnesses every companion without four extra projects.
#[test]
fn cargo_imported_interface_families_preserve_precedence() {
    let fixture = Fixture::new("binding-import-cargo-host", IMPORTED_BASE, IMPORTED_BASE);
    fixture.select("");
    fixture.source_packages(IMPORTED_BASE, IMPORTING_MODULE);
    let manifest = fixture.consumer.directory.path().join("Cargo.toml");
    let mut source = fs::read_to_string(&manifest).unwrap();
    source.push_str("binding-helper={path='helper'}\n");
    fs::write(manifest, source).unwrap();
    let resolved = graph(IMPORTED_BASE, Some(IMPORTING_MODULE));
    let owner = resolved.module("proof.service").unwrap();
    fixture.write("src/lib.rs", &format!(
        "{}\n#[test] fn consumer_metadata_keeps_its_owner() {{ assert_eq!(binding_provider::service::MODULE_NAME, \"proof.service\"); assert_eq!(binding_provider::service::MODULE_ID.to_uuid_bytes(), {:?}); assert_eq!(binding_provider::service::ABI_FINGERPRINT, {:?}); }}",
        imported_host_source(), owner.module_id.to_uuid_bytes(), owner.abi_fingerprint,
    ));
    for (label, selection) in [
        (
            "shared",
            r#"neocogi_idl::cargo::SharedSources::new("../schemas").module("proof.service", "binding_provider::service").import("proof.value", "binding_helper::selected").build().map(|mut outputs| outputs.remove(0))"#,
        ),
        (
            "packaged",
            r#"neocogi_idl::cargo::Module::new("../schemas", "proof/service.ncidl").rust_path("binding_provider::service").build()"#,
        ),
    ] {
        fixture.write(
            "provider/build.rs",
            &format!(
                r#"fn main() {{
                match {selection} {{
                    Ok(output) => {{
                        std::fs::write("last-output", output.include.parent().unwrap().to_str().unwrap()).unwrap();
                        println!("cargo::rustc-env=SERVICE_BINDINGS={{}}", output.include.display());
                    }}
                    Err(neocogi_idl::BuildError::Resolve(error)) => {{
                        assert!(error.to_string().contains("EIDL021"));
                        panic!("NC_IMPORTED_FAMILY_RESOLVE_REJECTION: {{error}}");
                    }}
                    Err(error) => panic!("wrong rejection phase: {{error}}"),
                }}
            }}"#,
            ),
        );
        let rival = if label == "shared" { imported_rival_source() } else { String::new() };
        let include_glob = if label == "shared" { "pub use crate::rival::*;" } else { "" };
        fixture.write(
            "provider/src/lib.rs",
            &format!(
                "extern crate self as binding_provider; {rival} pub mod service {{ {include_glob} include!(env!(\"SERVICE_BINDINGS\")); }} {}",
                imported_provider_source("service::IBase", false, true),
            ),
        );
        let output = fixture.cargo("test");
        assert!(output.status.success(), "{label}: {}", String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("test imported_call ... ok"), "{stdout}");
        assert!(stdout.contains("test consumer_metadata_keeps_its_owner ... ok"), "{stdout}");

        // Reuse these real Cargo builders after a successful publication. A
        // source-level imported-family collision must preserve every seeded file
        // and advertise no successful metadata for the rejected invocation.
        let output_dir = std::path::PathBuf::from(fs::read_to_string(fixture.consumer.directory.path().join("provider/last-output")).unwrap());
        crate::cargo::test_support::seed_unrelated(&output_dir);
        let before = crate::cargo::test_support::snapshot(&output_dir);
        fixture.write("schemas/proof/service.ncidl", &IMPORTING_MODULE.replace("struct Anchor", "struct IBaseAbi"));
        let rejected = fixture.cargo("check");
        let stderr = String::from_utf8_lossy(&rejected.stderr);
        assert!(!rejected.status.success(), "{label}: imported collision compiled");
        assert!(stderr.contains("NC_IMPORTED_FAMILY_RESOLVE_REJECTION"), "{label}: {stderr}");
        assert!(!stderr.contains("cargo::metadata="), "{label}: rejected build advertised graph ownership");
        assert_eq!(crate::cargo::test_support::snapshot(&output_dir), before);
        fixture.write("schemas/proof/service.ncidl", IMPORTING_MODULE);
    }
}
