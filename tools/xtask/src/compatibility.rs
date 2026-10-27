//! Independent compiler policies, exact harness admission and relocated execution.
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use crate::process::{Result, Runner, require_file, sources};

/// Only printed by the example after its calls, events, persistence and release pass.
const SUCCESS: &str = "factory object: 44 x 31";
/// Final host tests must come from the maintained manifest and these source roots.
pub(crate) const RUNTIME_TESTS: &[(&str, &str)] = &[
    ("factory_plugin_runtime", "generated_contract_factory_acquires_without_a_class_table"),
    ("object_persistence", "registry_ownership_is_local_and_releases_retained_factories"),
];
/// Pure ABI targets do not enable unused native runtime service clients.
pub(crate) const ABI_TESTS: &[(&str, &str)] = &[
    ("client_views", "mutable_inherited_views_preserve_identity_and_lifecycle"),
    ("checked_values", "invalid_nested_bool_fails_copied_and_buffer_thunks"),
    ("output_buffers", "raw_output_initializes_only_reported_prefix_and_reuses_capacity"),
    (
        "output_completion",
        "generated_calls::generated_commit_completes_with_all_further_allocation_denied",
    ),
    ("exclusive_com", "container_owner_destruction_expires_all_cached_interfaces"),
    ("weak_com", "weak_tokens_do_not_retain_values_and_all_projections_share_identity"),
    (
        "recursive_interfaces",
        "full_pointer_checks_reject_incompatible_arguments_and_release_owned_returns",
    ),
];

/// Select an exact maintained sentinel without depending on table order.
pub(crate) fn abi_sentinel(name: &str) -> &'static str {
    ABI_TESTS.iter().find(|(target, _)| *target == name).expect("maintained ABI target").1
}

/// One role's explicit code-generation policy, applied to all its dependencies.
#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct Build {
    /// Installed Rustup selector, recorded with rustc -vV before execution.
    pub toolchain: &'static str,
    /// Cargo profile spelling; dev artifacts live in debug/.
    pub profile: &'static str,
    /// Executable panic strategy, never an unwind across COM.
    pub panic: &'static str,
    /// off/thin/fat link-time optimization policy.
    pub lto: &'static str,
    /// Nightly private-layout stress with a reproducible fixed seed.
    pub randomize: bool,
}
impl Build {
    /// Ordinary debug/unwind/no-LTO role for one compiler.
    pub const fn ordinary(toolchain: &'static str) -> Self {
        Self {
            toolchain,
            profile: "dev",
            panic: "unwind",
            lto: "off",
            randomize: false,
        }
    }
}
/// Two roles must agree on native target C ABI while selecting compilers separately.
#[derive(Clone, Copy, Debug, Serialize)]
struct Case {
    /// Stable report identity.
    name: &'static str,
    /// Final executable owns the process runtime and application assertions.
    host: Build,
    /// Providers retain allocator-local object destruction.
    plugin: Build,
}
/// Deliberately small compatibility matrix shared by selection and receipts.
const CASES: [Case; 4] = [
    Case {
        name: "linked",
        host: Build::ordinary("stable"),
        plugin: Build::ordinary("stable"),
    },
    Case {
        name: "stable-host",
        host: Build::ordinary("stable"),
        plugin: Build {
            toolchain: "nightly",
            profile: "release",
            panic: "abort",
            lto: "fat",
            randomize: true,
        },
    },
    Case {
        name: "nightly-host",
        host: Build {
            toolchain: "nightly",
            profile: "release",
            panic: "abort",
            lto: "thin",
            randomize: true,
        },
        plugin: Build::ordinary("stable"),
    },
    Case {
        name: "stable-release",
        host: Build {
            toolchain: "stable",
            profile: "release",
            panic: "unwind",
            lto: "thin",
            randomize: false,
        },
        plugin: Build {
            toolchain: "stable",
            profile: "release",
            panic: "abort",
            lto: "fat",
            randomize: false,
        },
    },
];

