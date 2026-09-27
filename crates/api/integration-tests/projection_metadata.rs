//! Release-link regression for the descriptor graph previously retained by all objects.
use std::{path::Path, process::Command};

/// Builds and runs one inspection mode, returning its actual stripped executable.
fn linked_fixture(manifest: &Path, target: &Path, inspect: bool) -> Vec<u8> {
    let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command
        .args(["build", "--offline", "--locked", "--release", "--manifest-path"])
        .arg(manifest)
        .arg("--target-dir")
        .arg(target);
    if inspect {
        command.args(["--features", "inspect"]);
    }
    let built = command.output().expect("start fixture build");
    assert!(built.status.success(), "fixture build failed: {}", String::from_utf8_lossy(&built.stderr));
    let executable = target
        .join("release")
        .join(format!("projection-metadata-fixture{}", std::env::consts::EXE_SUFFIX));
    let executed = Command::new(&executable).output().expect("run linked fixture");
    assert!(executed.status.success(), "fixture failed: {}", String::from_utf8_lossy(&executed.stderr));
    let stdout = String::from_utf8(executed.stdout).unwrap();
    assert!(stdout.starts_with("value=42\n"));
    if inspect {
        // All four authored interfaces survive explicit inspection; IBase occurs once.
        assert_eq!(stdout.matches("test.projection.MetadataRetentionSentinel").count(), 4);
        assert_eq!(stdout.matches("metadata_retention_sentinel_method").count(), 1);
    } else {
        assert_eq!(stdout, "value=42\n");
    }
    std::fs::read(executable).expect("read stripped fixture")
}

/// Real linker reachability, rather than source spelling, governs retained schemas.
#[test]
fn ordinary_objects_discard_schemas_but_explicit_inspection_retains_them() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = root.join("integration-tests/fixtures/projection-metadata/Cargo.toml");
    let target = root.join("../../../target/projection-metadata-fixture");
    let ordinary = linked_fixture(&manifest, &target, false);
    let inspected = linked_fixture(&manifest, &target, true);
    for marker in [b"MetadataRetentionSentinel".as_slice(), b"metadata_retention_sentinel_method".as_slice()] {
        assert!(
            !ordinary.windows(marker.len()).any(|bytes| bytes == marker),
            "ordinary construction/query retained schema text"
        );
        assert!(
            inspected.windows(marker.len()).any(|bytes| bytes == marker),
            "explicit inspection lost schema text"
        );
    }
}
