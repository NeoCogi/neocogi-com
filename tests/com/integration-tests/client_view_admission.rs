//! Compile the actual generated facade under the active compiler, without snapshots.

use std::{fs, path::Path, process::Command};

/// Tests semantic rejection, so stable/MSRV diagnostic source snippets may differ.
#[test]
fn client_views_cannot_replace_capability_storage() {
    let owner = Path::new(env!("CARGO_MANIFEST_DIR"));
    let facade = owner.join("../../crates/com");
    let compiler = owner.join("../../crates/idl");
    let schema = facade.join("../../schemas/nc/tests/weak_com.ncidl");
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(project.path().join("Cargo.toml"), format!(
        "[package]\nname='client-view-admission'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={facade:?}}}\n[build-dependencies]\nneocogi-idl={{path={compiler:?}}}\n"
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
    println!("active admission compiler: {}", String::from_utf8_lossy(&version.stdout).trim());

    let check = |body: &str| {
        fs::write(
            project.path().join("src/lib.rs"),
            format!(
                r#"
            #![allow(dead_code)]
            use neocogi_com::{{ComPtr,ComRef,ComBox,WeakComBox,ThreadSafe}};
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
        fn calls(owner: &mut super::ComPtr<IDerivedAbi>) { let _=owner.value(); let _=owner.close(); }
        fn valid_swap(a: &mut super::ComPtr<IDerivedAbi>, b: &mut super::ComPtr<IDerivedAbi>) { std::mem::swap(a,b); }
        fn shared<T: ?Sized+Send+Sync>() {}
        fn reference_proof() { shared::<ISharedClient<super::ThreadSafe>>(); }
    "#,
    );
    assert!(positive.status.success(), "{}", String::from_utf8_lossy(&positive.stderr));
    for (label, body) in [
        (
            "owner/borrow",
            "fn swap(a:&mut super::ComPtr<IDerivedAbi>,b:&mut super::ComRef<'_,IDerivedAbi>){std::mem::swap(&mut **a,&mut **b);}",
        ),
        (
            "independent lenders",
            "fn swap<'a,'b>(a:&mut super::ComRef<'a,IDerivedAbi>,b:&mut super::ComRef<'b,IDerivedAbi>){std::mem::swap(&mut **a,&mut **b);}",
        ),
        (
            "exclusive/observer",
            "fn swap(a:&mut super::ComBox<IDerivedAbi>,b:&mut super::WeakComBox<IDerivedAbi>){std::mem::swap(&mut **a,&mut **b);}",
        ),
        (
            "inherited/base",
            "fn swap(a:&mut super::ComPtr<IDerivedAbi>,b:&mut super::ComPtr<IValueAbi>){let x:&mut IValueClient=&mut **a;std::mem::swap(x,&mut **b);}",
        ),
        (
            "same-module inner storage",
            "fn swap(a:&mut IDerivedClient,b:&mut IDerivedClient){std::mem::swap(&mut a.__nc_pointer,&mut b.__nc_pointer);}",
        ),
    ] {
        let output = check(body);
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{label} replaced a borrowed capability");
        assert!(error.contains("error[E0277]") && error.contains("[()]"), "{label}: {error}");
    }
    let output = check("fn send<T:?Sized+Send>(){} fn sync<T:?Sized+Sync>(){} fn local(){send::<IDerivedClient>();sync::<IDerivedClient>();}");
    let errors = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() && errors.contains("error[E0277]") && errors.contains("NcClientPointer<IDerivedAbi>"),
        "{errors}"
    );
}
