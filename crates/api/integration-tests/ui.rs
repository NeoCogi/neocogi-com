#[test]
fn ui_errors_are_stable() {
    let t = trybuild::TestCases::new();
    t.pass("integration-tests/ui/class_generic_owned.rs");
    t.compile_fail("integration-tests/ui/object_ownership_constructors.rs");
    t.compile_fail("integration-tests/ui/object_ownership_invalid.rs");
    t.compile_fail("integration-tests/ui/object_no_default_constructor.rs");
    t.pass("integration-tests/ui/object_ordinary_interface.rs");
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
    t.compile_fail("integration-tests/ui/event_query_escape.rs");
    t.compile_fail("integration-tests/ui/query_ref_thread_affinity.rs");
    t.compile_fail("integration-tests/ui/local_guard_thread_affinity.rs");
    t.compile_fail("integration-tests/ui/weak_local_thread_affinity.rs");
    t.compile_fail("integration-tests/ui/weak_local_construction_proof.rs");
    t.compile_fail("integration-tests/ui/enum_comptr_payload.rs");
    t.compile_fail("integration-tests/ui/enum_explicit_tag.rs");
    t.compile_fail("integration-tests/ui/enum_explicit_discriminant.rs");
    t.compile_fail("integration-tests/ui/value_explicit_tag.rs");
    t.compile_fail("integration-tests/ui/enum_tag_range.rs");
    t.compile_fail("integration-tests/ui/enum_generic.rs");
    t.compile_fail("integration-tests/ui/enum_pointer_payload.rs");
    t.pass("integration-tests/ui/enum_case_folded_variant.rs");
    t.compile_fail("integration-tests/ui/enum_string_payload.rs");
    t.compile_fail("integration-tests/ui/enum_where_clause.rs");
    t.pass("integration-tests/ui/enum_public_documentation.rs");
    t.compile_fail("integration-tests/ui/format_method_signature_requires_unsafe.rs");
    t.compile_fail("integration-tests/ui/interface_async_method.rs");
    t.compile_fail("integration-tests/ui/interface_consuming_unbounded_output.rs");
    t.compile_fail("integration-tests/ui/interface_copy_from_buffer_bad_element.rs");
    t.compile_fail("integration-tests/ui/interface_copy_from_buffer_missing_lifetime.rs");
    t.compile_fail("integration-tests/ui/interface_copy_from_buffer_static_lifetime.rs");
    t.compile_fail("integration-tests/ui/interface_copy_to_buffer_bad_element.rs");
    t.compile_fail("integration-tests/ui/interface_copy_to_buffer_missing_element.rs");
    t.compile_fail("integration-tests/ui/interface_copy_to_buffer_static_lifetime.rs");
    t.compile_fail("integration-tests/ui/interface_comref_static_lifetime.rs");
    t.compile_fail("integration-tests/ui/interface_value_ref_static_lifetime.rs");
    t.compile_fail("integration-tests/ui/interface_value_ref_named_lifetime.rs");
    t.compile_fail("integration-tests/ui/interface_value_ref_shape.rs");
    t.compile_fail("integration-tests/ui/interface_value_ref_escape.rs");
    t.compile_fail("integration-tests/ui/interface_default_method.rs");
    t.compile_fail("integration-tests/ui/interface_explicit_guid_name_mismatch.rs");
    t.compile_fail("integration-tests/ui/interface_empty_qualified_name.rs");
    t.compile_fail("integration-tests/ui/interface_generic_method.rs");
    t.compile_fail("integration-tests/ui/interface_generic_trait.rs");
    t.pass("integration-tests/ui/interface_generated_name_collision.rs");
    t.compile_fail("integration-tests/ui/interface_lifecycle_method_collision.rs");
    t.pass("integration-tests/ui/interface_noncolliding_generated_names.rs");
    t.pass("integration-tests/ui/interface_mut_receiver.rs");
    t.compile_fail("integration-tests/ui/interface_manual_safe_impl.rs");
    t.compile_fail("integration-tests/ui/interface_malformed_qualified_names.rs");
    t.compile_fail("integration-tests/ui/interface_missing_assigned_signature.rs");
    t.compile_fail("integration-tests/ui/interface_nested_vec_arg.rs");
    t.pass("integration-tests/ui/interface_canonical_client.rs");
    t.compile_fail("integration-tests/ui/interface_non_ascii_qualified_name.rs");
    t.compile_fail("integration-tests/ui/interface_overlong_qualified_name.rs");
    t.compile_fail("integration-tests/ui/interface_pattern_arg.rs");
    t.compile_fail("integration-tests/ui/interface_raw_pointer_arg.rs");
    t.compile_fail("integration-tests/ui/interface_reference_arg.rs");
    t.compile_fail("integration-tests/ui/interface_string_arg.rs");
    t.compile_fail("integration-tests/ui/interface_string_return.rs");
    t.compile_fail("integration-tests/ui/interface_vec_arg.rs");
    t.compile_fail("integration-tests/ui/interface_comptr_arg.rs");
    t.compile_fail("integration-tests/ui/interface_comref_return.rs");
    t.compile_fail("integration-tests/ui/interface_supertrait.rs");
    t.compile_fail("integration-tests/ui/interface_client_escape.rs");
    t.compile_fail("integration-tests/ui/interface_prefix_collision.rs");
    t.compile_fail("integration-tests/ui/interface_unsafe_trait.rs");
    t.compile_fail("integration-tests/ui/interface_unsupported_items.rs");
    t.compile_fail("integration-tests/ui/interface_wrong_error.rs");
    t.compile_fail("integration-tests/ui/interface_wrong_return.rs");
    t.compile_fail("integration-tests/ui/struct_comref_field.rs");
    t.compile_fail("integration-tests/ui/struct_comptr_field.rs");
    t.compile_fail("integration-tests/ui/struct_generic.rs");
    t.compile_fail("integration-tests/ui/struct_pointer_field.rs");
    t.compile_fail("integration-tests/ui/struct_raw_repr.rs");
    t.compile_fail("integration-tests/ui/struct_reference_field.rs");
    t.compile_fail("integration-tests/ui/struct_string_field.rs");
    t.compile_fail("integration-tests/ui/struct_vec_field.rs");
    t.compile_fail("integration-tests/ui/persistence_retired_policies.rs");
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