/// Fixed suites reject misspellings before any sources or targets are created.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Suite {
    /// Native C/ABI/host-owned regressions and the relocated example.
    Linked,
    /// Both stable/nightly role directions.
    CrossVersion,
    /// CrossVersion plus different release/panic/LTO policies.
    BuildFlags,
    /// Pure Rust ownership/raw-storage interpretation.
    Safety,
    /// AddressSanitizer for host and providers together.
    Address,
    /// ThreadSanitizer for host and providers together.
    Thread,
    /// Both sanitizer cases, successful only after both finish.
    Sanitizers,
}
/// Parsed options preserve caller-relative report paths despite lent source roots.
pub(crate) struct Options {
    /// One complete qualification workload.
    pub suite: Suite,
    /// Receipt owned by the caller, never by temporary build output.
    pub report: Option<PathBuf>,
}
impl Options {
    /// Parse explicit flag/value pairs, rejecting ambiguous duplicates and extras.
    pub fn parse(args: &[OsString]) -> Result<Self> {
        let mut suite = None;
        let mut report = None;
        let mut args = args.iter();
        while let Some(flag) = args.next() {
            let value = args.next().ok_or("expected a value after compatibility option")?;
            match flag.to_str() {
                Some("--suite") if suite.is_none() => {
                    suite = Some(match value.to_str() {
                        Some("linked") => Suite::Linked,
                        Some("cross-version") => Suite::CrossVersion,
                        Some("build-flags") => Suite::BuildFlags,
                        Some("safety") => Suite::Safety,
                        Some("address") => Suite::Address,
                        Some("thread") => Suite::Thread,
                        Some("sanitizers") => Suite::Sanitizers,
                        _ => return Err(format!("unknown suite: {value:?}").into()),
                    })
                }
                Some("--report") if report.is_none() => report = Some(std::path::absolute(value)?),
                _ => return Err(format!("unknown or duplicate compatibility option: {flag:?}").into()),
            }
        }
        Ok(Self {
            suite: suite.unwrap_or(Suite::CrossVersion),
            report,
        })
    }
}

impl Suite {
    /// One authoritative selection keeps combined sanitizer success dependent on
    /// both ordered cases and prevents rolling compiler rows from drifting apart.
    fn cases(self) -> Vec<Case> {
        match self {
            Self::Linked => CASES[..1].to_vec(),
            Self::CrossVersion => CASES[1..3].to_vec(),
            Self::BuildFlags => CASES[1..].to_vec(),
            Self::Safety => vec![],
            Self::Address | Self::Thread | Self::Sanitizers => {
                let mut selected = Vec::new();
                if self != Self::Thread {
                    selected.push(Case {
                        name: "address",
                        host: Build::ordinary("nightly"),
                        plugin: Build::ordinary("nightly"),
                    });
                }
                if self != Self::Address {
                    selected.push(Case {
                        name: "thread",
                        host: Build::ordinary("nightly"),
                        plugin: Build::ordinary("nightly"),
                    });
                }
                selected
            }
        }
    }
}

/// Only these native sanitizer policies are supported; target support is explicit.
#[derive(Clone, Copy)]
pub(crate) enum Sanitizer {
    /// Address and checked allocation instrumentation.
    Address,
    /// Data-race instrumentation.
    Thread,
}
impl Sanitizer {
    /// Rust compiler flag and report spelling.
    fn name(self) -> &'static str {
        match self {
            Self::Address => "address",
            Self::Thread => "thread",
        }
    }
}

/// Prove exact semantic names exist; benchmarks/substrings/empty listings fail.
pub(crate) fn require_tests(listing: &str, required: &[&str]) -> Result<()> {
    let names: BTreeSet<_> = listing.lines().filter_map(|line| line.strip_suffix(": test")).collect();
    if names.is_empty() || required.iter().any(|name| !names.contains(name)) {
        return Err(format!("required tests not discovered: {required:?}").into());
    }
    Ok(())
}
/// A successful process must also report at least one actually executed test.
pub(crate) fn require_execution(output: &str) -> Result<()> {
    let expression = regex::Regex::new(r"test result: ok\. (\d+) passed; (\d+) failed;").unwrap();
    let mut passed = false;
    for captures in expression.captures_iter(output) {
        if &captures[2] != "0" {
            return Err("failed harness execution".into());
        }
        passed |= captures[1].parse::<usize>()? > 0;
    }
    if !passed {
        return Err("no successful nonempty test execution".into());
    }
    Ok(())
}
/// Admit each required case before running the complete target with existing skips.
/// An ignored required test cannot hide behind unrelated passing helpers.
pub(crate) fn execute_tests(mut execute: impl FnMut(&[&str]) -> Result<String>, required: &[&str], harness: &[&str]) -> Result<Value> {
    let mut options = harness.to_vec();
    options.push("--list");
    require_tests(&execute(&options)?, required)?;
    for name in required {
        let mut options = harness.to_vec();
        options.extend(["--exact", name]);
        require_execution(&execute(&options)?)?;
    }
    require_execution(&execute(harness)?)?;
    Ok(json!({ "required": required, "exact": (!required.is_empty()).then_some("passed"), "complete": "passed" }))
}

