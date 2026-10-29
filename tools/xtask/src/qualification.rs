//! Quality, compiler-floor and workspace graph checks, with shared serial policy.
use std::{
    collections::BTreeSet,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};
use serde_json::Value;
use crate::{
    compatibility::{ABI_TESTS, IMPORTED_INTERFACE_PROVIDER_TEST, abi_sentinel, execute_tests, require_execution, require_tests, words},
    packages::PUBLISHED,
    process::{Result, Runner, sources},
};

/// SDK selectors are derived from the same six-package inventory as archive checks.
fn selectors(tests: bool, tooling: bool) -> Vec<OsString> {
    let mut args = Vec::new();
    for (name, _) in PUBLISHED {
        args.extend(words(&["-p", name]));
    }
    if tests {
        args.extend(words(&["-p", "neocogi-com-tests", "-p", "neocogi-tooling-tests"]));
    }
    if tooling {
        args.extend(words(&["-p", "xtask"]));
    }
    args
}
/// Execute one labeled stable Cargo row, retaining the locked dependency graph.
fn stable(runner: &Runner, label: &str, args: Vec<OsString>) -> Result<()> {
    println!("==> [quality] {label}");
    runner.run("rustup", words(&["run", "stable", "cargo"]).into_iter().chain(args))?;
    Ok(())
}
/// Run formatting, strict Clippy, feature boundaries, ABI docs, links and rustdoc.
pub(crate) fn quality(runner: &Runner) -> Result<()> {
    stable(runner, "format", words(&["fmt", "--all", "--check"]))?;
    runner.run(
        "rustup",
        words(&[
            "run",
            "stable",
            "rustfmt",
            "--edition",
            "2024",
            "--check",
            "--config-path",
            "rustfmt.toml",
            "examples/getting-started/build.rs",
            "examples/getting-started/host/src/main.rs",
            "examples/getting-started/plugin/src/lib.rs",
        ]),
    )?;
    let mut args = words(&["clippy"]);
    args.extend(selectors(true, true));
    args.extend(words(&["--all-targets", "--all-features", "--locked", "--", "-D", "warnings"]));
    stable(runner, "Clippy, all targets and features", args)?;
    let mut args = words(&["check"]);
    args.extend(selectors(false, false));
    args.extend(words(&["--all-features", "--locked"]));
    stable(runner, "SDK features", args)?;
    for (package, features) in [("neocogi-com", None), ("neocogi-com", Some("alloc")), ("neocogi-com-core", None)] {
        let mut args = words(&["check", "-p", package, "--no-default-features", "--locked"]);
        if let Some(feature) = features {
            args.extend(words(&["--features", feature]));
        }
        stable(runner, "allocation boundary", args)?;
    }
    stable(
        runner,
        "ABI v1 normative snippets",
        words(&[
            "test",
            "--manifest-path",
            "tests/abi-spec/Cargo.toml",
            "--doc",
            "--locked",
            "--target-dir",
            "target/abi-spec",
        ]),
    )?;
    crate::markdown::check(&runner.root)?;
    let mut docs = runner.clone();
    docs.env.insert("RUSTDOCFLAGS".into(), "-D warnings -D missing-docs".into());
    for mode in ["all", "default", "no-default", "alloc"] {
        let mut args = words(&["doc", "--no-deps", "--locked"]);
        if mode == "all" {
            args.extend(selectors(false, false));
            args.push("--all-features".into());
        } else {
            args.extend(words(&["-p", "neocogi-com"]));
            if matches!(mode, "no-default" | "alloc") {
                args.push("--no-default-features".into());
            }
            if mode == "alloc" {
                args.extend(words(&["--features", "alloc"]));
            }
        }
        stable(&docs, &format!("strict rustdoc: {mode}"), args)?;
    }
    println!("quality suite passed");
    Ok(())
}

