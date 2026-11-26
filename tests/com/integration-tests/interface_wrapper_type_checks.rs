//! Compile the actual generated facade under the active compiler, without snapshots.

use std::{fs, path::Path, process::Command};

/// Tests semantic rejection, so stable/MSRV diagnostic source snippets may differ.
#[test]
fn interface_wrappers_cannot_replace_pointer_pair() {
    let owner = Path::new(env!("CARGO_MANIFEST_DIR"));
    let facade = owner.join("../../crates/com");
    let compiler = owner.join("../../crates/idl");
    let schema = facade.join("../../schemas/nc/tests/weak_com.ncidl");
    let boxed_schema = facade.join("../../schemas/nc/tests/exclusive_com.ncidl");
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(project.path().join("Cargo.toml"), format!(
        "[package]\nname='interface-wrapper-type-checks'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com-sdk={{path={facade:?}}}\n[build-dependencies]\nneocogi-idl={{path={compiler:?}}}\n"
    )).unwrap();
    fs::write(
        project.path().join("build.rs"),
        format!(
            r#"
        fn main() {{
            let source=std::fs::read_to_string({schema:?}).unwrap();
            let module=neocogi_idl::parse_module(std::path::Path::new("nc/tests/weak_com.ncidl"), &source).unwrap();
            let graph=neocogi_idl::resolve_modules(vec![module]).unwrap();
            let rust=neocogi_idl::generate_rust(&graph,"nc.tests.weak_com").unwrap();
            let out=std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
            std::fs::write(out.join("bindings.rs"),rust).unwrap();
            let mut source=std::fs::read_to_string({boxed_schema:?}).unwrap();
            source.push_str("interface IReceiverLoan {{ fn pick(&self, other: &IValue) -> Option<&IValue>; }}");
            let module=neocogi_idl::parse_module(std::path::Path::new("nc/tests/exclusive_com.ncidl"),&source).unwrap();
            let graph=neocogi_idl::resolve_modules(vec![module]).unwrap();
            let rust=neocogi_idl::generate_rust(&graph,"nc.tests.exclusive_com").unwrap();
            std::fs::write(out.join("boxed.rs"),rust).unwrap();
        }}
    "#
        ),
    )
    .unwrap();
    let mut cargo = Command::new(env!("CARGO"));
    let version = cargo.arg("--version").output().unwrap();
    assert!(version.status.success());
    println!("active wrapper type-check compiler: {}", String::from_utf8_lossy(&version.stdout).trim());

    let check = |body: &str| {
        fs::write(
            project.path().join("src/lib.rs"),
            format!(
                r#"
            #![allow(dead_code)]
            use neocogi_com_sdk::{{ComPtr,ComRef,ComMut,ComBox,ThreadSafe}};
            mod boxed {{ include!(concat!(env!("OUT_DIR"),"/boxed.rs")); }}
            mod abi {{ include!(concat!(env!("OUT_DIR"),"/bindings.rs")); {body} }}
        "#
            ),
        )
        .unwrap();
        Command::new(env!("CARGO"))
            .args(["check", "--offline", "--manifest-path"])
            .arg(project.path().join("Cargo.toml"))
            .env(
                "CARGO_TARGET_DIR",
                std::env::var_os("CARGO_TARGET_DIR").unwrap_or_else(|| facade.join("../../target").into_os_string()),
            )
            .output()
            .unwrap()
    };
    let positive = check(
        r#"
        fn calls(owner: &mut super::ComBox<IDerivedAbi>) {
            let _ = owner.value();
            let _ = owner.close();
            let mut loan = owner.as_com_mut();
            let _ = loan.as_com_ref().value();
            let _ = loan.close();
        }
        fn joined(owner: &mut super::ComBox<super::boxed::IContainerAbi>) {
            let mut child=owner.child_mut().unwrap();
            std::thread::scope(|scope| {
                let mut nested=child.as_com_mut();
                scope.spawn(move || nested.set(3)).join().unwrap().unwrap();
            });
            child.set(4).unwrap();
        }
        fn valid_swap(a: &mut super::ComPtr<IDerivedAbi>, b: &mut super::ComPtr<IDerivedAbi>) { std::mem::swap(a,b); }
        fn shared<T: ?Sized+Send+Sync>() {}
        fn reference_proof() { shared::<ISharedWrapper<super::ThreadSafe>>(); }
    "#,
    );
    assert!(positive.status.success(), "{}", String::from_utf8_lossy(&positive.stderr));
    for (label, body) in [
        (
            "owner/borrow",
            "fn swap(a:&mut super::ComBox<IDerivedAbi>,b:&mut super::ComMut<'_,IDerivedAbi>){std::mem::swap(&mut **a,&mut **b);}",
        ),
        (
            "independent lenders",
            "fn swap<'a,'b>(a:&mut super::ComMut<'a,IDerivedAbi>,b:&mut super::ComMut<'b,IDerivedAbi>){std::mem::swap(&mut **a,&mut **b);}",
        ),
        (
            "inherited/base",
            "fn swap(a:&mut super::ComBox<IDerivedAbi>,b:&mut super::ComBox<IValueAbi>){let x:&mut IValueWrapper=&mut **a;std::mem::swap(x,&mut **b);}",
        ),
        (
            "same-module inner storage",
            "fn swap(a:&mut IDerivedWrapper,b:&mut IDerivedWrapper){std::mem::swap(&mut a.__nc_pointer,&mut b.__nc_pointer);}",
        ),
    ] {
        let output = check(body);
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{label} replaced a borrowed capability");
        assert!(error.contains("error[E0277]") && error.contains("[()]"), "{label}: {error}");
    }
    for (label, body, diagnostic) in [
        (
            "write through shared loan",
            "fn write(owner: &super::ComBox<super::boxed::IValueAbi>) { let mut loan = owner.as_com_ref(); let _ = loan.set(7); }",
            "E0596",
        ),
        (
            "generic retain through shared loan",
            "fn retain(loan: super::ComRef<'_, IDerivedAbi>) { let _ = loan.try_add_ref(); }",
            "E0599",
        ),
        (
            "generic downgrade through shared loan",
            "fn observe(loan: super::ComRef<'_, IDerivedAbi>) { let _ = loan.downgrade(); }",
            "E0599",
        ),
        (
            "shared owner cannot lend exclusive state",
            "fn lend(owner: &mut super::ComPtr<IDerivedAbi>) { let _ = owner.as_com_mut(); }",
            "E0599",
        ),
        (
            "copy of mutable loan",
            "fn copy(loan: super::ComMut<'_, IDerivedAbi>) { let first = loan; let second = loan; let _ = (first,second); }",
            "E0382",
        ),
        (
            "a second mutable loan from the same lender",
            "fn lend(owner: &mut super::ComBox<IDerivedAbi>) { let a = owner.as_com_mut(); let b = owner.as_com_mut(); let _ = (a,b); }",
            "E0499",
        ),
        (
            "returned child outlives source move",
            "fn take(owner: super::ComBox<super::boxed::IContainerAbi>) { let child = owner.child().unwrap(); drop(owner); let _=child.value(); }",
            "E0505",
        ),
        (
            "mutable returned child freezes source",
            "fn read(owner: &mut super::ComBox<super::boxed::IContainerAbi>) { let child = owner.child_read().unwrap(); let _=owner.child(); let _=child.value(); }",
            "E0502",
        ),
        (
            "returned child cannot run in detached worker",
            "fn detach(owner: &super::ComBox<super::boxed::IContainerAbi>) { let child = owner.child().unwrap(); std::thread::spawn(move || child.value()); }",
            "E0521",
        ),
        (
            "send-only target cannot transfer a shared read",
            "fn transfer(owner: &super::ComBox<super::boxed::ITransferCellAbi, neocogi_com_sdk::ThreadSend>) { let loan = owner.as_com_ref(); std::thread::scope(|scope| { scope.spawn(move || loan.value()); }); }",
            "E0277",
        ),
        (
            "sync-only target cannot transfer its owner",
            "fn transfer(owner: super::ComBox<super::boxed::ILocalLockAbi, neocogi_com_sdk::ThreadSync>) { std::thread::spawn(move || owner.value()); }",
            "E0277",
        ),
    ] {
        let output = check(body);
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success() && error.contains(diagnostic), "{label}: {error}");
    }
    // Override just the body of one real generated provider method. Its
    // elided output lifetime must be tied to `self` in both negative cases.
    let method = |expression| {
        format!(
            "struct Provider(std::cell::RefCell<super::ComBox<super::boxed::IValueAbi, super::ThreadSafe>>,
                             super::ComBox<super::boxed::IValueAbi, super::ThreadSafe>);
             impl super::boxed::IReceiverLoan for Provider {{
                 fn pick(&self, other: super::ComRef<'_, super::boxed::IValueAbi, super::ThreadSafe>)
                     -> Option<super::ComRef<'_, super::boxed::IValueAbi, super::ThreadSafe>> {{
                     {expression}
                 }}
             }}"
        )
    };
    let output = check(&method("Some(self.1.as_com_ref())"));
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    for (expression, diagnostic) in [
        ("Some(self.0.borrow().as_com_ref())", "E0515"),
        ("Some(other)", "lifetime may not live long enough"),
    ] {
        let output = check(&method(expression));
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success() && error.contains(diagnostic), "{expression}: {error}");
    }
    let output = check(
        "fn escape(provider: &super::ComPtr<super::boxed::IGuardedCellAbi>) {
            let _ = super::boxed::IGuardedCellAbi::with_child_mut(provider.as_com_ref(), |loan| loan);
         }",
    );
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success() && error.contains("lifetime may not live long enough"), "{error}");

    let output = check("fn send<T:?Sized+Send>(){} fn sync<T:?Sized+Sync>(){} fn local(){send::<IDerivedWrapper>();sync::<IDerivedWrapper>();}");
    let errors = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() && errors.contains("error[E0277]") && errors.contains("ComInterfacePointer<IDerivedAbi>"),
        "{errors}"
    );
}
