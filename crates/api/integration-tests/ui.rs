#[test]
fn ui_errors_are_stable() {
    let t = trybuild::TestCases::new();
    t.pass("integration-tests/ui/class_generic_owned.rs");
    t.compile_fail("integration-tests/ui/object_ownership_constructors.rs");
    t.compile_fail("integration-tests/ui/object_ownership_invalid.rs");
    t.compile_fail("integration-tests/ui/object_no_default_constructor.rs");
    t.pass("integration-tests/ui/object_ordinary_interface.rs");
    t.pass("integration-tests/ui/idl_metadata_static.rs");
    t.compile_fail("integration-tests/ui/idl_metadata_nonstatic_name.rs");
    t.compile_fail("integration-tests/ui/idl_metadata_nonstatic_signature.rs");
    t.compile_fail("integration-tests/ui/class_generic_borrowed.rs");
    t.compile_fail("integration-tests/ui/class_lifetime.rs");

    t.compile_fail("integration-tests/ui/class_explicit_iunknown.rs");
    t.compile_fail("integration-tests/ui/class_duplicate_interface.rs");
    t.compile_fail("integration-tests/ui/class_thread_safe_requires_send_sync.rs");
    t.compile_fail("integration-tests/ui/object_generic_threading.rs");
    t.compile_fail("integration-tests/ui/comptr_thread_local_send.rs");
    t.compile_fail("integration-tests/ui/local_guard_owner_lifetime.rs");
    t.compile_fail("integration-tests/ui/weak_box_borrow_escape.rs");
    t.compile_fail("integration-tests/ui/box_query_ref_escape.rs");
    t.compile_fail("integration-tests/ui/query_ref_thread_affinity.rs");
    t.compile_fail("integration-tests/ui/local_guard_thread_affinity.rs");
    t.compile_fail("integration-tests/ui/weak_local_thread_affinity.rs");
    t.compile_fail("integration-tests/ui/weak_local_construction_proof.rs");
    t.pass("integration-tests/ui/enum_case_folded_variant.rs");
    t.pass("integration-tests/ui/enum_public_documentation.rs");
    t.compile_fail("integration-tests/ui/format_method_signature_requires_unsafe.rs");
    t.compile_fail("integration-tests/ui/interface_value_ref_escape.rs");
    t.compile_fail("integration-tests/ui/value_vec_source_escape.rs");
    t.pass("integration-tests/ui/interface_generated_name_collision.rs");
    t.pass("integration-tests/ui/interface_noncolliding_generated_names.rs");
    t.pass("integration-tests/ui/interface_mut_receiver.rs");
    t.compile_fail("integration-tests/ui/interface_manual_safe_impl.rs");
    t.pass("integration-tests/ui/interface_canonical_client.rs");
    t.compile_fail("integration-tests/ui/interface_client_escape.rs");
    t.compile_fail("integration-tests/ui/persistence_retired_policies.rs");
    t.compile_fail("integration-tests/ui/persistence_packed_record.rs");
}

/// Construction accepts complete values; none of the former initializer APIs exist.
#[test]
fn object_construction_requires_initialized_values() {
    trybuild::TestCases::new().compile_fail("integration-tests/ui/object_no_cyclic_constructor.rs");
}

/// Missing implementation remains a compiler error for independently generated
/// bindings. Check the semantic rejection rather than compiler-version-specific
/// diagnostic expansions, so this coverage runs on nightly and stable alike.
#[test]
fn generated_objects_require_their_interface_implementation() {
    use std::{fs, path::Path, process::Command};
    let owner = Path::new(env!("CARGO_MANIFEST_DIR"));
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(project.path().join("Cargo.toml"), format!(
        "[package]\nname='missing-generated-impl'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n[build-dependencies]\nneocogi-idl={{path={:?}}}\n",
        owner, owner.join("../idl"),
    )).unwrap();
    fs::write(
        project.path().join("build.rs"),
        format!(
            r#"
        fn main() {{
            for module in neocogi_idl::cargo::SharedSources::new({:?})
                .module("nc.tests.ui_class_missing_impl", "missing_generated_impl::abi")
                .build().unwrap() {{
                println!("cargo::rustc-env=MISSING_IMPL_BINDINGS={{}}", module.include.display());
            }}
        }}
    "#,
            owner.join("../schemas")
        ),
    )
    .unwrap();
    fs::write(
        project.path().join("src/lib.rs"),
        r#"
        extern crate self as missing_generated_impl;
        pub mod abi { include!(env!("MISSING_IMPL_BINDINGS")); }
        #[neocogi_com::com_object(abi::IMissingImpl)]
        pub struct MissingImpl;
    "#,
    )
    .unwrap();
    let output = Command::new("cargo")
        .args(["check", "--offline", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            std::env::var_os("CARGO_TARGET_DIR").unwrap_or_else(|| owner.join("../../../target").into_os_string()),
        )
        .output()
        .unwrap();
    let errors = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "an object without its generated interface implementation compiled");
    assert!(
        errors.contains("IMissingImpl") && (errors.contains("error[E0277]") || errors.contains("error[E0599]")),
        "{errors}"
    );
}
