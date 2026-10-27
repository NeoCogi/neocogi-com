//! Regressions exercise actual process/harness boundaries without duplicate COM builds.
use std::{collections::BTreeMap, ffi::OsString, fs};
use serde_json::{Value, json};
use crate::{
    compatibility::*,
    process::{Result, Runner, clean_environment},
};

/// A child mode supplies a marker followed by failure to test exit-code priority.
#[test]
fn subprocess_fixture() {
    if std::env::var_os("XTASK_FAILURE_CHILD").is_some() {
        println!("factory object: 44 x 31");
        std::process::exit(7);
    }
}
/// A matching success marker cannot conceal a failed executable.
#[test]
fn process_exit_precedes_output_admission() -> Result<()> {
    let root = tempfile::tempdir()?;
    let mut runner = Runner::new(root.path().into());
    runner.env.insert("XTASK_FAILURE_CHILD".into(), "1".into());
    let error = runner
        .run(std::env::current_exe()?, ["--exact", "tests::subprocess_fixture", "--nocapture"])
        .unwrap_err();
    assert!(error.to_string().contains("7"));
    Ok(())
}
/// Exact semantic names and executed counts must survive ignored/filter cases.
#[test]
fn harness_admission_rejects_empty_substring_and_ignored_cases() -> Result<()> {
    for listing in ["", "0 tests", "prefix_required: test", "required: benchmark"] {
        assert!(require_tests(listing, &["required"]).is_err());
    }
    require_tests("required: test\nhelper: test", &["required"])?;
    for output in [
        "",
        "test result: ok. 0 passed; 0 failed; 1 ignored;",
        "test result: FAILED. 1 passed; 1 failed;",
    ] {
        assert!(require_execution(output).is_err());
    }
    let mut calls = Vec::new();
    assert!(
        execute_tests(
            |options| {
                calls.push(options.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>());
                Ok(if options.contains(&"--list") {
                    "required: test\nhelper: test"
                } else {
                    "test result: ok. 0 passed; 0 failed; 1 ignored;"
                }
                .into())
            },
            &["required"],
            &[]
        )
        .is_err()
    );
    assert_eq!(calls, [vec!["--list"], vec!["--exact", "required"]]);
    Ok(())
}
/// Real Rust test executables exercise ignored, failing and active sentinel paths.
#[test]
fn actual_harness_requires_sentinels_and_complete_execution() -> Result<()> {
    let checkout = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize()?;
    let runner = Runner::new(checkout);
    let owner = runner.scratch("harness-test-")?;
    let source = owner.path().join("cases.rs");
    fs::write(
        &source,
        r#"
        #[test] #[cfg_attr(ignored_required, ignore)]
        fn required_case() { assert!(!cfg!(failing_required)); }
        #[test] fn required_case_suffix() {}
        #[test] fn helper() { std::fs::write(std::env::var_os("HELPER_MARKER").unwrap(), b"ran").unwrap(); }
        #[test] #[ignore] fn unrelated_ignored() {}
    "#,
    )?;
    let result = (|| {
        for (mode, cfg) in [("ignored", Some("ignored_required")), ("failed", Some("failing_required")), ("active", None)] {
            let binary = owner.path().join(format!("{mode}{}", std::env::consts::EXE_SUFFIX));
            let mut args = words(&["--edition=2024", "--test"]);
            args.push(source.as_os_str().into());
            args.push("-o".into());
            args.push(binary.as_os_str().into());
            if let Some(cfg) = cfg {
                args.extend(words(&["--cfg", cfg]));
            }
            runner.run("rustc", args)?;
            let marker = owner.path().join(format!("{mode}.marker"));
            let mut child = runner.clone();
            child.env.insert("HELPER_MARKER".into(), marker.clone().into_os_string());
            let executed = execute_tests(|options| child.run(&binary, options), &["required_case"], &[]);
            if mode == "active" {
                executed?;
                assert_eq!(fs::read(&marker)?, b"ran");
                for required in [
                    vec!["required_cas"],
                    vec!["required_case", "unrelated_ignored"],
                    vec!["required_case", "missing_case"],
                ] {
                    assert!(execute_tests(|options| child.run(&binary, options), &required, &[]).is_err());
                }
                assert!(execute_tests(|options| child.run(&binary, options), &["required_case"], &["--skip", "required_case"]).is_err());
                execute_tests(
                    |options| child.run(&binary, options),
                    &["required_case", "required_case_suffix"],
                    &["--skip", "helper"],
                )?;
            } else {
                assert!(executed.is_err());
                assert!(!marker.exists());
            }
        }
        Ok(())
    })();
    crate::process::retire(owner, result)
}
/// Per-role command policy cannot inherit a caller's compiler/profile/loader state.
#[test]
fn explicit_policy_removes_ambient_state_and_keeps_harness_last() -> Result<()> {
    let env = clean_environment(
        [
            ("PATH", "tools"),
            ("RUSTC", "stale"),
            ("CARGO_TARGET_DIR", "stale"),
            ("CARGO_PROFILE_RELEASE_PANIC", "stale"),
            ("LD_LIBRARY_PATH", "stale"),
            ("CARGO_BUILD_JOBS", "99"),
            ("RUST_TEST_THREADS", "99"),
        ]
        .into_iter()
        .map(|(a, b)| (a.into(), b.into())),
    );
    assert_eq!(
        env,
        BTreeMap::from([
            ("PATH".into(), "tools".into()),
            ("CARGO_BUILD_JOBS".into(), "1".into()),
            ("RUST_TEST_THREADS".into(), "1".into())
        ])
    );
    let runner = Runner::new(std::env::current_dir()?);
    let build = Build {
        toolchain: "nightly",
        profile: "release",
        panic: "abort",
        lto: "fat",
        randomize: true,
    };
    let (runner, args) = cargo_policy(
        &runner,
        build,
        std::path::Path::new("target with spaces"),
        &words(&["test", "--lib"]),
        "x86_64-unknown-linux-gnu",
        Some(Sanitizer::Thread),
        &["--list"],
    )?;
    assert_eq!(&args[args.len() - 2..], words(&["--", "--list"]));
    assert!(args.iter().position(|s| s == "-Zbuild-std").unwrap() < args.iter().position(|s| s == "--").unwrap());
    assert!(args.contains(&OsString::from("target with spaces")));
    assert_eq!(runner.env[&OsString::from("CARGO_PROFILE_RELEASE_PANIC")], "abort");
    let flags = runner.env[&OsString::from("CARGO_ENCODED_RUSTFLAGS")].to_string_lossy();
    for flag in ["-Zrandomize-layout", "-Zsanitizer=thread", "-Clink-arg=-Wl,--export-dynamic"] {
        assert!(flags.contains(flag));
    }
    assert!(
        cargo_policy(
            &runner,
            build,
            "target".as_ref(),
            &words(&["test"]),
            "aarch64-apple-darwin",
            Some(Sanitizer::Thread),
            &[]
        )
        .is_err()
    );
    Ok(())
}
/// Source/manifest identity, cached artifacts, kind and executable bytes all matter.
#[test]
fn cargo_artifacts_reject_missing_foreign_duplicate_and_renamed_roots() -> Result<()> {
    let owner = tempfile::tempdir()?;
    let manifest = owner.path().join("fixtures/host-runtime/Cargo.toml");
    fs::create_dir_all(manifest.parent().unwrap())?;
    fs::write(&manifest, "[package]")?;
    fs::create_dir(owner.path().join("support"))?;
    let mut records = Vec::new();
    for (name, _) in RUNTIME_TESTS {
        let executable = owner.path().join(format!("{name}.exe"));
        fs::write(&executable, name)?;
        let source = owner.path().join(format!("support/{name}_cases.rs"));
        fs::write(&source, "// cases")?;
        records.push(json!({"reason":"compiler-artifact", "manifest_path":manifest, "target":{"name":name,"kind":["test"],"src_path":source},"profile":{"test":true},"executable":executable,"fresh":false}));
    }
    let admit = |rows: &[Value]| qualification_artifacts(&rows.iter().map(Value::to_string).collect::<Vec<_>>().join("\n"), &manifest);
    for fresh in [false, true] {
        for record in &mut records {
            record["fresh"] = json!(fresh);
        }
        assert_eq!(admit(&records)?.len(), 2);
    }
    assert!(admit(&records[..1]).is_err());
    let mut duplicates = records.clone();
    duplicates.push(records[0].clone());
    assert!(admit(&duplicates).is_err());
    for (pointer, value) in [
        ("/manifest_path", json!(owner.path().join("absent.toml"))),
        ("/executable", json!(owner.path().join("missing.exe"))),
        ("/target/name", json!("renamed")),
        ("/target/kind", json!(["bin"])),
        ("/target/src_path", json!(manifest)),
        ("/profile/test", json!(false)),
        ("/executable", records[1]["executable"].clone()),
    ] {
        let mut changed = records.clone();
        *changed[0].pointer_mut(pointer).unwrap() = value;
        assert!(admit(&changed).is_err(), "{pointer}");
    }
    Ok(())
}
/// Failed source preparation invalidates old success and leaves no scratch directory.
#[test]
fn failed_preparation_replaces_previous_success_receipt() -> Result<()> {
    let owner = tempfile::tempdir()?;
    let report = owner.path().join("receipt.json");
    fs::write(&report, "{\"status\":\"passed\"}")?;
    let runner = Runner::new(owner.path().into());
    assert!(
        run(
            &runner,
            Options {
                suite: Suite::Linked,
                report: Some(report.clone())
            }
        )
        .is_err()
    );
    let receipt: Value = serde_json::from_slice(&fs::read(report)?)?;
    assert_eq!(receipt["status"], "failed");
    assert_eq!(receipt["cases"], json!([]));
    assert_eq!(fs::read_dir(owner.path().join("target/xtask-work"))?.count(), 0);
    Ok(())
}
/// Flag validation precedes build work; default and combined workloads are explicit.
#[test]
fn compatibility_options_fail_closed() -> Result<()> {
    assert_eq!(Options::parse(&[])?.suite, Suite::CrossVersion);
    assert_eq!(Options::parse(&words(&["--suite", "sanitizers"]))?.suite, Suite::Sanitizers);
    for args in [
        vec!["--suite"],
        vec!["--suite", "typo"],
        vec!["--suite", "linked", "--suite", "thread"],
        vec!["--unknown", "value"],
    ] {
        assert!(Options::parse(&words(&args)).is_err());
    }
    Ok(())
}

