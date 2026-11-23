//! Compile the actual generated facade under the active compiler, without snapshots.

use std::{fs, path::Path, process::Command};

/// Tests semantic rejection, so stable/MSRV diagnostic source snippets may differ.
#[test]
fn interface_wrappers_cannot_replace_pointer_pair() {
    let owner = Path::new(env!("CARGO_MANIFEST_DIR"));
    let facade = owner.join("../../crates/com");
    let compiler = owner.join("../../crates/idl");
    let schema = facade.join("../../schemas/nc/tests/weak_com.ncidl");
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
            std::fs::write(std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("bindings.rs"),rust).unwrap();
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
            use neocogi_com_sdk::{{ComPtr,ComRef,ComMut,ComBox,WeakComBox,ThreadSafe}};
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
        fn calls(owner: &mut super::ComPtr<IDerivedAbi>) {
            let _ = owner.value();
            let _ = owner.close();
            let mut loan = owner.as_com_mut();
            let _ = loan.as_com_ref().value();
            let _ = loan.close();
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
            "fn swap(a:&mut super::ComPtr<IDerivedAbi>,b:&mut super::ComMut<'_,IDerivedAbi>){std::mem::swap(&mut **a,&mut **b);}",
        ),
        (
            "independent lenders",
            "fn swap<'a,'b>(a:&mut super::ComMut<'a,IDerivedAbi>,b:&mut super::ComMut<'b,IDerivedAbi>){std::mem::swap(&mut **a,&mut **b);}",
        ),
        (
            "exclusive/observer",
            "fn swap(a:&mut super::ComBox<IDerivedAbi>,b:&mut super::WeakComBox<IDerivedAbi>){std::mem::swap(&mut **a,&mut **b);}",
        ),
        (
            "inherited/base",
            "fn swap(a:&mut super::ComPtr<IDerivedAbi>,b:&mut super::ComPtr<IValueAbi>){let x:&mut IValueWrapper=&mut **a;std::mem::swap(x,&mut **b);}",
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
            "inherited write through shared loan",
            "fn write(owner: &super::ComPtr<IDerivedAbi>) { let mut loan = owner.as_com_ref(); let _ = loan.close(); }",
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
            "weak-to-live conversion",
            "fn lend(weak: &super::WeakComBox<IDerivedAbi>) { let _ = weak.as_com_ref(); }",
            "E0599",
        ),
        (
            "copy of mutable loan",
            "fn copy(loan: super::ComMut<'_, IDerivedAbi>) { let first = loan; let second = loan; let _ = (first,second); }",
            "E0382",
        ),
        (
            "a second mutable loan from the same lender",
            "fn lend(owner: &mut super::ComPtr<IDerivedAbi>) { let a = owner.as_com_mut(); let b = owner.as_com_mut(); let _ = (a,b); }",
            "E0499",
        ),
    ] {
        let output = check(body);
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success() && error.contains(diagnostic), "{label}: {error}");
    }
    let output = check("fn send<T:?Sized+Send>(){} fn sync<T:?Sized+Sync>(){} fn local(){send::<IDerivedWrapper>();sync::<IDerivedWrapper>();}");
    let errors = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() && errors.contains("error[E0277]") && errors.contains("ComInterfacePointer<IDerivedAbi>"),
        "{errors}"
    );
}
