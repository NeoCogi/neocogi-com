//! Isolated children exercise the public builders without mutating parent Cargo inputs.

use std::{fs, path::Path, process::Command};
use super::{
    BuildError, Module, SharedSources, contract_links_name,
    test_support::{seed_unrelated, snapshot},
};
use crate::syntax::failure;

/// Selects a child scenario only in IDL's own unit-test executable.
const CASE: &str = "NEOCOGI_IDL_OUTPUT_ADMISSION_CASE";
/// Child marker prevents a filtered-out test from satisfying the parent.
const OBSERVED: &str = "observed output admission boundary";

/// Public builder frontends retain their distinct source ownership rules.
#[derive(Clone, Copy, Debug)]
enum Builder {
    /// Independently generates two local modules.
    Shared,
    /// Publishes one packaged file module through Cargo links metadata.
    File,
    /// Publishes one packaged multipart module through Cargo links metadata.
    Directory,
}

impl Builder {
    /// Stable test-only spelling passed to child processes.
    fn name(self) -> &'static str {
        match self {
            Self::Shared => "shared",
            Self::File => "file",
            Self::Directory => "directory",
        }
    }
    /// Decodes only the three explicitly selected child frontends.
    fn child() -> Self {
        match std::env::var("NEOCOGI_IDL_OUTPUT_BUILDER").unwrap().as_str() {
            "shared" => Self::Shared,
            "file" => Self::File,
            "directory" => Self::Directory,
            other => panic!("unknown builder {other}"),
        }
    }
    /// Replaces authored fields while keeping logical module identities fixed.
    fn sources(self, root: &Path, scalar: &str) {
        let source = root.join("schemas").join(match self {
            Self::Directory => "proof/a/part.ncidl",
            Self::Shared | Self::File => "proof/a.ncidl",
        });
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        let ty = match self {
            Self::Shared => format!("Option<{scalar}>"),
            Self::File | Self::Directory => scalar.to_owned(),
        };
        fs::write(source, format!("#[version(1)] struct Ready {{ value: {ty} }}")).unwrap();
        if matches!(self, Self::Shared) {
            fs::write(root.join("schemas/proof/z.ncidl"), "#[version(1)] struct LongValue { value: u32 }").unwrap();
        }
    }
    /// Uses the actual public API and Cargo environment installed by the parent.
    fn build(self, owner: &str, reverse: bool) -> Result<(), BuildError> {
        match self {
            Self::Shared => {
                let request = SharedSources::new("schemas");
                let request = if reverse {
                    request.module("proof.z", owner).module("proof.a", "proof::a")
                } else {
                    request.module("proof.a", "proof::a").module("proof.z", owner)
                };
                assert_eq!(request.build()?.len(), 2);
            }
            Self::File => {
                Module::new("schemas", "proof/a.ncidl").rust_path(owner).build()?;
            }
            Self::Directory => {
                Module::directory("schemas", "proof/a").rust_path(owner).build()?;
            }
        }
        Ok(())
    }
}

/// Executes exactly one public-builder test with native Cargo input paths.
fn child(test: &str, case: &str, builder: Builder, seeded: bool, reverse: bool, non_unicode: bool) {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("consumer");
    #[cfg(unix)]
    let root = if non_unicode {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};
        root.join(OsString::from_vec(b"native-\xff".to_vec()))
    } else {
        root
    };
    #[cfg(not(unix))]
    assert!(!non_unicode);
    fs::create_dir_all(&root).unwrap();
    let output = root.join("output");
    if !matches!(builder, Builder::Shared) {
        fs::create_dir_all(&output).unwrap();
    }
    let result = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture", "--test-threads=1"])
        .env(CASE, case)
        .env("NEOCOGI_IDL_OUTPUT_BUILDER", builder.name())
        .env("NEOCOGI_IDL_OUTPUT_SEEDED", if seeded { "yes" } else { "no" })
        .env("NEOCOGI_IDL_OUTPUT_REVERSE", if reverse { "yes" } else { "no" })
        .env("CARGO_MANIFEST_DIR", &root)
        .env("OUT_DIR", output)
        .env("CARGO_MANIFEST_LINKS", contract_links_name("proof.a"))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        result.status.success(),
        "{builder:?}, seeded={seeded}, reverse={reverse}: {stdout}\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(stdout.contains(OBSERVED), "child must execute assertions: {stdout}");
    if case != "path" {
        assert!(!stdout.contains("cargo::metadata="), "rejection must not advertise success: {stdout}");
    }
}