/// Construct one role's isolated Cargo command without any shell interpolation.
/// Cargo options must precede the separator that introduces harness arguments.
pub(crate) fn cargo_policy(
    runner: &Runner,
    build: Build,
    target: &Path,
    args: &[OsString],
    native: &str,
    sanitizer: Option<Sanitizer>,
    harness: &[&str],
) -> Result<(Runner, Vec<OsString>)> {
    let mut flags = Vec::new();
    if build.randomize {
        flags.extend(["-Zrandomize-layout".to_owned(), "-Zlayout-seed=107".to_owned()]);
    }
    let mut extra = words(&["--target", native]);
    if let Some(sanitizer) = sanitizer {
        if native != "x86_64-unknown-linux-gnu" {
            return Err("sanitizer execution requires x86_64-unknown-linux-gnu".into());
        }
        flags.push(format!("-Zsanitizer={}", sanitizer.name()));
        if args.first().is_some_and(|s| s == "build" || s == "test") {
            flags.push("-Clink-arg=-Wl,--export-dynamic".to_owned());
        }
        extra.push("-Zbuild-std".into());
    }
    let mut runner = runner.clone();
    for (key, value) in [
        ("RUSTUP_TOOLCHAIN".to_owned(), build.toolchain.to_owned()),
        ("CARGO_ENCODED_RUSTFLAGS".to_owned(), flags.join("\u{1f}")),
        ("CARGO_INCREMENTAL".to_owned(), "0".to_owned()),
        (format!("CARGO_PROFILE_{}_PANIC", build.profile.to_ascii_uppercase()), build.panic.to_owned()),
        (
            format!("CARGO_PROFILE_{}_LTO", build.profile.to_ascii_uppercase()),
            if build.lto == "off" { "false" } else { build.lto }.to_owned(),
        ),
        (
            format!("CARGO_PROFILE_{}_CODEGEN_UNITS", build.profile.to_ascii_uppercase()),
            if build.lto == "off" { "16" } else { "1" }.to_owned(),
        ),
    ] {
        runner.env.insert(key.into(), value.into());
    }
    let mut command = words(&["run", build.toolchain, "cargo"]);
    command.extend_from_slice(args);
    command.extend(words(&["--locked", "--target-dir"]));
    command.push(target.as_os_str().to_owned());
    command.extend(words(&["--profile", build.profile]));
    command.extend(extra);
    if !harness.is_empty() {
        command.push("--".into());
        command.extend(words(harness));
    }
    Ok((runner, command))
}
/// Run an explicit role policy; all process failures remain authoritative.
fn cargo(runner: &Runner, build: Build, target: &Path, args: &[OsString], native: &str, sanitizer: Option<Sanitizer>, harness: &[&str]) -> Result<String> {
    let (runner, args) = cargo_policy(runner, build, target, args, native, sanitizer, harness)?;
    runner.run("rustup", args)
}
/// Convert static command words while keeping dynamic paths as native OsStrings.
pub(crate) fn words(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}
/// Add a fixture manifest as a native path argument.
fn manifest_args(root: &Path, command: &str, relative: &str) -> Vec<OsString> {
    let mut args = words(&[command, "--manifest-path"]);
    args.push(root.join(relative).into_os_string());
    args
}
/// Return native shared-library spelling; unsupported hosts fail explicitly.
fn library(name: &str) -> Result<String> {
    match std::env::consts::OS {
        "windows" => Ok(format!("{name}.dll")),
        "macos" => Ok(format!("lib{name}.dylib")),
        "linux" | "freebsd" | "openbsd" | "netbsd" => Ok(format!("lib{name}.so")),
        other => Err(format!("unsupported native platform: {other}").into()),
    }
}

