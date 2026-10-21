//! Direct scalar Rust must remain usable by a real allocation-free SDK consumer.
use std::{fs, path::Path, process::Command};
use neocogi_idl::{parse_module, resolve_modules};

/// Compiles every representation, flag operation, dense helper and stack NCB codec without facade features.
#[test]
fn scalar_bindings_compile_and_execute_without_default_features_or_alloc() {
    let module = parse_module(
        Path::new("proof/scalars.ncidl"),
        include_str!("../../test-contracts/idl/nc/tests/scalar_values.ncidl"),
    )
    .unwrap();
    let graph = resolve_modules(vec![module]).unwrap();
    let generated = neocogi_idl::generate_rust(&graph, "proof.scalars").unwrap();
    assert!(!generated.contains("com_int_enum"));
    assert!(!generated.contains("com_bit_enum"));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(project.path().join("Cargo.toml"), format!(
        "[package]\nname='scalar-no-alloc-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?},default-features=false}}\n",
        root.join("crates/com/api"),
    )).unwrap();
    fs::write(
        project.path().join("src/lib.rs"),
        format!(
            r#"
        #![no_std]
        {generated}
        #[cfg(test)] mod tests {{
            use super::*;
            use neocogi_com::{{binary, encode_inline}};
            #[test] fn primitive_storage_and_checked_values_need_no_alloc() {{
                assert_eq!(encode_inline(&Word::Maximum), Ok(u32::MAX));
                assert_eq!(encode_inline(&SignedWord::Minimum), Ok(i32::MIN));
                assert_eq!(encode_inline(&Byte::from_raw(42)), Err(neocogi_com::NC_E_INVALID_TAG));
                assert_eq!(encode_inline(&(TinyFlags::HIGH | TinyFlags::LOW)), Ok(129));
                assert_eq!(Order::FirstItem.canonical_name(), Some("first_item"));
                let mut bytes = [0; 8];
                let count = binary::to_slice(&WideFlags::from_raw(u32::MAX), &mut bytes).unwrap();
                assert_eq!(count, 8);
                assert_eq!(binary::from_slice::<WideFlags>(&bytes).unwrap().raw(), u32::MAX);
            }}
        }}
    "#
        ),
    )
    .unwrap();
    let result = Command::new("cargo")
        .args(["test", "--offline", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            std::env::var_os("CARGO_TARGET_DIR").unwrap_or_else(|| root.join("target").into_os_string()),
        )
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
