#[test]
fn ui_errors_are_stable() {
    let t = trybuild::TestCases::new();
    t.pass("integration-tests/ui/class_generic_owned.rs");
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
    t.compile_fail("integration-tests/ui/local_guard_owner_lifetime.rs");
    t.compile_fail("integration-tests/ui/box_query_ref_escape.rs");
    t.compile_fail("integration-tests/ui/query_ref_thread_affinity.rs");
    t.compile_fail("integration-tests/ui/weak_local_thread_affinity.rs");
    t.compile_fail("integration-tests/ui/weak_local_construction_proof.rs");
    t.pass("integration-tests/ui/enum_case_folded_variant.rs");
    t.pass("integration-tests/ui/enum_public_documentation.rs");
    t.compile_fail("integration-tests/ui/format_method_signature_requires_unsafe.rs");
    t.compile_fail("integration-tests/ui/value_vec_source_escape.rs");
    t.pass("integration-tests/ui/interface_generated_name_collision.rs");
    t.pass("integration-tests/ui/interface_noncolliding_generated_names.rs");
    t.pass("integration-tests/ui/interface_mut_receiver.rs");
    t.compile_fail("integration-tests/ui/interface_manual_safe_impl.rs");
    t.pass("integration-tests/ui/interface_inherited_lookup.rs");
    t.compile_fail("integration-tests/ui/interface_wrapper_cannot_outlive_owner.rs");
    t.compile_fail("integration-tests/ui/persistence_retired_policies.rs");
    t.compile_fail("integration-tests/ui/persistence_packed_record.rs");
}

/// Rust owns these diagnostics and changes their rendering between compilers.
/// Require the intended errors at the invalid expressions, rather than
/// snapshotting excerpts from std or lifetime annotations. Compilers may report
/// each unsatisfied Send/Sync obligation separately at the same expression;
/// every reported error must still match the expected code and source location.
#[test]
fn rust_ownership_rejections_match_the_invalid_expressions() {
    use std::{fs, path::Path, process::Command};
    let owner = Path::new(env!("CARGO_MANIFEST_DIR"));
    let project = tempfile::tempdir().unwrap();
    // A single independent package reuses compiled dependencies across the cases.
    let cases = [
        (
            "object_ownership_constructors",
            [("E0599", "Service::into_box"), ("E0599", "Widget::from_value")].as_slice(),
        ),
        ("weak_box_borrow_escape", [("lifetime may not live long enough", "weak.with_mut")].as_slice()),
        (
            "interface_value_ref_escape",
            [("lifetime may not live long enough", "self.saved = Some(value)")].as_slice(),
        ),
        ("class_thread_safe_requires_send_sync", [("E0277", "#[com_object(thread_safe,")].as_slice()),
        ("object_generic_threading", [("E0277", "Generic::from_value")].as_slice()),
        ("comptr_thread_local_send", [("E0277", "std::thread::spawn")].as_slice()),
        ("local_guard_thread_affinity", [("E0277", "scope.spawn")].as_slice()),
    ];
    let mut manifest = format!(
        "[package]\nname='ownership-rejections'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
        owner.join("../../crates/com"),
    );
    for (name, _) in &cases {
        manifest.push_str(&format!(
            "[[bin]]\nname={name:?}\npath={:?}\n",
            owner.join(format!("integration-tests/ui/{name}.rs"))
        ));
    }
    fs::write(project.path().join("Cargo.toml"), manifest).unwrap();
    for (name, expected) in cases {
        let fixture = owner.join(format!("integration-tests/ui/{name}.rs"));
        let source = fs::read_to_string(&fixture).unwrap();
        let output = Command::new("cargo")
            .current_dir(project.path())
            .args(["check", "--offline", "--message-format=json", "--bin", name])
            // The exported include belongs to this test owner, not this new package.
            .env("NC_COM_TEST_CONTRACTS", env!("NC_COM_TEST_CONTRACTS"))
            .env(
                "CARGO_TARGET_DIR",
                std::env::var_os("CARGO_TARGET_DIR").unwrap_or_else(|| owner.join("../../target").into_os_string()),
            )
            .output()
            .unwrap();
        let stdout = String::from_utf8(output.stdout).unwrap();
        let errors: Vec<serde_json::Value> = stdout
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|record| record["reason"] == "compiler-message" && record["message"]["level"] == "error")
            .map(|record| record["message"].clone())
            .collect();
        let diagnostics = format!("{name}:\n{stdout}\n{}", String::from_utf8_lossy(&output.stderr));
        assert!(!output.status.success(), "invalid ownership compiled: {diagnostics}");
        let matches = |error: &serde_json::Value, reason: &str, expression: &str| {
            let line = source.lines().position(|line| line.contains(expression)).unwrap() as u64 + 1;
            // E0277 covers all trait bounds. These four cases specifically
            // require a Send/Sync rejection, not another trait error at the call.
            let threading = reason != "E0277"
                || ["cannot be sent between threads safely", "cannot be shared between threads safely"]
                    .iter()
                    .any(|reason| error["message"].as_str().unwrap().ends_with(reason));
            // Coded errors use their stable identifier; uncoded lifetime
            // errors use only the primary reason, without rendered spans.
            (error["code"]["code"] == reason || (error["code"].is_null() && error["message"] == reason))
                && threading
                && error["spans"].as_array().unwrap().iter().any(|span| {
                    span["is_primary"] == true
                        && Path::new(span["file_name"].as_str().unwrap()).file_name() == fixture.file_name()
                        && span["line_start"].as_u64().unwrap() <= line
                        && span["line_end"].as_u64().unwrap() >= line
                })
        };
        if !expected.iter().all(|(reason, _)| *reason == "E0277") {
            assert_eq!(errors.len(), expected.len(), "unexpected compiler errors: {diagnostics}");
        }
        assert!(
            errors
                .iter()
                .all(|error| expected.iter().any(|(reason, expression)| matches(error, reason, expression))),
            "unrelated compiler errors: {diagnostics}"
        );
        for (reason, expression) in expected {
            assert!(
                errors.iter().any(|error| matches(error, reason, expression)),
                "missing {reason} at {expression}: {diagnostics}"
            );
        }
    }
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
        owner.join("../../crates/com"), owner.join("../../crates/idl"),
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
            owner.join("../../schemas")
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
            std::env::var_os("CARGO_TARGET_DIR").unwrap_or_else(|| owner.join("../../target").into_os_string()),
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
