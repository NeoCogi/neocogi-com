//! Plain Cargo builds must work after relocation, without contract libraries.

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

/// Computes the platform spelling of a shared-library artifact.
fn library(name: &str) -> String {
    format!("{}{name}{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX)
}

/// Uses one plugin source to prove dynamic activation and a self-contained monolith.
#[test]
fn ordinary_cargo_builds_both_linkage_modes_without_contract_artifacts() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let target = workspace.join("target/plain-cargo-com-fixture");
    let dynamic_stage = tempfile::tempdir().unwrap();
    let monolith_stage = tempfile::tempdir().unwrap();
    let executable = format!("com-plugin-host{}", std::env::consts::EXE_SUFFIX);

    // Plugin compilation has no dependency on a prebuilt runtime or an external
    // driver. The crate-type override affects only this final library target.
    run(&mut cargo(
        &workspace,
        &target,
        &[
            "rustc",
            "-p",
            "neocogi-example-com-plugin",
            "--lib",
            "--crate-type",
            "cdylib",
            "--features",
            "dynamic-plugin",
        ],
    ));
    let plugin = library("nc_example_com");
    std::fs::copy(target.join("debug").join(&plugin), dynamic_stage.path().join(&plugin)).unwrap();
    run(&mut cargo(
        &workspace,
        &target,
        &["rustc", "-p", "neocogi-com-runtime", "--lib", "--crate-type", "cdylib"],
    ));
    let runtime = library("nc_com");
    std::fs::copy(target.join("debug").join(&runtime), dynamic_stage.path().join(runtime)).unwrap();
    run(cargo(
        &workspace,
        &target,
        &["build", "-p", "neocogi-example-com-plugin", "--bin", "com-plugin-host", "--features", "host"],
    )
    .env("NC_COM_RUNTIME_LINK_DIR", target.join("debug")));
    std::fs::copy(target.join("debug").join(&executable), dynamic_stage.path().join(&executable)).unwrap();

    // Only these three files ship. Restrict the loader path to the relocated
    // stage so stale workspace libraries cannot accidentally satisfy a link.
    let dynamic = run(Command::new(dynamic_stage.path().join(&executable))
        .arg(dynamic_stage.path().join(plugin))
        .env("LD_LIBRARY_PATH", dynamic_stage.path())
        .env("DYLD_LIBRARY_PATH", dynamic_stage.path()));
    assert_eq!(String::from_utf8(dynamic.stdout).unwrap().trim(), "factory object: 44 x 31");
    assert_eq!(std::fs::read_dir(dynamic_stage.path()).unwrap().count(), 3);

    // Cargo statically links the plugin's rlib and the runtime into one host.
    // No native plugin export or runtime search directory is required.
    run(&mut cargo(
        &workspace,
        &target,
        &[
            "build",
            "-p",
            "neocogi-example-com-plugin",
            "--bin",
            "com-plugin-host",
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
