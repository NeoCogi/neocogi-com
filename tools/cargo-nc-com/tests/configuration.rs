//! Exercises the installed executable interface from an unrelated workspace.

use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

/// Minimal consumer workspace retained until all subprocesses complete.
struct Fixture {
    /// Temporary owner whose drop removes only this test's generated files.
    directory: tempfile::TempDir,
    /// Workspace path deliberately containing spaces to exercise argument handling.
    root: PathBuf,
}

impl Fixture {
    /// Creates dependency-free Cargo packages so planning needs no registry access.
    fn new(configuration: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("consumer with spaces");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        fs::write(
            root.join("Cargo.toml"),
            format!("[package]\nname = 'consumer'\nversion = '0.1.0'\nedition = '2024'\n[workspace]\n{configuration}"),
        )
        .unwrap();
        Self { directory, root }
    }

    /// Invokes the same binary Cargo installs, retaining diagnostics for assertions.
    fn run(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cargo-nc-com"))
            .current_dir(&self.root)
            .args(arguments)
            .env("CARGO_TARGET_DIR", self.directory.path().join("cache"))
            .output()
            .unwrap()
    }
}

/// Package metadata infers its application and preserves explicit feature overrides.
#[test]
fn package_metadata_and_variants() {
    let fixture = Fixture::new(
        "[package.metadata.nc-com]\nruntime = 'runtime'\nplugins = ['plugin']\n\
        [package.metadata.nc-com.variants.vulkan]\napplication-features = ['vulkan']\n\
        [package.metadata.nc-com.variants.vulkan.component-features]\nplugin = ['dynamic-plugin', 'vulkan']\n",
    );
    let output = fixture.run(&["nc-com", "plan", "--offline", "--variant", "vulkan"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let plan: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(plan["assembly"]["application"]["package"], "consumer");
    assert_eq!(plan["assembly"]["application"]["bin"], "consumer");
    assert_eq!(plan["assembly"]["plugins"][0]["features"][1], "vulkan");
    assert!(!fixture.directory.path().join("cache/debug").exists());
}

/// Standalone configuration overrides metadata and anchors paths to its own directory.
#[test]
fn standalone_configuration_precedence() {
    let fixture = Fixture::new("[package.metadata.nc-com]\nruntime = 'ignored'\n");
    fs::create_dir(fixture.root.join("config")).unwrap();
    fs::write(
        fixture.root.join("config/assembly.toml"),
        "default-assembly = 'demo'\n[assemblies.demo]\nruntime = {package = 'runtime', version = '1'}\n",
    )
    .unwrap();
    let output = fixture.run(&["plan", "--offline", "--config", "config/assembly.toml"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let plan: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(plan["name"], "demo");
    assert_eq!(plan["base"], fixture.root.join("config").to_str().unwrap());
    assert_eq!(plan["assembly"]["runtime"]["package"], "runtime");
}

/// Invalid metadata never reaches compilation or modifies an existing build directory.
#[test]
fn invalid_configuration_is_rejected() {
    for configuration in [
        "runtime = 'runtime'\nunknown = true",
        "runtime = 'runtime'\nplugins = ['runtime']",
        "runtime = 'runtime'\nassets = [{from = 'assets', to = '../outside'}]",
    ] {
        let fixture = Fixture::new(&format!("[package.metadata.nc-com]\n{configuration}\n"));
        let output = fixture.run(&["plan", "--offline"]);
        assert!(!output.status.success());
        assert!(!fixture.directory.path().join("cache").exists());
    }
}

/// CLI errors are diagnosed before project discovery, including conflicting profiles.
#[test]
fn command_line_errors_and_help() {
    let fixture = Fixture::new("");
    for arguments in [vec!["build", "--release", "--profile", "dev"], vec!["run", "--unknown"], vec!["build", "--"]] {
        let output = fixture.run(&arguments);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).starts_with("nc-com:"));
    }
    assert!(fixture.run(&["nc-com", "--help"]).status.success());
}

/// Workspace catalogs select named executables and variants can remove unused plugins.
#[test]
fn workspace_catalog_and_plugin_omission() {
    let fixture = Fixture::new(
        "[workspace.metadata.nc-com]\ndefault-assembly='demo'\n\
        [workspace.metadata.nc-com.assemblies.demo]\nruntime='runtime'\nplugins=['ui','assets']\n\
        [workspace.metadata.nc-com.assemblies.demo.variants.embedded]\n\
        add-application-features=['embedded']\nomit-plugins=['assets']\nassets=[]\n",
    );
    let output = fixture.run(&["plan", "--offline", "--variant", "embedded"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let plan: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(plan["name"], "demo");
    assert_eq!(plan["assembly"]["plugins"], serde_json::json!(["ui"]));
    assert_eq!(plan["assembly"]["application"]["features"], serde_json::json!(["embedded"]));
    let invalid = fixture.run(&["plan", "--offline", "--assembly", "absent"]);
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("unknown assembly"));
}