/// Compiler-floor rows keep RUSTUP_TOOLCHAIN in every compiler-spawning child.
pub(crate) fn msrv(runner: &Runner) -> Result<()> {
    let (owner, mut fixture) = sources(runner)?;
    fixture.env.insert("RUSTUP_TOOLCHAIN".into(), "1.85.0".into());
    let result = (|| {
        fixture.cargo("1.85.0", &["generate-lockfile"])?;
        let mut all = words(&["check"]);
        all.extend(selectors(true, true));
        all.extend(words(&["--all-targets", "--all-features", "--locked"]));
        fixture.run("rustup", words(&["run", "1.85.0", "cargo"]).into_iter().chain(all))?;
        let mut libs = words(&["test"]);
        libs.extend(selectors(false, false));
        libs.extend(words(&["--lib", "--locked"]));
        // Shipped backend tests are library tests, unlike floor_semantics' --test
        // targets. Admit the exact default-feature operation before the existing
        // complete SDK library phase; do not run the full IDL library twice.
        let imported_provider = IMPORTED_INTERFACE_PROVIDER_TEST;
        let listing = fixture.cargo("1.85.0", &["test", "-p", "neocogi-idl", "--lib", "--locked", "--", "--list"])?;
        require_tests(&listing, &[imported_provider])?;
        let execution = fixture.cargo(
            "1.85.0",
            &["test", "-p", "neocogi-idl", "--lib", "--locked", "--", "--exact", imported_provider],
        )?;
        require_execution(&execution)?;
        fixture.run("rustup", words(&["run", "1.85.0", "cargo"]).into_iter().chain(libs))?;
        floor_semantics(&fixture)?;
        for args in [
            vec!["test", "--manifest-path", "tests/abi-spec/Cargo.toml", "--doc", "--locked"],
            vec!["test", "-p", "neocogi-inspect", "--bin", "neocogi-inspect", "--locked"],
            vec!["check", "-p", "neocogi-com", "--locked"],
            vec!["check", "-p", "neocogi-com", "--no-default-features", "--locked"],
            vec!["check", "-p", "neocogi-com", "--no-default-features", "--features", "alloc", "--locked"],
            vec!["check", "-p", "neocogi-com-core", "--no-default-features", "--locked"],
        ] {
            fixture.cargo("1.85.0", &args)?;
        }
        println!("Rust 1.85.0 MSRV suite passed");
        Ok(())
    })();
    crate::process::retire(owner, result)
}
/// Exact semantic sentinels accompany complete integration target execution.
fn floor_semantics(runner: &Runner) -> Result<()> {
    let mut rows = vec![
        ("neocogi-com-tests", "client_views", vec![abi_sentinel("client_views")]),
        (
            "neocogi-com-tests",
            "client_view_admission",
            vec!["client_views_cannot_replace_capability_storage"],
        ),
        (
            "neocogi-com-tests",
            "provider_threading",
            vec!["shared_provider_minimum_is_independent_of_class_mode"],
        ),
    ];
    for (name, sentinel) in ABI_TESTS
        .iter()
        .filter(|(name, _)| matches!(*name, "checked_values" | "output_buffers" | "output_completion"))
    {
        rows.push(("neocogi-com-tests", name, vec![*sentinel]));
    }
    rows.push(("neocogi-tooling-tests", "unit_values", vec!["unit_values_use_checked_storage_in_copied_calls"]));
    rows.push((
        "neocogi-tooling-tests",
        "parser",
        vec![
            "rejects_explicit_receiver_lifetimes",
            "receiver_lifetime_diagnostics_preserve_source_parts",
            "rejects_unsupported_argument_attributes",
            "rejects_unrepresented_type_and_integer_syntax",
            "unsupported_syntax_diagnostics_preserve_source_parts",
            "rejects_rust_2024_keyword_members",
            "keyword_member_diagnostics_preserve_source_parts",
        ],
    ));
    for (package, name, required) in rows {
        execute_tests(
            |options| {
                let mut args = vec!["test", "-p", package, "--all-features", "--locked", "--test", name, "--"];
                args.extend_from_slice(options);
                runner.cargo("1.85.0", &args)
            },
            &required,
            &[],
        )?;
    }
    Ok(())
}

/// Real locked Cargo resolution is stronger than comparing manifest strings.
fn metadata(runner: &Runner, manifest: &Path) -> Result<Value> {
    let mut args = words(&[
        "run",
        "stable",
        "cargo",
        "metadata",
        "--locked",
        "--offline",
        "--format-version",
        "1",
        "--manifest-path",
    ]);
    args.push(manifest.as_os_str().to_owned());
    Ok(serde_json::from_str(&runner.read("rustup", args)?)?)
}