/// Cargo's compiler-artifact envelope; other message types are ignored explicitly.
#[derive(Deserialize)]
struct Artifact {
    /// Protocol discriminator.
    reason: String,
    /// Owner's manifest, present for compiler artifacts.
    manifest_path: Option<PathBuf>,
    /// Executable-producing target identity.
    target: Option<Target>,
    /// Test-profile identity; fresh and cached messages are equally authoritative.
    profile: Option<ArtifactProfile>,
    /// Only executable artifacts are admitted as qualification test roots.
    executable: Option<PathBuf>,
}
/// Identity fields necessary to distinguish a maintained test from another binary.
#[derive(Deserialize)]
struct Target {
    /// Cargo target name.
    name: String,
    /// Must be exactly one test kind.
    kind: Vec<String>,
    /// Actual maintained source root.
    src_path: PathBuf,
}
/// Test status is separate from target name and kind.
#[derive(Deserialize)]
struct ArtifactProfile {
    /// Whether Cargo compiled a test harness.
    test: bool,
}

/// Admit the complete, distinct expected executables from fresh or cached Cargo JSON.
pub(crate) fn qualification_artifacts(output: &str, manifest: &Path) -> Result<BTreeMap<String, PathBuf>> {
    let mut result = BTreeMap::new();
    for line in output.lines().filter(|line| line.starts_with('{')) {
        let artifact: Artifact = serde_json::from_str(line)?;
        if artifact.reason != "compiler-artifact" {
            continue;
        }
        let Some(executable) = artifact.executable else {
            continue;
        };
        let target = artifact.target.ok_or("missing artifact target")?;
        let expected = manifest.parent().unwrap().join(format!("../../support/{}_cases.rs", target.name));
        if !RUNTIME_TESTS.iter().any(|(name, _)| *name == target.name)
            || artifact.manifest_path.ok_or("missing artifact manifest")?.canonicalize()? != manifest.canonicalize()?
            || target.kind != ["test"]
            || !artifact.profile.is_some_and(|p| p.test)
            || target.src_path.canonicalize()? != expected.canonicalize()?
            || result.contains_key(&target.name)
        {
            return Err(format!("unexpected host qualification executable: {}", target.name).into());
        }
        require_file(&executable)?;
        result.insert(target.name, executable.canonicalize()?);
    }
    if result.len() != RUNTIME_TESTS.len() || result.values().collect::<BTreeSet<_>>().len() != result.len() {
        return Err("missing or duplicate host qualification test artifacts".into());
    }
    Ok(result)
}
/// Run relocated full host suites and bind their evidence to unchanged file bytes.
fn qualification_tests(runner: &Runner, artifacts: BTreeMap<String, PathBuf>) -> Result<Value> {
    let mut evidence = serde_json::Map::new();
    for (name, executable) in artifacts {
        let before = format!("{:x}", Sha256::digest(fs::read(&executable)?));
        let sentinel = RUNTIME_TESTS.iter().find(|(target, _)| *target == name).unwrap().1;
        let execution = execute_tests(|options| runner.run(&executable, options), &[sentinel], &["--test-threads=1"])?;
        if format!("{:x}", Sha256::digest(fs::read(&executable)?)) != before {
            return Err(format!("qualification executable changed: {name}").into());
        }
        evidence.insert(name, json!({"sha256": before, "host_owned_runtime": "passed", "execution": execution}));
    }
    Ok(evidence.into())
}

/// Preserve pure ABI, native C and intentionally embedded library regression groups.
fn ordinary_tests(runner: &Runner, build: Build, target: &Path, native: &str, sanitizer: Option<Sanitizer>) -> Result<Value> {
    let mut evidence = serde_json::Map::new();
    let mut tests: Vec<(String, Vec<OsString>, Vec<&str>)> = Vec::new();
    if cfg!(windows) {
        tests.push((
            "windows_plugin_paths".into(),
            words(&["test", "-p", "neocogi-com-tests", "--features", "runtime", "--test", "windows_plugin_paths"]),
            vec!["embedded_nul_path_is_rejected_before_descriptor_getter"],
        ));
    }
    for (name, sentinel) in ABI_TESTS {
        tests.push(((*name).into(), words(&["test", "-p", "neocogi-com-tests", "--test", name]), vec![sentinel]));
    }
    if sanitizer.is_some() {
        for package in ["neocogi-com-core", "neocogi-com-host", "neocogi-com"] {
            let mut args = words(&["test", "-p", package, "--lib"]);
            match package {
                "neocogi-com-core" => args.extend(words(&["--features", "std"])),
                "neocogi-com" => args.extend(words(&["--features", "runtime"])),
                _ => {}
            }
            tests.push((package.into(), args, vec![]));
        }
    } else {
        tests.push((
            "native_c_abi".into(),
            manifest_args(&runner.root, "test", "tests/com/integration-tests/fixtures/native-c-abi/Cargo.toml"),
            vec![],
        ));
    }
    for (name, args, required) in tests {
        evidence.insert(
            name,
            execute_tests(|harness| cargo(runner, build, target, &args, native, sanitizer, harness), &required, &[])?,
        );
    }
    Ok(evidence.into())
}

