//! Inspects a relocated factory plugin without any contract-library search path.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Reads the exact dynamic output reported by Cargo, independent of build-dir layout.
fn shared_artifact(output: &Output, target_name: &str) -> PathBuf {
    let filename = format!("{}{target_name}{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX);
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|record| record["reason"] == "compiler-artifact" && record["target"]["name"] == target_name)
        .flat_map(|record| {
            record["filenames"]
                .as_array()
                .unwrap()
                .iter()
                .map(|name| PathBuf::from(name.as_str().unwrap()))
                .collect::<Vec<_>>()
        })
        .find(|path| path.file_name().is_some_and(|name| name == filename.as_str()))
        .expect("Cargo reports the requested shared library")
}

/// Builds a pure provider that is inspectable without any initialized runtime.
fn build_factory_plugin() -> PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.join("target"));
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .args(["rustc", "--offline", "--locked", "--manifest-path"])
        .arg(workspace.join("examples/com-plugin/Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)
        .args(["--lib", "--crate-type", "cdylib", "--features", "dynamic-plugin", "--message-format=json"])
        .output()
        .expect("build runtime-free factory fixture");
    assert_success("factory plugin build", &output);
    shared_artifact(&output, "nc_example_com")
}

/// Builds the actual CLI and selects its executable from Cargo's artifact record.
/// The unpublished integration owner does not receive another package's
/// `CARGO_BIN_EXE` variables. Cargo, rather than a guessed target directory, owns
/// the executable's location and platform suffix.
fn build_inspector(profile: &str) -> PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let mut command = if profile == "min-size" {
        // Portable qualification copies intentionally omit this configuration.
        // This regression must exercise the actual checkout's Debug stripping.
        assert!(
            std::fs::read_to_string(workspace.join(".cargo/config.toml"))
                .unwrap()
                .contains("-Zfmt-debug=none")
        );
        let mut command = Command::new("rustup");
        command.args(["run", "nightly", "cargo"]);
        command
    } else {
        Command::new(cargo)
    };
    let output = command
        .current_dir(workspace)
        .arg("build")
        .args(["--profile", profile])
        .args([
            "--offline",
            "--locked",
            "-p",
            "neocogi-inspect",
            "--bin",
            "neocogi-inspect",
            "--message-format=json",
        ])
        .output()
        .expect("build inspector binary");
    assert_success("inspector binary build", &output);
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|record| {
            record["reason"] == "compiler-artifact" && record["target"]["name"] == "neocogi-inspect" && record["target"]["kind"] == serde_json::json!(["bin"])
        })
        .filter_map(|record| record["executable"].as_str().map(PathBuf::from))
        .next()
        .expect("Cargo reports the inspector executable")
}

