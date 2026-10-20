use std::path::PathBuf;
use std::process::Command;

#[test]
fn generated_abi_is_callable_from_native_c() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration-tests/fixtures/native-c-abi/Cargo.toml");
    let target_dir = std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../target"));
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let output = Command::new(cargo)
        .arg("test")
        .arg("--offline")
        .arg("--locked")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--target-dir")
        .arg(&target_dir)
        .output()
        .expect("failed to run cargo test for native C ABI fixture");

    assert!(
        output.status.success(),
        "native C ABI fixture failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
