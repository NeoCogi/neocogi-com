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

    if !rustc_is_nightly() {
        t.compile_fail("integration-tests/ui/class_missing_impl.rs");
    }

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

fn rustc_is_nightly() -> bool {
    if std::env::var("RUSTUP_TOOLCHAIN").is_ok_and(|toolchain| toolchain.contains("nightly")) {
        return true;
    }

    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let Ok(output) = std::process::Command::new(rustc).arg("--version").output() else {
        return false;
    };
    String::from_utf8_lossy(&output.stdout).contains("nightly")
}
