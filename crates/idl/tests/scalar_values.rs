//! Direct scalar Rust must remain usable by a real allocation-free SDK consumer.
use std::{fs, path::Path, process::Command};
use neocogi_idl::{parse_module, resolve_modules};

/// Compiles every representation, flag operation, dense helper and stack NCB codec without facade features.
#[test]
fn scalar_bindings_compile_and_execute_without_default_features_or_alloc() {
    // Deliberately unordered declarations exercise adjacent runs around zero,
    // sparse holes and integer endpoints without changing authored member order.
    let source = format!(
        "{}\n{}",
        include_str!("../../test-contracts/idl/nc/tests/scalar_values.ncidl"),
        r#"
        #[version(1)] enum SignedRuns: i8 {
            Minimum = -128, PositiveLast = 6, NegativeLast = -1, Zero = 0,
            NegativeFirst = -3, PositiveFirst = 4, PositiveMiddle = 5,
            NegativeMiddle = -2, Maximum = 127,
        }
        #[version(1)] enum UnsignedRuns: u8 {
            Maximum = 255, Middle = 3, Zero = 0, First = 2, Last = 4,
        }
        #[version(1)] bitenum OverlapFlags: u8 { NONE = 0, LOW = 1, MID = 2, BOTH = 3, HIGH = 128 }
        #[version(1)] bitenum EmptyFlags: u8 { NONE = 0 }
        #[version(1)] bitenum FullFlags: u8 { ALL = 255 }
        "#,
    );
    let module = parse_module(Path::new("proof/scalars.ncidl"), &source).unwrap();
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
                for raw in i8::MIN..=i8::MAX {{
                    let expected = [-128, -3, -2, -1, 0, 4, 5, 6, 127].contains(&raw);
                    let value = SignedRuns::from_raw(raw);
                    assert_eq!(value.is_known(), expected, "signed domain {{raw}}");
                    assert_eq!(encode_inline(&value).is_ok(), expected);
                }}
                for raw in u8::MIN..=u8::MAX {{
                    let expected = [0, 2, 3, 4, 255].contains(&raw);
                    let value = UnsignedRuns::from_raw(raw);
                    assert_eq!(value.is_known(), expected, "unsigned domain {{raw}}");
                    assert_eq!(encode_inline(&value).is_ok(), expected);
                    assert_eq!(OverlapFlags::from_raw(raw).is_valid(), raw & !131 == 0);
                    assert_eq!(EmptyFlags::from_raw(raw).is_valid(), raw == 0);
                    assert!(FullFlags::from_raw(raw).is_valid());
                }}
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
    // This checkout integration test owns a real downstream quality check.
    // Linting the compiler cannot inspect Rust it emits inside string fixtures.
    let result = Command::new("cargo")
        .args(["clippy", "--offline", "--all-targets", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .args(["--", "-D", "warnings"])
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