/// A later complete-path rejection preserves all prior outputs, including bundled copies.
#[test]
fn complete_path_rejection_preserves_the_entire_output_tree() {
    if std::env::var(CASE).as_deref() == Ok("path") {
        let builder = Builder::child();
        let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
        let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        let reverse = std::env::var("NEOCOGI_IDL_OUTPUT_REVERSE").unwrap() == "yes";
        builder.sources(&root, "u64");
        if std::env::var("NEOCOGI_IDL_OUTPUT_SEEDED").unwrap() == "yes" {
            builder.build("proof::z", reverse).unwrap();
            seed_unrelated(&output);
        }
        let before = snapshot(&output);
        builder.sources(&root, "u32");
        let error = builder.build(&"z".repeat(1020), reverse).unwrap_err();
        let BuildError::Idl(message) = error else {
            panic!("wrong rejection phase: {error:?}")
        };
        assert!(message.contains("invalid public Rust path"), "{message}");
        let module = match builder {
            Builder::Shared => "proof.z",
            Builder::File | Builder::Directory => "proof.a",
        };
        assert!(message.contains(&format!("cannot generate module {module:?}")), "{message}");
        let suffix = match builder {
            Builder::Shared => "::LongValue",
            Builder::File | Builder::Directory => "::Ready",
        };
        assert!(message.contains(&format!("is {} bytes", 1020 + suffix.len())), "{message}");
        assert!(message.contains("maximum is 1024"), "{message}");
        assert_eq!(snapshot(&output), before);
        // The exact complete-path limit succeeds using the same current authored input.
        let prefix = "z".repeat(neocogi_com_core::NC_DESCRIPTOR_RUST_PATH_MAX_LEN - suffix.len());
        builder.build(&prefix, reverse).unwrap();
        assert_ne!(snapshot(&output), before);
        println!("{OBSERVED}");
        return;
    }
    for builder in [Builder::Shared, Builder::File, Builder::Directory] {
        for seeded in [false, true] {
            for reverse in [false, true] {
                child(
                    "cargo::output_admission_tests::complete_path_rejection_preserves_the_entire_output_tree",
                    "path",
                    builder,
                    seeded,
                    reverse,
                    false,
                );
            }
        }
    }
}

/// Denies the second local generator rather than the earlier bundled-source parser.
#[test]
fn later_generation_worker_failure_preserves_the_entire_output_tree() {
    if std::env::var(CASE).as_deref() == Ok("worker") {
        let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
        let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        Builder::Shared.sources(&root, "u64");
        // SharedSources never emits packaged ownership metadata, including on
        // this successful seed before the later rejected public invocation.
        if std::env::var("NEOCOGI_IDL_OUTPUT_SEEDED").unwrap() == "yes" {
            Builder::Shared.build("proof::z", false).unwrap();
            seed_unrelated(&output);
        }
        let before = snapshot(&output);
        Builder::Shared.sources(&root, "u32");
        // Two authored parses, one bundled parse, first generator; deny the next.
        let denial = failure::after(4);
        let error = Builder::Shared.build("proof::z", false).unwrap_err();
        drop(denial);
        let BuildError::Idl(message) = error else {
            panic!("denied wrong phase: {error:?}")
        };
        assert!(message.contains("cannot start generated-Rust syntax worker"), "{message}");
        // An extra earlier worker must not silently move this regression to
        // first-module generation while retaining the same generic error text.
        assert!(message.contains("cannot generate module \"proof.z\""), "{message}");
        assert!(message.contains("test denied syntax worker start"), "{message}");
        assert_eq!(snapshot(&output), before);
        println!("{OBSERVED}");
        return;
    }
    for seeded in [false, true] {
        child(
            "cargo::output_admission_tests::later_generation_worker_failure_preserves_the_entire_output_tree",
            "worker",
            Builder::Shared,
            seeded,
            false,
            false,
        );
    }
}

/// Native roots can serialize unsuccessfully even when relative map paths are UTF-8.
#[cfg(unix)]
#[test]
fn graph_encoding_rejection_precedes_output_changes() {
    if std::env::var(CASE).as_deref() == Ok("graph") {
        let builder = Builder::child();
        let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
        let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        builder.sources(&root, "u32");
        if std::env::var("NEOCOGI_IDL_OUTPUT_SEEDED").unwrap() == "yes" {
            seed_unrelated(&output);
            fs::write(output.join("neocogi-idl-graph.json"), "old graph bytes").unwrap();
        } else {
            // Packaged Cargo supplies an existing OUT_DIR; SharedSources owns
            // its own creation and must leave an absent directory absent.
            if !matches!(builder, Builder::Shared) {
                fs::create_dir_all(&output).unwrap();
            }
        }
        let before = snapshot(&output);
        let error = builder.build("proof::z", false).unwrap_err();
        let BuildError::Json { operation, source, .. } = error else {
            panic!("wrong rejection phase: {error:?}")
        };
        assert!(operation.contains("encode") && operation.contains("graph"), "{operation}");
        assert!(source.to_string().contains("UTF-8"), "{source}");
        assert_eq!(snapshot(&output), before);
        println!("{OBSERVED}");
        return;
    }
    for builder in [Builder::Shared, Builder::File, Builder::Directory] {
        for seeded in [false, true] {
            child(
                "cargo::output_admission_tests::graph_encoding_rejection_precedes_output_changes",
                "graph",
                builder,
                seeded,
                false,
                true,
            );
        }
    }
}
