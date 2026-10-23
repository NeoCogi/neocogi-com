//! A relocated monolith must run without any native NeoCogi library.

use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

/// Runs a child command and keeps compiler/loader diagnostics on failure.
fn run(command: &mut Command) -> Output {
    let output = command.output().expect("start Cargo or sample host");
    assert!(
        output.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// Builds in a separate directory so nested Cargo never waits for the test's lock.
fn cargo(workspace: &Path, target: &Path, arguments: &[&str]) -> Command {
    let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command
        .current_dir(workspace)
        .args(arguments)
        .args(["--offline", "--locked", "--target-dir"])
        .arg(target);
    command
}

/// Dynamic cross-builds live in the compatibility driver; this checks the static mode.
#[test]
fn ordinary_cargo_builds_a_relocatable_monolith() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let builds = tempfile::tempdir().unwrap();
    let target = builds.path();
    let monolith_stage = tempfile::tempdir().unwrap();
    let executable = format!("neocogi-example-com{}", std::env::consts::EXE_SUFFIX);

    // Cargo statically links the plugin's rlib and the runtime into one host.
    // No native plugin export or runtime search directory is required.
    run(&mut cargo(
        &workspace,
        &target,
        &[
            "build",
            "-p",
            "neocogi-example-com-host",
            "--bin",
            "neocogi-example-com",
            "--features",
            "monolith",
        ],
    ));
    std::fs::copy(target.join("debug").join(&executable), monolith_stage.path().join(&executable)).unwrap();
    let monolith = run(Command::new(monolith_stage.path().join(executable))
        .env_remove("LD_LIBRARY_PATH")
        .env_remove("DYLD_LIBRARY_PATH"));
    assert_eq!(String::from_utf8(monolith.stdout).unwrap().trim(), "factory object: 44 x 31");
    assert_eq!(std::fs::read_dir(monolith_stage.path()).unwrap().count(), 1);
}
