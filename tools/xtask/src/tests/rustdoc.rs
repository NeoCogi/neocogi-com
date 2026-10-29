//! Real Cargo children prove documentation policy at the shared process boundary.
use std::{
    ffi::OsString,
    fs,
    path::Path,
    process::{Command, Output},
};

use crate::{
    process::{Result, Runner},
    qualification::strict_rustdoc,
};

/// An outer test captures the real Runner's inherited stderr without changing
/// production streaming. Ambient case variables enter before Runner cleans them.
#[test]
fn selected_policy_child() -> Result<()> {
    let Some(project) = std::env::var_os("XTASK_RUSTDOC_PROJECT") else {
        return Ok(());
    };
    let runner = Runner::new(project.into());
    println!("entered real Rustdoc policy fixture");
    match std::env::var("XTASK_RUSTDOC_OPERATION")?.as_str() {
        "check" => runner.cargo("stable", &["check", "--offline", "--color", "never"]),
        "doc" => strict_rustdoc(&runner).cargo("stable", &["doc", "--no-deps", "--offline", "--color", "never"]),
        operation => return Err(format!("unknown documentation fixture operation {operation:?}").into()),
    }?;
    Ok(())
}

/// Create a dependency-free crate whose only bad-doc difference is the link.
fn project(root: &Path, broken: bool) -> Result<()> {
    fs::create_dir_all(root.join("src"))?;
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='xtask-doc-policy'\nversion='0.0.0'\nedition='2024'\n[workspace]\n",
    )?;
    let link = if broken { "[XtaskMissingTarget]" } else { "[`documented_api`]" };
    fs::write(
        root.join("src/lib.rs"),
        format!("//! A documented crate.\n#![deny(missing_docs)]\n/// Read {link}.\npub fn documented_api() {{}}\n"),
    )?;
    Ok(())
}

/// Invoke only the fixture child; policy is selected inside it, never copied here.
fn observe(runner: &Runner, root: &Path, operation: &str, inherited: &[(&str, OsString)], marker: &Path, cargo_home: &Path) -> Result<Output> {
    let mut child = Command::new(std::env::current_exe()?);
    child
        .args(["--exact", "tests::rustdoc::selected_policy_child", "--show-output"])
        .env_clear()
        .envs(&runner.env)
        .env("XTASK_RUSTDOC_PROJECT", root)
        .env("XTASK_RUSTDOC_OPERATION", operation)
        .env("XTASK_RUSTDOC_SENTINEL", marker)
        .env("CARGO_HOME", cargo_home)
        .current_dir(root);
    for (name, value) in inherited {
        child.env(name, value);
    }
    Ok(child.output()?)
}

/// Success must be a real child case, and failures must be the intended diagnostic.
fn require_outcome(output: &Output, broken: bool) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("entered real Rustdoc policy fixture"),
        "fixture did not execute\n{stdout}\n{stderr}"
    );
    if broken {
        assert!(!output.status.success(), "strict docs unexpectedly succeeded\n{stdout}\n{stderr}");
        assert!(stderr.contains("unresolved link to `XtaskMissingTarget`"), "wrong failure\n{stdout}\n{stderr}");
    } else {
        assert!(output.status.success(), "valid fixture failed\n{stdout}\n{stderr}");
        assert!(
            stdout.lines().any(|line| line == "test tests::rustdoc::selected_policy_child ... ok"),
            "child case was filtered\n{stdout}"
        );
    }
}

/// Filtered inherited selectors cannot lower lint policy, replace Rustdoc or
/// redirect output. Every case uses fresh projects so cached docs cannot mask it.
#[test]
fn strict_documentation_owns_flags_executable_and_output() -> Result<()> {
    let checkout = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize()?;
    let runner = Runner::new(checkout);
    let owner = runner.scratch("rustdoc-policy-")?;
    let result = (|| {
        let sentinel_source = owner.path().join("sentinel.rs");
        fs::write(
            &sentinel_source,
            "/// Records any attempted replacement of Rustdoc.\nfn main() { std::fs::write(std::env::var_os(\"XTASK_RUSTDOC_SENTINEL\").unwrap(), b\"selected\").unwrap(); }\n",
        )?;
        let sentinel = owner.path().join(format!("sentinel{}", std::env::consts::EXE_SUFFIX));
        runner.run(
            "rustup",
            [
                OsString::from("run"),
                "stable".into(),
                "rustc".into(),
                sentinel_source.into_os_string(),
                "-o".into(),
                sentinel.clone().into_os_string(),
            ],
        )?;
        let outside_target = owner.path().join("unowned-target");
        let outside_build = owner.path().join("unowned-build");
        let cases: &[(&str, Vec<(&str, OsString)>)] = &[
            ("clean", vec![]),
            ("encoded", vec![("CARGO_ENCODED_RUSTDOCFLAGS", "--cap-lints\u{1f}allow".into())]),
            ("ordinary", vec![("RUSTDOCFLAGS", "--cap-lints allow".into())]),
            ("build-flags", vec![("CARGO_BUILD_RUSTDOCFLAGS", "--cap-lints allow".into())]),
            ("rustdoc", vec![("RUSTDOC", sentinel.clone().into_os_string())]),
            ("build-rustdoc", vec![("CARGO_BUILD_RUSTDOC", sentinel.into_os_string())]),
            ("warning-policy", vec![("CARGO_BUILD_WARNINGS", "allow".into())]),
            ("target-alias", vec![("CARGO_BUILD_TARGET_DIR", outside_target.clone().into_os_string())]),
            ("build-alias", vec![("CARGO_BUILD_BUILD_DIR", outside_build.clone().into_os_string())]),
        ];
        for (label, inherited) in cases {
            let marker = owner.path().join(format!("{label}.selected"));
            let bad = owner.path().join(label).join("bad");
            let good = owner.path().join(label).join("good");
            project(&bad, true)?;
            project(&good, false)?;
            let cargo_home = owner.path().join("cargo-home");
            if *label == "clean" {
                require_outcome(&observe(&runner, &bad, "check", inherited, &marker, &cargo_home)?, false);
            }
            require_outcome(&observe(&runner, &bad, "doc", inherited, &marker, &cargo_home)?, true);
            require_outcome(&observe(&runner, &good, "doc", inherited, &marker, &cargo_home)?, false);
            assert!(!marker.exists(), "{label} selected an inherited Rustdoc executable");
            assert!(
                good.join("target/doc/xtask_doc_policy/index.html").is_file(),
                "{label} did not produce owned documentation"
            );
            assert!(
                !outside_target.exists() && !outside_build.exists(),
                "{label} redirected output outside its project target"
            );
            println!("Rustdoc policy case {label}: intended diagnostic rejected, valid docs passed, executable and output ownership preserved");
        }
        Ok(())
    })();
    crate::process::retire(owner, result)
}
