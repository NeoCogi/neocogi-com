//! Compile provider admission with the active compiler and execute valid COM calls.

use std::{fs, path::Path, process::Command};

/// Shared minimums apply independently of constructor spelling and handle permissions.
#[test]
fn shared_provider_minimum_is_independent_of_class_mode() {
    let owner = Path::new(env!("CARGO_MANIFEST_DIR"));
    let facade = owner.join("../../crates/com");
    let compiler = owner.join("../../crates/idl");
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(project.path().join("Cargo.toml"), format!(
        "[package]\nname='provider-threading-admission'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={facade:?}}}\n[build-dependencies]\nneocogi-idl={{path={compiler:?}}}\n"
    )).unwrap();
    fs::write(
        project.path().join("contract.ncidl"),
        r#"
        #[version(1)] interface Local { fn value(&self) -> Result<u32>; }
        #[version(1)] #[threading(shared)] interface Shared { fn value(&self) -> Result<u32>; }
        #[version(1)] #[threading(shared)] interface Derived: Shared {}
        #[version(1)] #[threading(shared)] interface Combined: Local + Shared {}
    "#,
    )
    .unwrap();
    fs::write(
        project.path().join("build.rs"),
        r#"
        fn main() {
            let source = std::fs::read_to_string("contract.ncidl").unwrap();
            let module = neocogi_idl::parse_module(std::path::Path::new("proof/threading.ncidl"), &source).unwrap();
            let graph = neocogi_idl::resolve_modules(vec![module]).unwrap();
            let rust = neocogi_idl::generate_rust(&graph, "proof.threading").unwrap();
            std::fs::write(std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("bindings.rs"), rust).unwrap();
        }
    "#,
    )
    .unwrap();

    // One consumer and target are reused sequentially. Only source changes between
    // cases; positive binaries execute actual generated tables, not checked stubs.
    let invoke = |action: &str, body: &str| {
        fs::write(
            project.path().join("src/main.rs"),
            format!(
                r#"
            #![allow(dead_code, unused_imports)]
            use neocogi_com::{{com_object, ComInterface, ComObject, ComPtr, NcStatus, ThreadLocal, ThreadSafe, ThreadingModel}};
            use std::{{cell::Cell, marker::PhantomData, rc::Rc, sync::{{Arc, MutexGuard}}}};
            mod bindings {{ include!(concat!(env!("OUT_DIR"), "/bindings.rs")); }}
            use bindings::*;
            fn send<T: Send>() {{}}
            fn sync<T: Sync>() {{}}
            fn class_mode<T: ComObject>() -> u32 {{ T::Threading::ABI_THREADING_MODE.as_raw() }}
            fn handle_mode<I: ComInterface, M: ThreadingModel>(_: &ComPtr<I, M>) -> u32 {{ M::ABI_THREADING_MODE.as_raw() }}
            {body}
        "#
            ),
        )
        .unwrap();
        let output = Command::new(env!("CARGO"))
            .args([action, "--offline", "--manifest-path"])
            .arg(project.path().join("Cargo.toml"))
            .env(
                "CARGO_TARGET_DIR",
                std::env::var_os("CARGO_TARGET_DIR").unwrap_or_else(|| facade.join("../../target").into_os_string()),
            )
            .output()
            .unwrap();
        println!("provider case {action}: {}", String::from_utf8_lossy(&output.stdout));
        output
    };

    for (attribute, mode) in [("interfaces(Shared)", 0), ("Shared", 1), ("interfaces(Shared), thread_safe", 1)] {
        let body = format!(
            r#"
            #[com_object({attribute})]
            struct Provider {{ state: Arc<u32> }}
            impl Shared for Provider {{ fn value(&self) -> Result<u32, NcStatus> {{ Ok(*self.state) }} }}
            fn main() {{
                let owner = Provider::from_value(Provider {{ state: Arc::new(42) }});
                let pointer = owner.query::<SharedAbi>().unwrap();
                assert_eq!(pointer.value().unwrap(), 42);
                assert_eq!(class_mode::<Provider>(), {mode});
                assert_eq!(handle_mode(&pointer), {mode});
                assert_eq!(<SharedAbi as ComInterface>::Threading::ABI_THREADING_MODE.as_raw(), 1);
                println!("shared provider called; mode={mode}");
            }}
        "#
        );
        let output = invoke("run", &body);
        assert!(output.status.success(), "{attribute}: {}", String::from_utf8_lossy(&output.stderr));
        assert!(String::from_utf8_lossy(&output.stdout).contains(&format!("shared provider called; mode={mode}")));
    }

    let output = invoke(
        "run",
        r#"
        #[com_object(interfaces(Local))]
        struct Provider { state: Rc<u32> }
        impl Local for Provider { fn value(&self) -> Result<u32, NcStatus> { Ok(*self.state) } }
        fn main() {
            // Establish each independent marker fixture before using it negatively.
            send::<Cell<u32>>();
            sync::<MutexGuard<'static, u32>>();
            let pointer: ComPtr<LocalAbi, ThreadLocal> = Provider::from_value(Provider { state: Rc::new(42) }).query().unwrap();
            assert_eq!(pointer.value().unwrap(), 42);
            println!("local provider and marker premises passed");
        }
    "#,
    );
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("local provider and marker premises passed"));

    let output = invoke(
        "run",
        r#"
        #[com_object(interfaces(Derived, Combined), ownership = exclusive)]
        struct Provider<T: Send + Sync + 'static> { state: T }
        impl<T: Send + Sync + 'static> Local for Provider<T> { fn value(&self) -> Result<u32, NcStatus> { Ok(42) } }
        impl<T: Send + Sync + 'static> Shared for Provider<T> { fn value(&self) -> Result<u32, NcStatus> { Ok(42) } }
        impl<T: Send + Sync + 'static> Derived for Provider<T> {}
        impl<T: Send + Sync + 'static> Combined for Provider<T> {}
        #[com_object(interfaces(Shared))]
        struct Phantom<T: 'static> { marker: PhantomData<fn() -> T> }
        impl<T: 'static> Shared for Phantom<T> { fn value(&self) -> Result<u32, NcStatus> { Ok(7) } }
        fn main() {
            let owner = Provider::into_box(Provider { state: Arc::new(1u32) });
            assert_eq!(class_mode::<Provider<Arc<u32>>>(), 0);
            assert_eq!(Provider::<Arc<u32>>::COM_INTERFACE_COUNT, 5);
            assert_eq!(owner.query_ref::<DerivedAbi>().unwrap().value().unwrap(), 42);
            assert_eq!(owner.query_ref::<CombinedAbi>().unwrap().value().unwrap(), 42);
            assert_eq!(owner.query_ref::<SharedAbi>().unwrap().value().unwrap(), 42);
            let pointer: ComPtr<SharedAbi, ThreadLocal> = Phantom::<Rc<u32>>::from_value(Phantom { marker: PhantomData }).query().unwrap();
            assert_eq!(pointer.value().unwrap(), 7);
            println!("generic exclusive projections and phantom called");
        }
    "#,
    );
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("generic exclusive projections and phantom called"));

    // Default-list negatives isolate the provider minimum from the stronger
    // class-mode proof. Rc alone could not expose one accidentally missing bound.
    for (attribute, state, marker) in [
        ("interfaces(Shared)", "Rc<u32>", "Send"),
        ("Shared", "Rc<u32>", "Send"),
        ("interfaces(Shared), thread_safe", "Rc<u32>", "Send"),
        ("interfaces(Shared)", "Cell<u32>", "Sync"),
        ("interfaces(Shared)", "MutexGuard<'static, u32>", "Send"),
        ("interfaces(Shared), ownership = exclusive", "Rc<u32>", "Send"),
    ] {
        println!("reject shared provider: {attribute}, {state}, missing {marker}");
        let output = invoke(
            "check",
            &format!(
                r#"
            #[com_object({attribute})] struct Provider {{ state: {state} }}
            impl Shared for Provider {{ fn value(&self) -> Result<u32, NcStatus> {{ Ok(42) }} }}
            fn main() {{}}
        "#
            ),
        );
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{attribute} admitted shared {state}");
        assert!(
            error.contains("error[E0277]") && error.contains(marker) && error.contains("Shared"),
            "{attribute}/{state}: {error}"
        );
    }
    for (label, body) in [
        (
            "generic",
            r#"
            #[com_object(interfaces(Shared))] struct Provider<T: 'static> { state: Rc<u32>, marker: PhantomData<fn() -> T> }
            impl<T: 'static> Shared for Provider<T> { fn value(&self) -> Result<u32, NcStatus> { Ok(42) } }
            fn main() {}
        "#,
        ),
        (
            "required sibling",
            r#"
            #[com_object(interfaces(Combined))] struct Provider { state: Rc<u32> }
            impl Local for Provider { fn value(&self) -> Result<u32, NcStatus> { Ok(42) } }
            impl Shared for Provider { fn value(&self) -> Result<u32, NcStatus> { Ok(42) } }
            impl Combined for Provider {}
            fn main() {}
        "#,
        ),
        (
            "inherited prefix",
            r#"
            #[com_object(interfaces(Derived))] struct Provider { state: Rc<u32> }
            impl Shared for Provider { fn value(&self) -> Result<u32, NcStatus> { Ok(42) } }
            impl Derived for Provider {}
            fn main() {}
        "#,
        ),
        (
            "trait without object macro",
            r#"
            struct Provider { state: Rc<u32> }
            impl Shared for Provider { fn value(&self) -> Result<u32, NcStatus> { Ok(42) } }
            fn main() {}
        "#,
        ),
        (
            "local interface with stronger class",
            r#"
            #[com_object(interfaces(Local), thread_safe)] struct Provider { state: Rc<u32> }
            impl Local for Provider { fn value(&self) -> Result<u32, NcStatus> { Ok(42) } }
            fn main() { let _ = Provider::from_value(Provider { state: Rc::new(1) }); }
        "#,
        ),
    ] {
        println!("reject provider path: {label}");
        let output = invoke("check", body);
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{label} admitted Rc");
        assert!(error.contains("error[E0277]") && error.contains("Send"), "{label}: {error}");
    }
}
