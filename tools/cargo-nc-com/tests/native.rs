//! End-to-end source builds, native linking, descriptor ordering, and failed updates.
//!
//! The consumer lives outside this repository. Its runtime/plugin sources use path
//! declarations, exercising the same generated Cargo projects as registry sources.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

/// Creates a regular file and any missing parents within a test-owned directory.
fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// TOML string quoting is serialization, never interpolated shell command text.
fn quoted(path: &Path) -> String {
    toml::Value::String(path.to_str().unwrap().into()).to_string()
}

/// Test-owned consumer application with independent COM components and build caches.
struct Consumer {
    /// Drop owner for all generated sources, Cargo caches, and assembly outputs.
    temporary: tempfile::TempDir,
    /// Consumer directory containing spaces to catch accidental shell argument splitting.
    root: PathBuf,
}

impl Consumer {
    /// Builds a graph with a real shared runtime and two plugins listed in reverse order.
    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("native consumer");
        let com = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/com").canonicalize().unwrap();
        write(
            &root.join("Cargo.toml"),
            &format!(
                "[package]\nname='consumer'\nversion='0.1.0'\nedition='2024'\n[workspace]\nexclude=['provider','dependent']\n\
             [dependencies]\nnc-com-runtime-sys={{path={sys}}}\n\
             [package.metadata.nc-com]\nruntime={{package='nc-com-runtime',path={runtime}}}\n\
             plugins=[{{package='dependent',path='dependent',features=['dependent']}},{{package='provider',path='provider'}}]\n\
             assets=[{{from='assets',to='assets'}}]\n",
                sys = quoted(&com.join("nc-com-runtime-sys")),
                runtime = quoted(&com.join("nc-com-runtime"))
            ),
        );
        write(&root.join("src/main.rs"), include_str!("fixtures/application.rs"));
        write(&root.join("assets/message.txt"), "external asset\n");
        for name in ["provider", "dependent"] {
            write(
                &root.join(name).join("Cargo.toml"),
                &format!(
                    "[package]\nname='{name}'\nversion='0.1.0'\nedition='2024'\n[workspace]\n\
                 [lib]\nname='custom_{name}'\ncrate-type=['rlib','cdylib']\n\
                 [features]\ndefault=[]\ndependent=[]\n\
                 [dependencies]\nnc-com-core={{path={core}}}\n",
                    core = quoted(&com.join("nc-com-core"))
                ),
            );
            write(&root.join(name).join("src/lib.rs"), include_str!("fixtures/plugin.rs"));
        }
        Self { temporary, root }
    }

    /// Creates a command with isolated Cargo output while preserving the selected compiler.
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_cargo-nc-com"));
        command.current_dir(&self.root).env("CARGO_TARGET_DIR", self.temporary.path().join("cache"));
        command.env("CARGO_TERM_COLOR", "never");
        command
    }

    /// Requires success and includes subprocess diagnostics in any assertion failure.
    fn build(&self, arguments: &[&str]) -> Output {
        let output = self.command().args(arguments).output().unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }
}

/// Finds the final stage from the command's user-visible completion record.
fn stage(output: &Output) -> PathBuf {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("Staged ").map(PathBuf::from))
        .unwrap()
}