/// Failed teardown and a failed second sanitizer keep completed evidence but fail.
#[test]
fn reports_publish_success_only_after_all_work_and_teardown() -> Result<()> {
    let owner = tempfile::tempdir()?;
    let path = owner.path().join("receipt.json");
    let options = Options {
        suite: Suite::Sanitizers,
        report: Some(path.clone()),
    };
    for reason in ["thread failure", "source cleanup failed"] {
        fs::write(&path, "{\"status\":\"passed\"}")?;
        let result = reported(&options, |cases| {
            let pending: Value = serde_json::from_slice(&fs::read(&path)?)?;
            assert_eq!(pending["status"], "running");
            assert_eq!(pending["cases"], json!([]));
            cases.push(json!({"case":"address", "status":"passed"}));
            Err(reason.into())
        });
        assert!(result.is_err());
        let receipt: Value = serde_json::from_slice(&fs::read(&path)?)?;
        assert_eq!(receipt["status"], "failed");
        assert_eq!(receipt["error"], reason);
        assert_eq!(receipt["cases"].as_array().unwrap().len(), 1);
    }
    reported(&options, |cases| {
        let pending: Value = serde_json::from_slice(&fs::read(&path)?)?;
        assert_eq!(pending["status"], "running");
        cases.extend([json!({"case":"address"}), json!({"case":"thread"})]);
        Ok(())
    })?;
    let receipt: Value = serde_json::from_slice(&fs::read(&path)?)?;
    assert_eq!(receipt["status"], "passed");
    assert_eq!(receipt["cases"].as_array().unwrap().len(), 2);
    Ok(())
}

/// A completed child result is withheld until its scratch has actually retired.
#[test]
fn owned_scratch_retires_after_success_and_failure() -> Result<()> {
    let owner = tempfile::tempdir()?;
    let path = owner.path().to_owned();
    fs::write(path.join("bytes"), b"fixture")?;
    assert_eq!(crate::process::retire(owner, Ok(42))?, 42);
    assert!(!path.exists());
    let owner = tempfile::tempdir()?;
    let path = owner.path().to_owned();
    let result = crate::process::retire::<()>(owner, Err("child failed".into()));
    assert!(result.unwrap_err().to_string().contains("child failed"));
    assert!(!path.exists());
    Ok(())
}