/// Native role targets are cleaned before their copied artifacts execute.
fn clean(runner: &Runner, build: Build, target: &Path) -> Result<()> {
    runner.run(
        "rustup",
        words(&["run", build.toolchain, "cargo", "clean", "--target-dir"])
            .into_iter()
            .chain([target.as_os_str().to_owned()]),
    )?;
    Ok(())
}
/// Own independently built providers and final host; publish success after cleanup.
fn execute(runner: &Runner, case: Case, linked: bool, sanitizer: Option<Sanitizer>) -> Result<Value> {
    println!("{}", serde_json::to_string(&case)?);
    let mut versions = BTreeMap::new();
    for name in [case.host.toolchain, case.plugin.toolchain] {
        if !versions.contains_key(name) {
            versions.insert(name, runner.run("rustup", ["run", name, "rustc", "-vV"])?);
        }
    }
    let triples: BTreeSet<_> = versions
        .values()
        .map(|version| {
            version
                .lines()
                .find_map(|line| line.strip_prefix("host: "))
                .ok_or("missing compiler host triple")
        })
        .collect::<std::result::Result<_, _>>()?;
    if triples.len() != 1 {
        return Err("host/provider compilers must target the same native C ABI".into());
    }
    let native = *triples.first().unwrap();
    let owner = runner.scratch("compatibility-")?;
    let work = owner.path();
    let result = (|| {
        let stage = work.join("relocated");
        fs::create_dir(&stage)?;
        let executable = format!("neocogi-example-com{}", std::env::consts::EXE_SUFFIX);
        let plugin = library("nc_example_com")?;
        let peer = library("nc_registry_peer")?;
        let mut peer_args = manifest_args(&runner.root, "rustc", "tests/com/integration-tests/fixtures/registry-peer/Cargo.toml");
        peer_args.extend(words(&["--lib", "--crate-type", "cdylib", "--features", "dynamic-plugin"]));
        for (role, build, args, filename) in [
            (
                "plugin",
                case.plugin,
                words(&[
                    "rustc",
                    "-p",
                    "neocogi-example-com-plugin",
                    "--lib",
                    "--crate-type",
                    "cdylib",
                    "--features",
                    "dynamic-plugin",
                ]),
                &plugin,
            ),
            ("peer", case.plugin, peer_args, &peer),
            (
                "host",
                case.host,
                words(&["build", "-p", "neocogi-example-com-host", "--bin", "neocogi-example-com"]),
                &executable,
            ),
        ] {
            let target = work.join(role);
            cargo(runner, build, &target, &args, native, sanitizer, &[])?;
            let artifact = target
                .join(native)
                .join(if build.profile == "dev" { "debug" } else { build.profile })
                .join(filename);
            require_file(&artifact)?;
            fs::copy(artifact, stage.join(filename))?;
            clean(runner, build, &target)?;
            if target.exists() {
                return Err(format!("build target survived cleanup: {}", target.display()).into());
            }
        }
        let mut runtime = runner.clone();
        if sanitizer.is_some() {
            runtime.env.insert("ASAN_OPTIONS".into(), "detect_leaks=0".into());
            runtime.env.insert("LSAN_OPTIONS".into(), "detect_leaks=0".into());
        }
        let mut groups = serde_json::Map::new();
        if linked || sanitizer.is_some() {
            let target = work.join("tests");
            groups.insert(
                "ordinary_abi_and_embedded".into(),
                ordinary_tests(&runtime, case.host, &target, native, sanitizer)?,
            );
            clean(runner, case.host, &target)?;
            let manifest = runner.root.join("tests/com/integration-tests/fixtures/host-runtime/Cargo.toml");
            require_file(&manifest.with_file_name("Cargo.lock"))?;
            let mut args = manifest_args(&runner.root, "test", "tests/com/integration-tests/fixtures/host-runtime/Cargo.toml");
            args.extend(words(&["--tests", "--no-run", "--message-format=json"]));
            let output = cargo(runner, case.host, &target, &args, native, sanitizer, &[])?;
            let qualification = work.join("qualification");
            fs::create_dir(&qualification)?;
            let mut artifacts = BTreeMap::new();
            for (name, original) in qualification_artifacts(&output, &manifest)? {
                let destination = qualification.join(original.file_name().unwrap());
                fs::copy(original, &destination)?;
                artifacts.insert(name, destination);
            }
            clean(runner, case.host, &target)?;
            if target.exists() {
                return Err("qualification target survived cleanup".into());
            }
            let mut qualified = runtime.clone();
            qualified.root = qualification;
            groups.insert("host_runtime_regressions".into(), qualification_tests(&qualified, artifacts)?);
        }
        let expected: BTreeSet<_> = [&executable, &plugin, &peer].into_iter().map(OsString::from).collect();
        let actual = fs::read_dir(&stage)?
            .map(|entry| entry.map(|e| e.file_name()))
            .collect::<std::io::Result<BTreeSet<_>>>()?;
        if actual != expected {
            return Err("deployment must contain exactly host and two runtime-free providers".into());
        }
        runtime.root = stage.clone();
        let output = runtime.run(stage.join(&executable), [stage.join(&plugin), stage.join(&peer)])?;
        if output.trim() != SUCCESS {
            return Err("host did not report completion of behavioral checks".into());
        }
        groups.insert("independent_sample".into(), json!("passed"));
        Ok(json!({"case": case, "compilers": versions, "sanitizer": sanitizer.map(Sanitizer::name), "groups": groups, "status": "passed"}))
    })();
    crate::process::retire(owner, result)
}