/// A fresh build links the runtime dynamically, sorts plugins, copies resources, and
/// launches without source-tree paths or Cargo-provided dynamic-library search paths.
#[test]
fn external_consumer_build_run_and_failed_updates() {
    let consumer = Consumer::new();
    let output = consumer.build(&["build", "--offline"]);
    let destination = stage(&output);
    let plugin_list = fs::read_to_string(destination.join("plugins.txt")).unwrap();
    let names: Vec<_> = plugin_list.lines().collect();
    assert_eq!(names.len(), 2);
    // The generated roots have private names; Cargo still imports each component's
    // custom Rust library target and preserves its original descriptor identity.
    assert!(names[0].contains("nc_com_shared_provider"));
    assert!(names[1].contains("nc_com_shared_dependent"));
    let executable = destination.join(format!("consumer{}", std::env::consts::EXE_SUFFIX));
    let relocated = consumer.temporary.path().join("relocated assembly");
    copy_sources(&destination, &relocated);
    let cache = consumer.temporary.path().join("cache");
    let hidden_cache = consumer.temporary.path().join("hidden-cache");
    fs::rename(&cache, &hidden_cache).unwrap();
    let launched = Command::new(relocated.join(executable.file_name().unwrap()))
        .current_dir(consumer.temporary.path())
        .env_remove("LD_LIBRARY_PATH")
        .env_remove("DYLD_LIBRARY_PATH")
        .output()
        .unwrap();
    // Restore the cache before checking status so subsequent locked-build checks keep
    // their original artifacts. During execution, every original build path was absent.
    fs::rename(hidden_cache, cache).unwrap();
    assert!(launched.status.success(), "{}", String::from_utf8_lossy(&launched.stderr));
    assert_eq!(String::from_utf8(launched.stdout).unwrap(), "runtime=1; external asset\n");
    // All generated resolver lockfiles now exist, so --locked must reuse them.
    let second = consumer.build(&["build", "--offline", "--locked"]);
    assert_eq!(stage(&second), destination);
    assert!(
        !String::from_utf8_lossy(&second.stderr).contains("Compiling "),
        "unchanged assemblies must reuse every compiled target: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    let status = consumer.command().args(["run", "--offline", "--locked", "--", "23"]).output().unwrap().status;
    assert_eq!(status.code(), Some(23));
    // Even a valid edit to the consumer lock must not silently replace the resolver's
    // recorded seed during --locked. An unlocked build explicitly accepts the refresh.
    let lock = consumer.root.join("Cargo.lock");
    write(&lock, &(fs::read_to_string(&lock).unwrap() + "\n# consumer lock refreshed\n"));
    let stale = consumer.command().args(["build", "--offline", "--locked"]).output().unwrap();
    assert!(!stale.status.success());
    assert!(String::from_utf8_lossy(&stale.stderr).contains("updated application lock"));
    consumer.build(&["build", "--offline"]);
    let before = fs::read(&executable).unwrap();
    let failed = consumer
        .command()
        .args(["build", "--offline"])
        .env("NC_COM_FIXTURE_MISSING_DEPENDENCY", "1")
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("descriptor inspection failed"));
    assert_eq!(fs::read(&executable).unwrap(), before);
    assert_eq!(fs::read_to_string(destination.join("plugins.txt")).unwrap(), plugin_list);
    // A resource colliding with the reserved manifest also preserves the last assembly.
    write(
        &consumer.root.join("collision.toml"),
        &fs::read_to_string(consumer.root.join("Cargo.toml"))
            .unwrap()
            .split("[package.metadata.nc-com]\n")
            .nth(1)
            .unwrap()
            .replace("to='assets'", "to='plugins.txt'"),
    );
    let failed = consumer.command().args(["build", "--offline", "--config", "collision.toml"]).output().unwrap();
    assert!(!failed.status.success());
    assert_eq!(fs::read(&executable).unwrap(), before);
}

/// The generic release path works without repository aliases or custom profile tables.
#[test]
fn release_and_foreign_target_validation() {
    let consumer = Consumer::new();
    let manifest = consumer.root.join("Cargo.toml");
    write(&manifest, &(fs::read_to_string(&manifest).unwrap() + "\n[profile.release]\nlto='fat'\n"));
    write(&consumer.root.join(".cargo/config.toml"), "[term]\nverbose=true\n");
    let output = consumer.build(&["build", "--offline", "--release"]);
    assert_eq!(stage(&output).file_name().unwrap(), "release");
    let diagnostics = String::from_utf8_lossy(&output.stderr);
    for name in ["nc_com_runtime", "provider", "dependent"] {
        let root = format!("--crate-name nc_com_shared_{name} ");
        let invocation = diagnostics
            .lines()
            .find(|line| line.contains(&root))
            .expect("Cargo reports the shared root's rustc command");
        assert!(invocation.contains("--crate-type cdylib"), "{invocation}");
        assert!(invocation.contains("-C lto=fat"), "the deployed library must receive LTO: {invocation}");
    }
    let failed = consumer
        .command()
        .args(["build", "--offline", "--target", "wasm32-unknown-unknown"])
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("cannot be inspected"));
}