/// Reports child diagnostics before failing a build or inspection assertion.
fn assert_success(subject: &str, output: &Output) {
    assert!(
        output.status.success(),
        "{subject} failed\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Exercises each real output sink independently; earlier writes cannot satisfy later cases.
fn assert_native_output_paths(inspector: &Path, plugin: &Path, intended: &Path, neighbor: &Path) {
    assert_ne!(intended, neighbor);
    for command in ["dump", "plan"] {
        for option in ["--output", "-o"] {
            std::fs::write(intended, b"intended sentinel").unwrap();
            std::fs::write(neighbor, b"neighbor sentinel").unwrap();
            let output = Command::new(inspector)
                .arg(command)
                .arg(plugin)
                .args(["--format", "json", option])
                .arg(intended)
                .env_remove("LD_LIBRARY_PATH")
                .env_remove("DYLD_LIBRARY_PATH")
                .output()
                .expect("inspect to native output path");
            assert_success(command, &output);
            let artifact: serde_json::Value = serde_json::from_slice(&std::fs::read(intended).unwrap()).expect("command-specific output JSON");
            match command {
                "dump" => {
                    assert_eq!(artifact["schema_version"], 9);
                    assert_eq!(artifact["plugin"]["name"], "nc.examples.factory_plugin_component");
                }
                "plan" => {
                    assert_eq!(artifact["schema"], "neocogi-com.load-plan");
                    assert_eq!(artifact["plugins"].as_array().unwrap().len(), 1);
                    assert_eq!(artifact["load_order"].as_array().unwrap().len(), 1);
                    assert_eq!(artifact["load_order"][0]["name"], "nc.examples.factory_plugin_component");
                }
                _ => unreachable!(),
            }
            assert_eq!(std::fs::read(neighbor).unwrap(), b"neighbor sentinel");
        }
        let missing = intended.parent().unwrap().join("missing-plugin.so");
        assert!(!missing.exists());
        let mut inline_output = std::ffi::OsString::from("--output=");
        inline_output.push(intended);
        for option in [inline_output, std::ffi::OsString::from("--format=json")] {
            std::fs::write(intended, b"intended sentinel").unwrap();
            std::fs::write(neighbor, b"neighbor sentinel").unwrap();
            let output = Command::new(inspector)
                .arg(command)
                .arg(&missing)
                .arg(option)
                .output()
                .expect("reject inline option before inspection");
            assert_eq!(
                output.status.code(),
                Some(2),
                "usage failure, not operational load failure: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(std::fs::read(intended).unwrap(), b"intended sentinel");
            assert_eq!(std::fs::read(neighbor).unwrap(), b"neighbor sentinel");
        }
    }
}

/// Checks actual source-path identity and render-before-publication with the
/// same relocated provider used for profile/native-destination qualification.
fn assert_native_source_paths(inspector: &Path, plugin: &Path, directory: &Path) {
    let unicode = directory.join(format!("source-é{}", std::env::consts::DLL_SUFFIX));
    std::fs::copy(plugin, &unicode).unwrap();
    let result = Command::new(inspector)
        .arg("plan")
        .arg(&unicode)
        .env_remove("LD_LIBRARY_PATH")
        .env_remove("DYLD_LIBRARY_PATH")
        .output()
        .unwrap();
    assert_success("Unicode source plan", &result);
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["plugins"][0]["path"], unicode.to_str().unwrap());
    assert_eq!(value["load_order"][0]["path"], unicode.to_str().unwrap());

    #[cfg(target_os = "linux")]
    {
        use std::os::unix::ffi::OsStringExt;
        let mut name = b"source-\xff".to_vec();
        name.extend_from_slice(std::env::consts::DLL_SUFFIX.as_bytes());
        let native = directory.join(std::ffi::OsString::from_vec(name));
        assert!(native.to_str().is_none());
        std::fs::copy(plugin, &native).unwrap();
        for command in ["check", "dump"] {
            let result = Command::new(inspector)
                .arg(command)
                .arg(&native)
                .env_remove("LD_LIBRARY_PATH")
                .env_remove("DYLD_LIBRARY_PATH")
                .output()
                .unwrap();
            assert_success("native source inspection", &result);
        }
        let result = Command::new(inspector)
            .arg("plan")
            .arg(&native)
            .args(["--format", "text"])
            .env_remove("LD_LIBRARY_PATH")
            .env_remove("DYLD_LIBRARY_PATH")
            .output()
            .unwrap();
        assert_success("native source text plan", &result);
        assert!(String::from_utf8(result.stdout).unwrap().contains("nc.examples.factory_plugin_component"));

        let existing = directory.join("source-plan-existing-é.json");
        let absent = directory.join("source-plan-absent.json");
        let neighbor = directory.join("source-plan-neighbor.json");
        assert!(!absent.exists());
        for explicit_format in [false, true] {
            for output in [None, Some(existing.as_path()), Some(absent.as_path())] {
                std::fs::write(&existing, b"existing sentinel").unwrap();
                std::fs::write(&neighbor, b"neighbor sentinel").unwrap();
                let mut command = Command::new(inspector);
                command.arg("plan").arg(&native).env_remove("LD_LIBRARY_PATH").env_remove("DYLD_LIBRARY_PATH");
                if explicit_format {
                    command.args(["--format", "json"]);
                }
                if let Some(output) = output {
                    command.arg("--output").arg(output);
                }
                let result = command.output().unwrap();
                let errors = String::from_utf8(result.stderr).unwrap();
                assert_eq!(result.status.code(), Some(1), "{errors}");
                assert!(result.stdout.is_empty());
                assert!(errors.contains("path contains invalid UTF-8"), "{errors}");
                assert_eq!(std::fs::read(&existing).unwrap(), b"existing sentinel");
                assert_eq!(std::fs::read(&neighbor).unwrap(), b"neighbor sentinel");
                assert!(!absent.exists());
            }
        }
        // Renderer validation must not replace existing inspection/planner errors.
        let missing = directory.join("missing-source-plugin.so");
        assert!(!missing.exists());
        for (other, expected) in [(&native, "multiple artifacts claim plugin"), (&missing, "OpenLibrary")] {
            let result = Command::new(inspector)
                .arg("plan")
                .arg(&native)
                .arg(other)
                .env_remove("LD_LIBRARY_PATH")
                .env_remove("DYLD_LIBRARY_PATH")
                .output()
                .unwrap();
            let errors = String::from_utf8(result.stderr).unwrap();
            assert_eq!(result.status.code(), Some(1), "{errors}");
            assert!(result.stdout.is_empty());
            assert!(errors.contains(expected), "{errors}");
            assert!(!errors.contains("invalid UTF-8"), "{errors}");
        }
    }
}

/// Compare real command output across profiles, including valid and rejected
/// snapshots. Classification and concrete evidence must survive Debug stripping.
fn snapshot_diagnostics(inspector: &Path, base: &serde_json::Value, directory: &Path) -> Vec<(i32, String, String)> {
    let old = directory.join("old.json");
    let new = directory.join("new.json");
    std::fs::write(&old, serde_json::to_vec(base).unwrap()).unwrap();
    let mut results = Vec::new();
    let mut compare = |value: &serde_json::Value, mode: &str, status: i32, stdout: &[&str], stderr: &[&str]| {
        std::fs::write(&new, serde_json::to_vec(value).unwrap()).unwrap();
        let result = Command::new(inspector).arg("diff").arg(&old).arg(&new).arg(mode).output().unwrap();
        let output = String::from_utf8(result.stdout).unwrap();
        let errors = String::from_utf8(result.stderr).unwrap();
        assert_eq!(result.status.code(), Some(status), "{output}\n{errors}");
        for expected in stdout {
            assert!(output.contains(expected), "missing {expected} in {output}");
        }
        for expected in stderr {
            assert!(errors.contains(expected), "missing {expected} in {errors}");
        }
        results.push((status, output, errors));
    };
    compare(base, "--exact", 0, &["no differences"], &[]);
    let mut value = base.clone();
    let old_threading = base["factory"]["root_factory_threading"].as_str().unwrap();
    let new_threading = if old_threading == "thread_local" { "thread_safe" } else { "thread_local" };
    value["factory"]["root_factory_threading"] = new_threading.into();
    value["factory"]["root_factory_signature"] = "11".repeat(32).into();
    compare(
        &value,
        "--exact",
        1,
        &[
            &format!("factory.root_factory_threading: \"{old_threading}\" -> \"{new_threading}\""),
            &format!(
                "factory.root_factory_signature: \"{}\" -> \"{}\"",
                base["factory"]["root_factory_signature"].as_str().unwrap(),
                "11".repeat(32)
            ),
        ],
        &[],
    );
    value = base.clone();
    value["target"]["triple"] = "different-unknown-target".into();
    value["target"]["pointer_width"] = if base["target"]["pointer_width"] == 64 { 32 } else { 64 }.into();
    value["target"]["endianness"] = if base["target"]["endianness"] == "little" { "big" } else { "little" }.into();
    compare(
        &value,
        "--exact",
        1,
        &[
            "target.triple:",
            "different-unknown-target",
            &format!(
                "target.pointer_width: {} -> {}",
                base["target"]["pointer_width"], value["target"]["pointer_width"]
            ),
            "\"little\"",
            "\"big\"",
        ],
        &[],
    );
    compare(
        &value,
        "--compat",
        1,
        &[
            "compatibility: breaking",
            "target ABI changed:",
            base["target"]["triple"].as_str().unwrap(),
            "different-unknown-target",
            "\"pointer_width\":",
            "\"endianness\":",
        ],
        &[],
    );
    value = base.clone();
    value["plugin"]["version"] = r#"v"quoted\release"#.into();
    compare(&value, "--exact", 1, &[r#"v\"quoted\\release"#], &[]);
    compare(&value, "--compat", 0, &["compatibility: compatible", r#"v\"quoted\\release"#], &[]);
    value = base.clone();
    value["schema"] = "wrong\nschema".into();
    compare(
        &value,
        "--exact",
        1,
        &[],
        &[r#"schema: expected "neocogi-com.inspection", found "wrong\nschema""#],
    );
    value = base.clone();
    value["producer"]["name"] = "other-tool".into();
    compare(
        &value,
        "--compat",
        1,
        &[],
        &[r#"producer.name: expected "neocogi-inspect", found "other-tool""#],
    );
    results
}

/// Stages two trusted native rejections from valid static storage: a retired
/// getter and a current getter advertising an old descriptor generation. Copy
/// each exact Cargo artifact before rebuilding the fixture's other feature set.
fn rejected_plugins(directory: &Path) -> Vec<(PathBuf, &'static str)> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.join("target"));
    let mut cases = vec![(directory.join("missing-plugin.so"), "OpenLibrary")];
    assert!(!cases[0].0.exists());
    for (features, phase) in [(Some("legacy-symbol"), "FindSymbol"), (None, "Metadata")] {
        let mut command = Command::new(env!("CARGO"));
        command
            .args([
                "build",
                "--offline",
                "--locked",
                "--lib",
                "--no-default-features",
                "--message-format=json",
                "--manifest-path",
            ])
            .arg(workspace.join("tests/com/integration-tests/fixtures/legacy-bootstrap/Cargo.toml"))
            .arg("--target-dir")
            .arg(&target);
        if let Some(features) = features {
            command.args(["--features", features]);
        }
        let output = command.output().expect("build trusted rejected provider");
        assert_success("rejected provider build", &output);
        let artifact = shared_artifact(&output, "nc_legacy_bootstrap_fixture");
        let staging = directory.join(phase);
        std::fs::create_dir(&staging).unwrap();
        let plugin = staging.join(artifact.file_name().unwrap());
        std::fs::copy(artifact, &plugin).unwrap();
        cases.push((plugin, phase));
    }
    cases
}

/// Real loader failures must retain their stage, path and detail through every
/// CLI inspection command. Metadata also carries its symbolic and raw ABI status.
fn native_load_diagnostics(inspector: &Path, cases: &[(PathBuf, &str)], directory: &Path) -> Vec<(i32, String, String)> {
    let marker = directory.join("unexpected-bootstrap");
    let mut results = Vec::new();
    for (plugin, phase) in cases {
        for command in ["dump", "check", "plan"] {
            let result = Command::new(inspector)
                .arg(command)
                .arg(plugin)
                .env("NC_GENERATION_BOOTSTRAP_MARKER", &marker)
                .output()
                .unwrap();
            let output = String::from_utf8(result.stdout).unwrap();
            let errors = String::from_utf8(result.stderr).unwrap();
            assert_eq!(result.status.code(), Some(1), "{command}: {errors}");
            assert!(output.is_empty(), "{output}");
            let prefix = format!("error: {phase} while loading {}: ", plugin.display());
            let detail = errors.strip_prefix(&prefix).unwrap_or_else(|| panic!("missing {prefix} in {errors}"));
            assert!(!detail.trim().is_empty(), "platform/validation detail must survive");
            if *phase == "Metadata" {
                assert_eq!(detail, "invalid plugin descriptor (NC_E_ABI_VERSION(-5))\n");
            }
            assert!(!marker.exists(), "inspection must not call the retired bootstrap");
            results.push((1, output, errors));
        }
    }
    results
}

/// Relocation and native output identity share one provider build and staging owner.
#[test]
fn inspects_a_standalone_plugin_and_its_exact_factory_signature() {
    let inspector = build_inspector("dev");
    let original = build_factory_plugin();
    let staging = tempfile::tempdir().expect("isolated plugin directory");
    let plugin = staging.path().join(original.file_name().unwrap());
    std::fs::copy(original, &plugin).expect("relocate plugin without build outputs");
    let output = Command::new(&inspector)
        .arg("dump")
        .arg(&plugin)
        .args(["--format", "json"])
        .env_remove("LD_LIBRARY_PATH")
        .env_remove("DYLD_LIBRARY_PATH")
        .output()
        .expect("failed to inspect relocated plugin");
    assert_success("standalone plugin inspection", &output);
    let artifact: serde_json::Value = serde_json::from_slice(&output.stdout).expect("inspection JSON");
    assert_eq!(artifact["schema_version"], 9);
    assert_eq!(artifact["description_abi_version"], 3);
    assert_eq!(artifact["plugin"]["name"], "nc.examples.factory_plugin_component");
    let factory = &artifact["factory"];
    assert_eq!(factory["root_factory_signature"].as_str().unwrap().len(), 64);
    assert!(factory.get("modules").is_none());
    assert!(factory.get("direct_contracts").is_none());
    let output = Command::new(&inspector)
        .arg("check")
        .arg(&plugin)
        .env_remove("LD_LIBRARY_PATH")
        .env_remove("DYLD_LIBRARY_PATH")
        .output()
        .expect("failed to check relocated plugin");
    assert_success("standalone plugin check", &output);

    let _default_diagnostics = snapshot_diagnostics(&inspector, &artifact, staging.path());
    let rejected = rejected_plugins(staging.path());
    let _default_load_diagnostics = native_load_diagnostics(&inspector, &rejected, staging.path());

    assert_native_source_paths(&inspector, &plugin, staging.path());

    assert_native_output_paths(
        &inspector,
        &plugin,
        &staging.path().join("report-é.json"),
        &staging.path().join("unicode-neighbor.json"),
    );
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::ffi::OsStringExt;
        // Linux qualifies the checkout's min-size profile and native byte paths.
        let min_size_inspector = build_inspector("min-size");
        assert_eq!(snapshot_diagnostics(&min_size_inspector, &artifact, staging.path()), _default_diagnostics);
        assert_eq!(
            native_load_diagnostics(&min_size_inspector, &rejected, staging.path()),
            _default_load_diagnostics
        );
        assert_native_source_paths(&min_size_inspector, &plugin, staging.path());
        let intended = staging.path().join(std::ffi::OsString::from_vec(b"report-\xff.json".to_vec()));
        let neighbor = staging.path().join("report-\u{fffd}.json");
        assert_native_output_paths(&inspector, &plugin, &intended, &neighbor);
    }
}

/// Copies the maintained generated calculator into one scratch workspace.
/// Local crate paths exercise the candidate SDK. Return a private build directory
/// so another fixture cannot overwrite the library between Cargo and relocation.
fn prepare_calculator_workspace(root: &Path) -> tempfile::TempDir {
    let checkout = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let example = checkout.join("examples/getting-started");
    for path in [
        "build.rs",
        "schemas/tutorial/calculator.ncidl",
        "plugin/Cargo.toml",
        "plugin/src/lib.rs",
        "host/Cargo.toml",
        "host/src/main.rs",
    ] {
        let output = root.join(path);
        std::fs::create_dir_all(output.parent().unwrap()).unwrap();
        std::fs::copy(example.join(path), output).unwrap();
    }
    let crates = checkout.join("crates");
    std::fs::write(root.join("Cargo.toml"), format!(
        "[workspace]\nresolver='3'\nmembers=['host','plugin']\n[patch.crates-io]\n         neocogi-com-sdk={{path={:?}}}\nneocogi-com-host={{path={:?}}}\nneocogi-idl={{path={:?}}}\n",
        crates.join("com"), crates.join("host"), crates.join("idl"),
    )).unwrap();
    let targets = checkout.join("target");
    std::fs::create_dir_all(&targets).unwrap();
    tempfile::Builder::new().prefix("inspect-calculator-").tempdir_in(targets).unwrap()
}

/// Builds selected scratch packages in their private target and retains Cargo artifact records.
fn build_calculator_workspace(root: &Path, target: &Path, packages: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO"));
    command
        .current_dir(root)
        .args(["build", "--offline", "--message-format=json", "--target-dir"])
        .arg(target);
    for package in packages {
        command.args(["-p", package]);
    }
    let output = command.output().unwrap();
    assert_success("calculator fixture build", &output);
    output
}

/// Inspect the actual native artifacts. A downstream plugin is blocked, and
/// an independent provider is absent from the reported remainder.
#[test]
fn native_plan_reports_the_blocked_plugin_remainder() {
    use neocogi_com::guid_from_qualified_name;
    let inspector = build_inspector("dev");
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let target = prepare_calculator_workspace(root);
    let source = root.join("plugin/src/lib.rs");
    let original = std::fs::read_to_string(&source).unwrap();
    let mut paths = Vec::new();
    let mut blocked = Vec::new();
    let mut independent = None;
    for (suffix, dependency) in [("a", Some("b")), ("b", Some("a")), ("c", Some("a")), ("independent", None)] {
        let name = format!("nc.tests.plan.{suffix}");
        let mut code = original.replace("nc.tutorial.calculator", &name);
        let id = guid_from_qualified_name(&name);
        if let Some(dependency) = dependency {
            code = code.replace(
                &format!(r#"#[neocogi_com_sdk::plugin(name = "{name}")]"#),
                &format!(r#"#[neocogi_com_sdk::plugin(name = "{name}", dependencies = [REQUIRED])]"#),
            );
            code.push_str(&format!(
                "\nconst REQUIRED: neocogi_com_sdk::NcGuid = neocogi_com_sdk::guid_from_qualified_name(\"nc.tests.plan.{dependency}\");\n",
            ));
            blocked.push(id);
        } else {
            independent = Some(id);
        }
        std::fs::write(&source, code).unwrap();
        let result = build_calculator_workspace(root, target.path(), &["getting-started-plugin"]);
        let library = shared_artifact(&result, "getting_started_plugin");
        let staged = root.join(format!("{suffix}{}", std::env::consts::DLL_SUFFIX));
        std::fs::copy(library, &staged).unwrap();
        assert_success("valid plan descriptor", &Command::new(&inspector).arg("check").arg(&staged).output().unwrap());
        paths.push(staged);
    }
    blocked.sort_by_key(|id| (id.data1, id.data2, id.data3, id.data4));
    let path = root.join("plan.json");
    std::fs::write(&path, b"previous plan").unwrap();
    let result = Command::new(&inspector)
        .arg("plan")
        .args(&paths)
        .args(["--format", "json", "--output"])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    let error = String::from_utf8(result.stderr).unwrap();
    assert_eq!(
        error.trim(),
        format!("error: plugins blocked by dependency cycles: {} {} {}", blocked[0], blocked[1], blocked[2])
    );
    assert!(!error.contains(&independent.unwrap().to_string()));
    assert_eq!(std::fs::read(&path).unwrap(), b"previous plan");

    // Removing A's dependency leaves C requiring A. The CLI orders the
    // provider first even when the input paths arrive in reverse order.
    std::fs::write(source, original.replace("nc.tutorial.calculator", "nc.tests.plan.a")).unwrap();
    let result = build_calculator_workspace(root, target.path(), &["getting-started-plugin"]);
    let library = shared_artifact(&result, "getting_started_plugin");
    let acyclic = root.join(format!("acyclic{}", std::env::consts::DLL_SUFFIX));
    std::fs::copy(library, &acyclic).unwrap();
    let result = Command::new(inspector)
        .arg("plan")
        .arg(&paths[2])
        .arg(acyclic)
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert_success("acyclic provider-first plan", &result);
    let plan: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let order = plan["load_order"].as_array().unwrap();
    assert_eq!(order.len(), 2);
    assert_eq!(order[0]["name"], "nc.tests.plan.a");
    assert_eq!(order[1]["name"], "nc.tests.plan.c");
}

/// A generated threading edit changes the exact root digest and the old host
/// rejects that provider. Inspector describes the threading change as context.
#[test]
fn generated_root_threading_change_is_context_for_a_breaking_digest() {
    let inspector = build_inspector("dev");
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let target = prepare_calculator_workspace(root);
    let source = root.join("host/src/main.rs");
    let old = std::fs::read_to_string(&source).unwrap();
    let expected = r#"    let calculator = runtime
        .acquire_plugin_root::<contracts::CalculatorAbi>(plugin)
        .expect("compatible calculator factory");"#;
    assert!(old.contains(expected));
    std::fs::write(
        &source,
        old.replace(
            expected,
            r#"    let root = runtime.acquire_plugin_root::<contracts::CalculatorAbi>(plugin);
    if std::env::args().nth(2).as_deref() == Some("different-contract") {
        assert_eq!(root.err(), Some(neocogi_com_sdk::NC_E_ABI_VERSION));
        println!("observed exact contract rejection");
        return;
    }
    let calculator = root.expect("compatible calculator factory");"#,
        ),
    )
    .unwrap();
    let result = build_calculator_workspace(root, target.path(), &["getting-started-host", "getting-started-plugin"]);
    let host = String::from_utf8_lossy(&result.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|item| item["reason"] == "compiler-artifact" && item["target"]["name"] == "getting-started-host")
        .and_then(|item| item["executable"].as_str().map(PathBuf::from))
        .expect("old fixture host executable");
    let local = root.join(format!("local{}", std::env::consts::DLL_SUFFIX));
    std::fs::copy(shared_artifact(&result, "getting_started_plugin"), &local).unwrap();

    let schema = root.join("schemas/tutorial/calculator.ncidl");
    let old = std::fs::read_to_string(&schema).unwrap();
    assert!(old.contains("interface Calculator"));
    std::fs::write(&schema, old.replace("interface Calculator", "#[threading(shared)]\ninterface Calculator")).unwrap();
    let source = root.join("plugin/src/lib.rs");
    let old = std::fs::read_to_string(&source).unwrap();
    assert!(old.contains("ComPtr<contracts::CalculatorAbi>"));
    assert!(old.contains("com_object(contracts::Calculator)"));
    let shared_provider = old
        .replace(
            "com_object(contracts::Calculator)",
            "com_object(interfaces(contracts::Calculator), thread_safe)",
        )
        .replace(
            "ComPtr<contracts::CalculatorAbi>",
            "ComPtr<contracts::CalculatorAbi, neocogi_com_sdk::ThreadSafe>",
        );
    std::fs::write(&source, shared_provider).unwrap();
    let result = build_calculator_workspace(root, target.path(), &["getting-started-plugin"]);
    let shared = root.join(format!("shared{}", std::env::consts::DLL_SUFFIX));
    std::fs::copy(shared_artifact(&result, "getting_started_plugin"), &shared).unwrap();

    let compatible = Command::new(&host).arg(&local).output().unwrap();
    assert_success("old host with exact provider", &compatible);
    assert_eq!(String::from_utf8_lossy(&compatible.stdout).trim(), "2 + 3 = 5");
    let incompatible = Command::new(&host).arg(&shared).arg("different-contract").output().unwrap();
    assert_success("old host observing exact contract rejection", &incompatible);
    assert_eq!(String::from_utf8_lossy(&incompatible.stdout).trim(), "observed exact contract rejection");

    let inspect = |library: &Path, name: &str| {
        let output = Command::new(&inspector).arg("dump").arg(library).args(["--format", "json"]).output().unwrap();
        assert_success("generated root descriptor", &output);
        let artifact: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        std::fs::write(root.join(name), &output.stdout).unwrap();
        artifact["factory"].clone()
    };
    let old = inspect(&local, "local.json");
    let new = inspect(&shared, "shared.json");
    assert_eq!(old["root_factory_id"], new["root_factory_id"]);
    assert_ne!(old["root_factory_signature"], new["root_factory_signature"]);
    assert_eq!(old["root_factory_threading"], "thread_local");
    assert_eq!(new["root_factory_threading"], "thread_safe");
    let result = Command::new(&inspector)
        .current_dir(root)
        .args(["diff", "local.json", "shared.json", "--compat"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    let report = String::from_utf8(result.stdout).unwrap();
    assert!(report.contains("compatibility: breaking"), "{report}");
    assert!(report.contains("breaking:\n  - root factory ABI signature changed"), "{report}");
    assert!(report.contains("diagnostic:\n  - root factory threading changed"), "{report}");
    assert!(!report.contains("compatible additions and relaxations"), "{report}");

    let result = Command::new(inspector)
        .current_dir(root)
        .args(["diff", "local.json", "shared.json", "--exact"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    let report = String::from_utf8(result.stdout).unwrap();
    assert!(report.contains("factory.root_factory_signature:"), "{report}");
    assert!(
        report.contains(r#"factory.root_factory_threading: "thread_local" -> "thread_safe""#),
        "{report}"
    );
}
