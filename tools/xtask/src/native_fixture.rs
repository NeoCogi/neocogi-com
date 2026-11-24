//! Execution evidence shared by the workspace and standalone native-C gates.

/// Required independent C checks for the host, callable slots, and array wires.
pub const NATIVE_C_CASES: &[&str] = &[
    "callable_slots::native_c_consumes_owned_optional_and_scoped_slots",
    "callable_slots::generated_callers_validate_native_c_result_protocols",
    "fixed_arrays::native_c_calls_portable_fixed_array_wires",
    "host_abi::plain_c_forwards_host_access_to_the_typed_bootstrap",
];

/// Checks a named success after the caller has verified the child exited successfully.
pub fn require_named_execution(output: &str, required: &str) -> Result<(), String> {
    let passed = format!("test {required} ... ok");
    if output.lines().any(|line| line.trim() == passed) {
        Ok(())
    } else {
        Err(format!("required test did not execute successfully: {required}"))
    }
}

/// Requires the native cases in a single complete fixture run.
pub fn require_native_c_execution(output: &str) -> Result<(), String> {
    for case in NATIVE_C_CASES {
        require_named_execution(output, case)?;
    }
    Ok(())
}