/// Ordinary application dependencies and configured components share one feature
/// graph, including rlib-only implementations; explicit isolation preserves another.
#[test]
fn shared_features_and_explicit_isolation() {
    for isolated in [false, true] {
        let consumer = Consumer::new();
        let manifest = consumer.root.join("Cargo.toml");
        let condition = if isolated { "forbid-shared" } else { "expect-shared" };
        let text = fs::read_to_string(&manifest)
            .unwrap()
            .replace("[dependencies]\n", "[dependencies]\nprovider={path='provider',features=['shared']}\n")
            .replace(
                "package='provider',path='provider'",
                &format!("package='provider',path='provider',features=['{condition}'],isolated={isolated}"),
            );
        write(&manifest, &text);
        let provider = consumer.root.join("provider/Cargo.toml");
        let text = fs::read_to_string(&provider)
            .unwrap()
            .replace("['rlib','cdylib']", "['rlib']")
            .replace("dependent=[]", "dependent=[]\nshared=[]\nexpect-shared=[]\nforbid-shared=[]");
        write(&provider, &text);
        let source = consumer.root.join("provider/src/lib.rs");
        let text = fs::read_to_string(&source).unwrap()
            + "\n\
            #[cfg(all(feature=\"expect-shared\",not(feature=\"shared\")))]\n\
            compile_error!(\"application features did not reach the shared graph\");\n\
            #[cfg(all(feature=\"forbid-shared\",feature=\"shared\"))]\n\
            compile_error!(\"isolated features were unified\");\n";
        write(&source, &text);
        consumer.build(&["run", "--offline"]);
        let warm = consumer.build(&["build", "--offline", "--locked"]);
        assert!(!String::from_utf8_lossy(&warm.stderr).contains("Compiling "));
        if isolated {
            // Removing isolation must actually merge the mutually exclusive features.
            write(&manifest, &fs::read_to_string(&manifest).unwrap().replace("isolated=true", "isolated=false"));
            let failed = consumer.command().args(["build", "--offline"]).output().unwrap();
            assert!(!failed.status.success());
            assert!(String::from_utf8_lossy(&failed.stderr).contains("isolated features were unified"));
        }
    }
}

/// Existing cdylib-only packages remain supported without requiring a Rust API.
/// Both grouped and isolated paths must retain their original native exports.
#[test]
fn native_only_components() {
    let consumer = Consumer::new();
    let provider = consumer.root.join("provider/Cargo.toml");
    write(&provider, &fs::read_to_string(&provider).unwrap().replace("['rlib','cdylib']", "['cdylib']"));
    consumer.build(&["run", "--offline"]);
    let warm = consumer.build(&["build", "--offline", "--locked"]);
    assert!(
        !String::from_utf8_lossy(&warm.stderr).contains("Compiling "),
        "{}",
        String::from_utf8_lossy(&warm.stderr)
    );
    let manifest = consumer.root.join("Cargo.toml");
    write(
        &manifest,
        &fs::read_to_string(&manifest)
            .unwrap()
            .replace("package='provider',path='provider'", "package='provider',path='provider',isolated=true"),
    );
    consumer.build(&["run", "--offline"]);
}