/// Native-only budget/source-policy cases remain in linked/sanitizer execution.
const MIRI_SKIPS: &[&str] = &[
    "buffer_count_includes_each_nested_record",
    "buffers_count_selected_variants_without_charging_larger_alternatives",
    "maximum_variant_cost_is_admitted_before_dispatch_on_both_sides",
    "provider_validates_all_converted_outputs_before_publishing_any",
    "method_names::tests::every_public_handle_receiver_is_reserved",
];
/// Interpret memory-access targets without native loading or diagnostic suppression.
fn safety(runner: &Runner) -> Result<Value> {
    let compiler = runner.run("rustup", ["run", "nightly", "rustc", "-vV"])?;
    runner.cargo("nightly", &["miri", "--version"])?;
    let owner = runner.scratch("miri-")?;
    let result = (|| {
        let mut targets = vec![
            (
                // Interpret quota semantics; native/MSRV/archive suites own the
                // separate borrowed-rendering and full code-generation checks.
                words(&["-p", "neocogi-idl", "--lib", "semantic_budget_"]),
                vec![
                    "specialize::tests::semantic_budget_cache_hits_spend_batch_work",
                    "specialize::tests::semantic_budget_composes_reentrant_descent",
                ],
            ),
            (
                words(&["-p", "neocogi-com-core", "--lib", "--features", "std"]),
                vec!["abi::array_tests::array_signature_matches_known_sha256_and_distinguishes_shapes"],
            ),
            (
                words(&["-p", "neocogi-com-host", "--lib"]),
                vec!["loader::plan::tests::orders_dependencies_before_dependents_deterministically"],
            ),
            (
                words(&["-p", "neocogi-com", "--lib", "--features", "runtime"]),
                vec![
                    "tests::metadata_identity_query_checks_and_releases_a_concrete_projection",
                    "object::tests::lease_release_does_not_keep_a_protected_value_reference",
                ],
            ),
        ];
        let mut interfaces = vec![
            ("local_com", "local_and_foreign_calls_share_one_gate_and_one_model"),
            (
                "com_main",
                "copied_access::copied_access_outlives_the_outer_record_and_shares_state_across_threads",
            ),
            ("borrowed_values", "owned_message_outlives_producer_guard_and_source"),
        ];
        // Keep the interpreter workload explicit: adding a native ABI target
        // must not silently introduce native-only work or inflate Miri's matrix.
        for name in [
            "client_views",
            "checked_values",
            "output_buffers",
            "output_completion",
            "exclusive_com",
            "weak_com",
        ] {
            interfaces.push((name, abi_sentinel(name)));
        }
        for (name, sentinel) in interfaces {
            let mut required = vec![sentinel];
            if name == "exclusive_com" {
                required.push("thread_safe_owner_drop_waits_for_admitted_readers_and_writers");
            }
            targets.push((words(&["-p", "neocogi-com-tests", "--features", "runtime", "--test", name]), required));
        }
        let harness: Vec<_> = MIRI_SKIPS.iter().flat_map(|name| ["--skip", *name]).collect();
        let mut evidence = Vec::new();
        for (args, required) in targets {
            let mut command = words(&["run", "nightly", "cargo", "miri", "test"]);
            command.extend(args.clone());
            command.extend(words(&["--locked", "--target-dir"]));
            command.push(owner.path().as_os_str().to_owned());
            command.push("--".into());
            let executed = execute_tests(
                |options| runner.run("rustup", command.iter().cloned().chain(words(options))),
                &required,
                &harness,
            )?;
            // These selectors are static UTF-8 words, not filesystem paths.
            // Serialize readable words rather than OsString's platform byte enum.
            let labels = args.iter().map(|arg| arg.to_str().expect("static Miri selector")).collect::<Vec<_>>();
            evidence.push(json!({"target": labels, "execution": executed}));
        }
        Ok(json!({"suite": "safety", "compiler": compiler, "status": "passed", "targets": evidence.len(), "execution": evidence}))
    })();
    // Delete only this task's target, never cargo-miri's shared sysroot cache.
    crate::process::retire(owner, result)
}