/// Every checkout-only integration source has exactly one unpublished owner.
/// Derive the expected inventory from files so adding a new source cannot
/// silently omit it after public packages disable automatic test discovery.
pub(crate) fn checkout_tests(root: &Path, packages: &[Value]) -> Result<()> {
    let mut expected = BTreeSet::new();
    for relative in ["crates/idl/tests", "tools/neocogi-inspect/tests"] {
        for entry in fs::read_dir(root.join(relative))? {
            let path = entry?.path();
            if path.extension().is_some_and(|extension| extension == "rs") {
                expected.insert(path.canonicalize()?);
            }
        }
    }
    let owner = packages
        .iter()
        .find(|package| package["name"] == "neocogi-tooling-tests")
        .ok_or("missing checkout tooling test owner")?;
    if owner["publish"] != serde_json::json!([]) {
        return Err("checkout tooling tests must remain unpublished".into());
    }
    let targets = |package: &Value| -> Result<BTreeSet<PathBuf>> {
        package["targets"]
            .as_array()
            .ok_or("missing Cargo targets")?
            .iter()
            .filter(|target| target["kind"] == serde_json::json!(["test"]))
            .map(|target| Ok(Path::new(target["src_path"].as_str().ok_or("missing test source")?).canonicalize()?))
            .collect()
    };
    if targets(owner)? != expected {
        return Err("checkout integration sources and registered targets differ".into());
    }
    for name in ["neocogi-idl", "neocogi-inspect"] {
        let package = packages
            .iter()
            .find(|package| package["name"] == name)
            .ok_or("missing public tooling package")?;
        if !targets(package)?.is_empty() {
            return Err(format!("{name} must not advertise checkout-only integration tests").into());
        }
    }
    Ok(())
}
/// Resolve every independent fixture and compile a compiler-free no-default consumer.
pub(crate) fn workspace(runner: &Runner) -> Result<()> {
    let graph = metadata(runner, &runner.root.join("Cargo.toml"))?;
    let members = graph["workspace_members"].as_array().ok_or("missing workspace members")?;
    let packages = graph["packages"].as_array().ok_or("missing workspace packages")?;
    checkout_tests(&runner.root, packages)?;
    let mut published = Vec::new();
    for package in packages {
        if members.contains(&package["id"]) && package["publish"] != serde_json::json!([]) {
            published.push(package);
        }
    }
    if published.len() != PUBLISHED.len() {
        return Err("publishable workspace inventory changed".into());
    }
    let root: toml::Value = toml::from_str(&fs::read_to_string(runner.root.join("Cargo.toml"))?)?;
    let version = root["workspace"]["package"]["version"].as_str().ok_or("missing workspace version")?;
    for package in published {
        if package["version"] != version || !PUBLISHED.iter().any(|(name, _)| package["name"] == *name) {
            return Err("published package version or inventory mismatch".into());
        }
    }
    // Package roles are a dependency invariant, not just naming convention.
    // Host-dependent integration tests belong to tests/com, keeping the SDK
    // independently testable and avoiding a development-only dependency cycle.
    let sdk = packages.iter().find(|package| package["name"] == "neocogi-com").ok_or("missing shared SDK")?;
    let host = packages
        .iter()
        .find(|package| package["name"] == "neocogi-com-host")
        .ok_or("missing host implementation")?;
    let sdk_dependencies = sdk["dependencies"].as_array().ok_or("missing SDK dependencies")?;
    if sdk_dependencies.iter().any(|dependency| dependency["name"] == "neocogi-com-host") {
        return Err("shared SDK must not depend on a host implementation, even for tests".into());
    }
    if !host["dependencies"]
        .as_array()
        .ok_or("missing host dependencies")?
        .iter()
        .any(|dependency| dependency["name"] == "neocogi-com" && dependency["kind"].is_null())
    {
        return Err("host implementation must depend on the shared SDK".into());
    }
    let fixtures = runner.root.join("tests/com/integration-tests/fixtures");
    for entry in fs::read_dir(fixtures)? {
        let path = entry?.path();
        if path.join("Cargo.lock").is_file() {
            metadata(runner, &path.join("Cargo.toml"))?;
        }
    }
    metadata(runner, &runner.root.join("tests/abi-spec/Cargo.toml"))?;
    let owner = runner.scratch("no-default-")?;
    let result = (|| {
        let root = owner.path();
        fs::create_dir(root.join("src"))?;
        let path = toml::Value::String(runner.root.join("crates/com").to_str().ok_or("non-UTF8 manifest path")?.into());
        fs::write(
            root.join("Cargo.toml"),
            format!(
                "[package]\nname='no-default-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={path},default-features=false}}\n"
            ),
        )?;
        fs::write(
            root.join("src/lib.rs"),
            "#![no_std]\npub fn identity() -> neocogi_com::NcGuid { neocogi_com::IID_IUNKNOWN }\n",
        )?;
        let mut consumer = runner.clone();
        consumer.root = root.to_path_buf();
        consumer.env.insert("CARGO_TARGET_DIR".into(), root.join("target").into_os_string());
        let tree = consumer.cargo("stable", &["tree", "--offline", "--edges", "normal,build", "--prefix", "none"])?;
        for compiler in ["neocogi-idl", "syn ", "prettyplease", "serde_json", "neocogi-com-host"] {
            if tree.contains(compiler) {
                return Err(format!("compiler or host dependency in no-default SDK graph: {compiler}").into());
            }
        }
        consumer.cargo("stable", &["check", "--offline"])?;
        let service_tree = consumer.cargo(
            "stable",
            &[
                "tree",
                "--offline",
                "--edges",
                "normal,build",
                "--prefix",
                "none",
                "--features",
                "neocogi-com/macros,neocogi-com/runtime",
            ],
        )?;
        if service_tree.contains("neocogi-com-host") {
            return Err("SDK service clients or macros select a host implementation".into());
        }
        Ok(())
    })();
    crate::process::retire(owner, result)
}
/// Execute 32-bit layout and native-C tests; toolchain/multilib are caller supplied.
pub(crate) fn abi_32(runner: &Runner) -> Result<()> {
    let target = std::env::var("NC_COM_32_BIT_TARGET").unwrap_or_else(|_| "i686-unknown-linux-gnu".into());
    runner.cargo(
        "stable",
        &["test", "-p", "neocogi-com-core", "--target", &target, "--no-default-features", "--locked"],
    )?;
    let owner = runner.scratch("abi32-")?;
    let mut fixture = runner.clone();
    fixture.env.insert("CARGO_TARGET_DIR".into(), owner.path().into());
    let result = fixture
        .cargo(
            "stable",
            &[
                "test",
                "--manifest-path",
                "tests/com/integration-tests/fixtures/native-c-abi/Cargo.toml",
                "--target",
                &target,
                "--locked",
            ],
        )
        .map(|_| ());
    crate::process::retire(owner, result)
}