/// Copies fixture source trees without modifying the repository's original crates.
fn copy_sources(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_sources(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// Versioned components resolve through Cargo's registry source machinery and retain
/// lockfiles. A directory source replacement keeps this test offline and unpublished.
#[test]
fn registry_components_with_offline_source_replacement() {
    let consumer = Consumer::new();
    let com = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/com").canonicalize().unwrap();
    let vendor = consumer.root.join("vendor");
    for name in ["nc-com-core", "nc-com-runtime", "provider", "dependent"] {
        let directory = vendor.join(name);
        let extra = match name {
            "nc-com-core" => "[features]\ndefault=[]\nalloc=[]\nstd=['alloc']\n",
            "nc-com-runtime" => "[lib]\ncrate-type=['rlib','cdylib']\n[dependencies]\nnc-com-core={version='0.1.0',features=['std']}\n",
            "provider" => "[lib]\nname='custom_provider'\ncrate-type=['rlib','cdylib']\n[features]\ndependent=[]\n[dependencies]\nnc-com-core='0.1.0'\n",
            _ => "[lib]\nname='custom_dependent'\ncrate-type=['rlib','cdylib']\n[features]\ndependent=[]\n[dependencies]\nnc-com-core='0.1.0'\n",
        };
        write(
            &directory.join("Cargo.toml"),
            &format!("[package]\nname='{name}'\nversion='0.1.0'\nedition='2024'\n{extra}"),
        );
        if name.starts_with("nc-com-") {
            copy_sources(&com.join(name).join("src"), &directory.join("src"));
        } else {
            write(&directory.join("src/lib.rs"), include_str!("fixtures/plugin.rs"));
        }
        // Directory sources have no downloadable tarball. The fixture checksum simply
        // provides a stable registry identity while Cargo exercises its normal resolver.
        write(
            &directory.join(".cargo-checksum.json"),
            &format!("{{\"files\":{{}},\"package\":\"{}\"}}", "0".repeat(64)),
        );
    }
    write(
        &consumer.root.join(".cargo/config.toml"),
        &format!(
            "[source.crates-io]\nreplace-with='fixture-registry'\n[source.fixture-registry]\ndirectory={}\n",
            quoted(&vendor)
        ),
    );
    write(
        &consumer.root.join("registry.toml"),
        "runtime={package='nc-com-runtime',version='=0.1.0'}\n\
         plugins=[{package='dependent',version='=0.1.0',features=['dependent']},{package='provider',version='=0.1.0'}]\n\
         assets=[{from='assets',to='assets'}]\n",
    );
    let output = consumer.build(&["run", "--offline", "--config", "registry.toml"]);
    assert!(String::from_utf8_lossy(&output.stdout).contains("runtime=1; external asset"));
    consumer.build(&["build", "--offline", "--locked", "--config", "registry.toml"]);
    let lock = consumer
        .temporary
        .path()
        .join("cache/nc-com/consumer/default/default/sources/provider/Cargo.lock");
    assert!(
        fs::read_to_string(lock)
            .unwrap()
            .contains("registry+https://github.com/rust-lang/crates.io-index")
    );
    // Consumer source overrides must retain their original relative-path meaning in
    // every generated workspace, including the shared application/component graph.
    let patched = consumer.root.join("patched-provider");
    copy_sources(&vendor.join("provider"), &patched);
    let manifest = consumer.root.join("Cargo.toml");
    write(
        &manifest,
        &(fs::read_to_string(&manifest).unwrap() + "\n[patch.crates-io]\nprovider={path='patched-provider'}\n"),
    );
    let patched_output = consumer.build(&["run", "--offline", "--config", "registry.toml"]);
    assert!(String::from_utf8_lossy(&patched_output.stderr).contains("patched-provider"));
    consumer.build(&["build", "--offline", "--locked", "--config", "registry.toml"]);
    // The short metadata form also works when Cargo.toml already declares the sources.
    let manifest = consumer.root.join("Cargo.toml");
    let content = fs::read_to_string(&manifest).unwrap().replace(
        "[dependencies]\n",
        "[features]\nenable-plugin=['dep:dependent']\n[dependencies]\nnc-com-runtime='=0.1.0'\nprovider='=0.1.0'\ndependent={version='=0.1.0',features=['dependent'],optional=true}\n",
    );
    fs::write(manifest, content).unwrap();
    write(
        &consumer.root.join("declared.toml"),
        "runtime='nc-com-runtime'\nplugins=[{package='dependent',features=['dependent']},'provider']\nassets=[{from='assets',to='assets'}]\n[application]\nfeatures=['enable-plugin']\n",
    );
    let output = consumer.build(&["run", "--offline", "--config", "declared.toml"]);
    assert!(String::from_utf8_lossy(&output.stdout).contains("runtime=1; external asset"));
}

/// The bundled nightly preset must work in a consumer with no .cargo aliases or
/// min-size profile, while retaining compatible standard-library artifacts on rebuild.
#[test]
#[ignore = "requires a nightly toolchain with rust-src on x86_64 Linux"]
fn nightly_min_size_without_workspace_configuration() {
    let consumer = Consumer::new();
    write(&consumer.root.join(".cargo/config.toml"), "[term]\nverbose=true\n");
    let output = consumer.build(&["run", "--offline", "--profile", "min-size"]);
    assert!(String::from_utf8_lossy(&output.stdout).contains("runtime=1; external asset"));
    for name in ["nc_com_runtime", "provider", "dependent"] {
        let root = format!("--crate-name nc_com_shared_{name} ");
        let diagnostics = String::from_utf8_lossy(&output.stderr);
        let invocation = diagnostics.lines().find(|line| line.contains(&root)).expect("shared root is compiled");
        assert!(invocation.contains("-C lto=fat"), "{invocation}");
    }
    let rebuilt = consumer.build(&["run", "--offline", "--locked", "--profile", "min-size"]);
    let diagnostics = String::from_utf8_lossy(&rebuilt.stderr);
    assert!(!diagnostics.contains("Compiling "), "{diagnostics}");
    for name in ["core", "std"] {
        let needle = format!("Compiling {name} v0.0.0");
        assert_eq!(diagnostics.lines().filter(|line| line.contains(&needle)).count(), 0, "{diagnostics}");
    }
}