/// Receipt states follow running -> passed/failed; cleanup is part of the task.
#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    /// Invalidates any previous success before preparation.
    Running,
    /// Every selected case and all owned cleanup completed.
    Passed,
    /// Preparation, execution or cleanup failed.
    Failed,
}
/// Caller-owned report cannot remain green after failed current preparation.
fn report(options: &Options, status: Status, cases: &[Value], error: Option<String>) -> Result<()> {
    if let Some(path) = &options.report {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            path,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&json!({"suite": options.suite, "status": status, "cases": cases, "error": error}))?
            ),
        )?;
    }
    Ok(())
}
/// Invalidate old evidence before source preparation and finalize only after teardown.
pub(crate) fn run(runner: &Runner, options: Options) -> Result<()> {
    reported(&options, |cases| {
        let (owner, fixture) = sources(runner)?;
        let executed = (|| {
            if options.suite == Suite::Safety {
                cases.push(safety(&fixture)?);
            }
            for case in options.suite.cases() {
                let sanitizer = match case.name {
                    "address" => Some(Sanitizer::Address),
                    "thread" => Some(Sanitizer::Thread),
                    _ => None,
                };
                cases.push(execute(&fixture, case, options.suite == Suite::Linked, sanitizer)?);
                report(&options, Status::Running, cases, None)?;
            }
            Ok(())
        })();
        crate::process::retire(owner, executed)
    })
}

/// Own the receipt lifecycle around preparation, all cases and explicit cleanup.
/// Keeping publication outside task execution prevents teardown failures from
/// retaining a passed receipt, while completed cases survive a later failure.
pub(crate) fn reported(options: &Options, task: impl FnOnce(&mut Vec<Value>) -> Result<()>) -> Result<()> {
    report(options, Status::Running, &[], None)?;
    let mut cases = Vec::new();
    let result = task(&mut cases);
    match result {
        Ok(()) => {
            report(options, Status::Passed, &cases, None)?;
            println!("Compatibility checks passed: {} executed case(s)", cases.len());
            Ok(())
        }
        Err(error) => {
            report(options, Status::Failed, &cases, Some(error.to_string()))?;
            Err(error)
        }
    }
}

#[cfg(test)]
mod selection_tests {
    use super::*;
    /// Combined workloads contain all required rows in their intended order.
    #[test]
    fn suite_selection_keeps_both_sanitizers_and_compiler_directions() {
        for (suite, names) in [
            (Suite::Linked, vec!["linked"]),
            (Suite::CrossVersion, vec!["stable-host", "nightly-host"]),
            (Suite::BuildFlags, vec!["stable-host", "nightly-host", "stable-release"]),
            (Suite::Safety, vec![]),
            (Suite::Address, vec!["address"]),
            (Suite::Thread, vec!["thread"]),
            (Suite::Sanitizers, vec!["address", "thread"]),
        ] {
            assert_eq!(suite.cases().iter().map(|case| case.name).collect::<Vec<_>>(), names);
        }
    }
}
