#!/usr/bin/env python3
"""Execute independently selected COM builds; successful compilation alone never passes a case.

Only Python's standard library and installed Rust toolchains are required. Targets
are temporary, cleaned after each artifact is copied, and absent when the staged
host runs. Shell/PowerShell/CI entry points delegate build policy to this file.
"""
from __future__ import annotations

import argparse
from collections.abc import Callable
from dataclasses import asdict, dataclass
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import tempfile

# Resolve from the script, so callers need neither a particular cwd nor shell quoting.
ROOT = Path(__file__).resolve().parents[1]
# Printed only after the example verifies calls, event identity, persistence and final release.
SUCCESS = "factory object: 44 x 31"


@dataclass(frozen=True)
class Build:
    """One artifact's independent compiler and code-generation policy."""
    toolchain: str  # Rustup toolchain, resolved and recorded with rustc -vV before building.
    profile: str = "dev"  # Cargo profile; unlike test harnesses, executable panic settings apply.
    panic: str = "unwind"  # Provider/host may use different policies, without unwinding over COM.
    lto: str = "off"  # Off, thin or fat; each process build receives its own setting.
    randomize: bool = False  # Nightly stress of private Rust layout, never a change to the C ABI.


@dataclass(frozen=True)
class Case:
    """Two independently built roles that must communicate through one declared C ABI."""
    name: str  # Stable report/CLI identity; unknown names fail before any build.
    host: Build  # Owns the runtime, application queue and executes all behavioral assertions.
    plugin: Build  # Owns the object implementation and its allocator-local destruction.


# A small deliberate matrix replaces duplicated same-toolchain/profile loops.
CASES = (
    Case("linked", Build("stable"), Build("stable")),
    Case("stable-host", Build("stable"), Build("nightly", "release", "abort", "fat", True)),
    Case("nightly-host", Build("nightly", "release", "abort", "thin", True), Build("stable")),
    Case("stable-release", Build("stable", "release", "unwind", "thin"), Build("stable", "release", "abort", "fat")),
)
# Keep semantic sentinels next to their actual target, not stale names in several scripts.
RUNTIME_TESTS = {
    "factory_plugin_runtime": "generated_contract_factory_acquires_without_a_class_table",
    "object_persistence": "registry_ownership_is_local_and_releases_retained_factories",
}
# These targets exercise generated calls and ownership without native runtime imports.
ABI_TESTS = {
    "client_views": "mutable_inherited_views_preserve_identity_and_lifecycle",
    "checked_values": "invalid_nested_bool_fails_copied_and_buffer_thunks",
    "output_buffers": "raw_output_initializes_only_reported_prefix_and_reuses_capacity",
    "output_completion": "generated_calls::generated_commit_completes_with_all_further_allocation_denied",
    "exclusive_com": "container_owner_destruction_expires_all_cached_interfaces",
    "weak_com": "weak_tokens_do_not_retain_values_and_all_projections_share_identity",
    "recursive_interfaces": "full_pointer_checks_reject_incompatible_arguments_and_release_owned_returns",
}


def run(arguments: list[str], *, env: dict[str, str], cwd: Path | None = None) -> str:
    """Run one explicit argument vector, retaining diagnostics and requiring success."""
    print("+", subprocess.list2cmdline([str(arg) for arg in arguments]), flush=True)
    # Stream expensive compiler/Miri work into CI logs while retaining the same
    # output for discovery and execution checks after the process has exited.
    lines = []
    with subprocess.Popen(arguments, cwd=ROOT if cwd is None else cwd, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT) as process:
        for line in process.stdout:
            print(line, end="", flush=True)
            lines.append(line)
        status = process.wait()
    if status:
        raise RuntimeError(f"command exited {status}: {arguments[0]}")
    return "".join(lines)


def clean_environment() -> dict[str, str]:
    """Remove ambient build/loader overrides so a case cannot inherit its claimed differences.

    Registry/network credentials and OS tool discovery are retained. Explicit
    compiler/profile/target/loader state belongs exclusively to this driver.
    """
    env = dict(os.environ)
    for key in tuple(env):
        if key.startswith(("CARGO_PROFILE_", "CARGO_TARGET_", "RUSTUP_TOOLCHAIN")) or key in {
            "CARGO", "RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "RUSTFLAGS",
            "CARGO_ENCODED_RUSTFLAGS", "RUSTDOCFLAGS", "CARGO_BUILD_TARGET", "CARGO_BUILD_RUSTFLAGS",
            "CARGO_BUILD_RUSTC", "CARGO_BUILD_RUSTC_WRAPPER", "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
            "MIRIFLAGS", "MIRI_SYSROOT", "ASAN_OPTIONS", "LSAN_OPTIONS", "TSAN_OPTIONS",
            "LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH", "DYLD_FALLBACK_LIBRARY_PATH",
        }:
            env.pop(key)
    return env


def library(name: str) -> str:
    """Return the native shared-library spelling used by Rust on the executing platform."""
    match platform.system():
        case "Windows": return name + ".dll"
        case "Darwin": return "lib" + name + ".dylib"
        case "Linux" | "FreeBSD" | "OpenBSD" | "NetBSD": return "lib" + name + ".so"
        case other: raise RuntimeError(f"unsupported execution platform: {other}")


def require_file(path: Path) -> Path:
    """Reject missing/empty artifacts immediately instead of falling back to old target contents."""
    if not path.is_file() or path.stat().st_size == 0:
        raise RuntimeError(f"required artifact was not produced: {path}")
    return path


def require_tests(listing: str, required: str | None = None) -> None:
    """Check exact discovered test names; an empty filter is a failure, never green evidence."""
    names = {line.removesuffix(": test") for line in listing.splitlines() if line.endswith(": test")}
    if not names or (required is not None and required not in names):
        raise RuntimeError(f"required tests not discovered: {required or 'at least one test'}")


def require_execution(output: str) -> None:
    """Ensure the harness executed tests, rather than reporting only filtered/ignored cases."""
    results = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed;", output)
    if not results or not any(int(passed) > 0 for passed, _ in results) or any(int(failed) for _, failed in results):
        raise RuntimeError("no successful nonempty test execution")


def execute_tests(execute: Callable[[tuple[str, ...]], str], required: str | tuple[str, ...] | None = None,
                  harness: tuple[str, ...] = ()) -> dict:
    """Prove every required case ran, then retain the complete maintained target.

    Discovery alone includes ignored tests. Selecting the exact required name
    makes an ignored/filtered case produce zero passed tests, so unrelated helper
    successes cannot substitute for it. Existing skips remain active in every run.
    The callback owns compiler/target/role policy and must reject child failures.
    """
    names = (required,) if isinstance(required, str) else required or ()
    listing = execute((*harness, "--list"))
    require_tests(listing)
    for name in names:
        require_tests(listing, name)
    for name in names:
        require_execution(execute((*harness, "--exact", name)))
    require_execution(execute(harness))
    return {"required": required, "exact": "passed" if required else None, "complete": "passed"}


def cargo(build: Build, target: Path, arguments: list[str], env: dict[str, str], native_target: str, sanitizer: str | None = None, harness: tuple[str, ...] = ()) -> str:
    """Apply one build policy to the final artifact and all its Rust dependencies."""
    flags = ["-Zrandomize-layout", "-Zlayout-seed=107"] if build.randomize else []
    extra = ["--target", native_target]
    if sanitizer:
        # Instrument std and the host and provider artifacts consistently; sanitizer runs do
        # not claim support for arbitrary mixtures of instrumented runtimes.
        if native_target != "x86_64-unknown-linux-gnu":
            raise RuntimeError("sanitizer execution currently supports x86_64-unknown-linux-gnu")
        flags += [f"-Zsanitizer={sanitizer}"]
        if arguments[0] in ("build", "test"):
            # The sanitizer runtime lives in the executable. A later dlopen may
            # need hooks unused by the host itself; export them before section
            # collection rather than discovering missing symbols at activation.
            flags += ["-Clink-arg=-Wl,--export-dynamic"]
        extra += ["-Zbuild-std"]
    policy = {
        **env,
        "CARGO_ENCODED_RUSTFLAGS": "\x1f".join(flags),
        "CARGO_INCREMENTAL": "0",
        f"CARGO_PROFILE_{build.profile.upper()}_PANIC": build.panic,
        f"CARGO_PROFILE_{build.profile.upper()}_LTO": "false" if build.lto == "off" else build.lto,
        f"CARGO_PROFILE_{build.profile.upper()}_CODEGEN_UNITS": "1" if build.lto != "off" else "16",
    }
    return run(["rustup", "run", build.toolchain, "cargo", *arguments,
                "--locked", "--target-dir", str(target), "--profile", build.profile, *extra,
                *(["--", *harness] if harness else [])], env=policy)


def ordinary_tests(build: Build, target: Path, env: dict[str, str], native_target: str, sanitizer: str | None) -> dict:
    """Execute runtime-free ABI cases and explicitly embedded library owners.

    These are valuable regressions, but their test counts do not prove cross-provider
    authority. They receive no runtime DSO or loader/link-directory overrides.
    """
    evidence = {}
    if platform.system() == "Windows":
        args = ["test", "-p", "neocogi-com-tests", "--features", "runtime", "--test", "windows_plugin_paths"]
        evidence["windows_plugin_paths"] = execute_tests(
            lambda options: cargo(build, target, args, env, native_target, harness=options),
            "embedded_nul_path_is_rejected_before_descriptor_getter")
    for target_name, sentinel in ABI_TESTS.items():
        # These cases need no runtime client capability. Exporting sanitizer
        # hooks retains public functions, so an unused runtime feature would
        # create native imports even though none of these tests calls them.
        args = ["test", "-p", "neocogi-com-tests", "--test", target_name]
        evidence[target_name] = execute_tests(
            lambda options: cargo(build, target, args, env, native_target, sanitizer, options), sentinel)
    if sanitizer:
        # Preserve pure library coverage without the retired multi-plugin examples.
        for package in ("neocogi-com-core", "neocogi-com-runtime", "neocogi-com"):
            args = ["test", "-p", package, "--lib"]
            match package:
                case "neocogi-com-core": args += ["--features", "std"]
                case "neocogi-com": args += ["--features", "runtime"]
            evidence[package] = execute_tests(
                lambda options: cargo(build, target, args, env, native_target, sanitizer, options))
    else:
        # Execute the C fixture directly so its target is owned and cleaned here.
        # It checks native C layout; it does not instrument the C compiler itself.
        manifest = ROOT / "tests/com/integration-tests/fixtures/native-c-abi/Cargo.toml"
        args = ["test", "--manifest-path", str(manifest)]
        evidence["native_c_abi"] = execute_tests(
            lambda options: cargo(build, target, args, env, native_target, harness=options))
    return evidence


def qualification_artifacts(output: str, manifest: Path) -> dict[str, Path]:
    """Admit the exact maintained final-host test artifacts in fresh or cached builds.

    Cached compiler-artifact records count just as fresh ones: admitting only
    records rebuilt this time must not hide an unexpected test executable.
    The maintained case roots, package manifest and test kinds jointly identify
    this fixed fixture; package/test names alone cannot identify its assembly.
    """
    artifacts = {}
    for line in output.splitlines():
        if not line.startswith("{"):
            continue
        record = json.loads(line)
        if record.get("reason") != "compiler-artifact":
            continue
        target = record["target"]
        if not record.get("executable"):
            continue
        name = target["name"]
        if (Path(record["manifest_path"]).resolve() != manifest.resolve()
                or name not in RUNTIME_TESTS or target["kind"] != ["test"]
                or not record["profile"]["test"]
                or Path(target["src_path"]).resolve() != (manifest.parent / f"../../support/{name}_cases.rs").resolve()
                or name in artifacts):
            raise RuntimeError(f"unexpected host qualification executable: {name}")
        artifacts[name] = require_file(Path(record["executable"]))
    if artifacts.keys() != RUNTIME_TESTS.keys():
        raise RuntimeError("missing host qualification test artifacts")
    if len(set(artifacts.values())) != len(artifacts):
        raise RuntimeError("host qualification executables must be distinct")
    return artifacts


def qualification_tests(artifacts: dict[str, Path], env: dict[str, str], cwd: Path) -> dict:
    """Execute complete relocated host-owned suites without a COM runtime DSO."""
    evidence = {}
    for name, executable in artifacts.items():
        digest = hashlib.sha256(require_file(executable).read_bytes()).hexdigest()
        executed = execute_tests(lambda options: run([str(executable), *options], env=env, cwd=cwd),
                                 RUNTIME_TESTS[name], ("--test-threads=1",))
        if hashlib.sha256(require_file(executable).read_bytes()).hexdigest() != digest:
            raise RuntimeError(f"qualification executable changed during execution: {name}")
        evidence[name] = {"sha256": digest, "host_owned_runtime": "passed", "execution": executed}
    return evidence


def safety() -> dict:
    """Interpret current pure Rust targets with Miri; native loading stays in other suites.

    Test executables explicitly retain their runtime: Miri cannot execute arbitrary dlopen
    calls. No Miri safety diagnostic is suppressed and no deleted plugin fixture is
    reconstructed. Exact sentinels and executed counts are checked just as for
    native tests, and temporary build outputs are removed on success or failure.
    """
    env = clean_environment()
    if not (ROOT / "Cargo.lock").is_file():
        run(["rustup", "run", "nightly", "cargo", "generate-lockfile"], env=env)
    version = run(["rustup", "run", "nightly", "rustc", "-vV"], env=env)
    run(["rustup", "run", "nightly", "cargo", "miri", "--version"], env=env)
    targets = [
        (["-p", "neocogi-com-core", "--lib", "--features", "std"], "abi::array_tests::array_signature_matches_known_sha256_and_distinguishes_shapes"),
        (["-p", "neocogi-com-runtime", "--lib"], "loader::plan::tests::orders_dependencies_before_dependents_deterministically"),
        (["-p", "neocogi-com", "--lib", "--features", "runtime"], (
            "tests::metadata_identity_query_checks_and_releases_a_concrete_projection",
            "object::tests::lease_release_does_not_keep_a_protected_value_reference",
        )),
    ]
    targets.extend((["-p", "neocogi-com-tests", "--features", "runtime", "--test", name],
                    (sentinel, "thread_safe_owner_drop_waits_for_admitted_readers_and_writers") if name == "exclusive_com" else sentinel) for name, sentinel in (
        ("local_com", "local_and_foreign_calls_share_one_gate_and_one_model"),
        ("com_main", "copied_access::copied_access_outlives_the_outer_record_and_shares_state_across_threads"),
        ("borrowed_values", "owned_message_outlives_producer_guard_and_source"),
        # Name required targets explicitly: reordering the native table must
        # not silently substitute a different Miri workload after a migration.
        *((name, ABI_TESTS[name]) for name in ("client_views", "checked_values", "output_buffers", "output_completion", "exclusive_com", "weak_com")),
    ))
    # Large budget boundaries run in linked/ASan/TSan suites. The handle-name
    # inventory audit parses Rust implementations with syn and runs in native
    # unit tests; interpreting that compiler-policy work does not exercise COM
    # memory access. Miri retains all ownership/raw-storage cases and the small
    # checked allocation-fault and budget tests.
    native_only_tests = (
        "buffer_count_includes_each_nested_record",
        "buffers_count_selected_variants_without_charging_larger_alternatives",
        "maximum_variant_cost_is_admitted_before_dispatch_on_both_sides",
        "provider_validates_all_converted_outputs_before_publishing_any",
        "method_names::tests::every_public_handle_receiver_is_reserved",
    )
    harness = [argument for name in native_only_tests for argument in ("--skip", name)]
    executed = []
    with tempfile.TemporaryDirectory(prefix="neocogi-com-miri-") as directory:
        for args, sentinel in targets:
            command = ["rustup", "run", "nightly", "cargo", "miri", "test", *args, "--locked", "--target-dir", directory, "--"]
            executed.append({"target": args, **execute_tests(
                lambda options: run([*command, *options], env=env), sentinel, tuple(harness))})
        # Miri owns a private cache layout below this directory, not a normal
        # Cargo target root. Drop exactly our temporary directory; cargo miri
        # clean would additionally remove the user's shared Miri sysroot cache.
    return {"suite": "safety", "compiler": version, "status": "passed", "targets": len(targets), "execution": executed}


def execute(case: Case, *, linked: bool = False, sanitizer: str | None = None) -> dict:
    """Build independent providers and their host, remove targets, then run relocated."""
    print(json.dumps({"case": asdict(case), "sanitizer": sanitizer}), flush=True)
    env = clean_environment()
    if not (ROOT / "Cargo.lock").is_file():
        # The source-only fixture starts with no lock; resolution happens once,
        # using an explicitly selected compiler, before all locked role builds.
        run(["rustup", "run", case.host.toolchain, "cargo", "generate-lockfile"], env=env)
    versions = {name: run(["rustup", "run", name, "rustc", "-vV"], env=env)
                for name in sorted({case.host.toolchain, case.plugin.toolchain})}
    # Different target ABIs are not interchangeable build flags. Reject the case
    # rather than leaving this implicit in whichever compiler fails first.
    triples = {version.split("host: ")[1].splitlines()[0] for version in versions.values()}
    if len(triples) != 1:
        raise RuntimeError("host/provider compilers must target the same native C ABI")
    with tempfile.TemporaryDirectory(prefix="neocogi-com-compat-") as directory:
        work = Path(directory)
        stage = work / "relocated"
        stage.mkdir()
        executable = "neocogi-example-com" + (".exe" if os.name == "nt" else "")
        for role, build, arguments, filename in (
            ("plugin", case.plugin, ["rustc", "-p", "neocogi-example-com-plugin", "--lib", "--crate-type", "cdylib", "--features", "dynamic-plugin"], library("nc_example_com")),
            ("peer", case.plugin, ["rustc", "--manifest-path", str(ROOT / "tests/com/integration-tests/fixtures/registry-peer/Cargo.toml"), "--lib", "--crate-type", "cdylib", "--features", "dynamic-plugin"], library("nc_registry_peer")),
            ("host", case.host, ["build", "-p", "neocogi-example-com-host", "--bin", "neocogi-example-com"], executable),
        ):
            target = work / role
            cargo(build, target, arguments, env, next(iter(triples)), sanitizer)
            output = target / next(iter(triples)) / ("debug" if build.profile == "dev" else build.profile)
            shutil.copy2(require_file(output / filename), stage / filename)
            # Nothing in subsequent execution may resolve back into build outputs.
            run(["rustup", "run", build.toolchain, "cargo", "clean", "--target-dir", str(target)], env=env)
        runtime_env = dict(env)
        if sanitizer:
            # Plugin mappings and the process-global runtime intentionally remain pinned.
            runtime_env["ASAN_OPTIONS"] = "detect_leaks=0"
            runtime_env["LSAN_OPTIONS"] = "detect_leaks=0"
        groups = {"independent_sample": "pending"}
        if linked or sanitizer:
            # ABI tests/library owners are ordinary Rust assemblies, independent
            # of the shared fixture. Instrumentation retains its established
            # pinned-library leak policy without claiming native imports here.
            ordinary_env = {**env, **({"ASAN_OPTIONS": "detect_leaks=0", "LSAN_OPTIONS": "detect_leaks=0"} if sanitizer else {})}
            ordinary_evidence = ordinary_tests(case.host, work / "tests", ordinary_env, next(iter(triples)), sanitizer)
            run(["rustup", "run", case.host.toolchain, "cargo", "clean", "--target-dir", str(work / "tests")], env=env)
            groups["ordinary_abi_and_embedded"] = ordinary_evidence
            manifest = ROOT / "tests/com/integration-tests/fixtures/host-runtime/Cargo.toml"
            require_file(manifest.with_name("Cargo.lock"))
            output = cargo(case.host, work / "tests", ["test", "--manifest-path", str(manifest), "--tests", "--no-run", "--message-format=json"],
                           env, next(iter(triples)), sanitizer)
            qualification = work / "qualification"
            qualification.mkdir()
            artifacts = {}
            for name, original in qualification_artifacts(output, manifest).items():
                artifacts[name] = qualification / original.name
                shutil.copy2(original, artifacts[name])
            run(["rustup", "run", case.host.toolchain, "cargo", "clean", "--target-dir", str(work / "tests")], env=env)
        if linked or sanitizer:
            groups["host_runtime_regressions"] = qualification_tests(artifacts, runtime_env, qualification)
        expected = {executable, library("nc_example_com"), library("nc_registry_peer")}
        if {path.name for path in stage.iterdir()} != expected:
            raise RuntimeError("deployment must contain exactly the host and two runtime-free providers")
        output = run([str(stage / executable), str(stage / library("nc_example_com")), str(stage / library("nc_registry_peer"))], env=runtime_env, cwd=stage)
        if output.strip() != SUCCESS:
            raise RuntimeError("host did not report completion of its behavioral checks")
        groups["independent_sample"] = "passed"
    return {"case": asdict(case), "compilers": versions, "sanitizer": sanitizer, "groups": groups, "status": "passed"}


def main() -> None:
    """Select only maintained cases and emit a report only after every requested case passes."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--suite", choices=("cross-version", "build-flags", "linked", "safety", "address", "thread", "sanitizers"), default="cross-version")
    parser.add_argument("--report", type=Path, help="write exact compiler/build evidence outside temporary targets")
    args = parser.parse_args()
    match args.suite:
        case "safety": selected = ()
        case "linked": selected = CASES[:1]
        case "cross-version": selected = CASES[1:3]
        case "build-flags": selected = CASES[1:]
        case "address" | "thread" | "sanitizers":
            names = ("address", "thread") if args.suite == "sanitizers" else (args.suite,)
            selected = tuple(Case(name, Build("nightly"), Build("nightly")) for name in names)
    results = []
    def report(status: str, error: str | None = None) -> None:
        """Overwrite old evidence before starting; interrupted/failed work must not look green."""
        if args.report:
            args.report.parent.mkdir(parents=True, exist_ok=True)
            args.report.write_text(json.dumps({"suite": args.suite, "status": status, "cases": results, "error": error}, indent=2) + "\n")
    report("running")
    try:
        if args.suite == "safety":
            results.append(safety())
        for case in selected:
            results.append(execute(case, linked=args.suite == "linked", sanitizer=case.name if case.name in ("address", "thread") else None))
            report("running")
    except Exception as error:
        report("failed", str(error))
        raise
    report("passed")
    print(f"Compatibility checks passed: {len(results)} executed case(s)")


if __name__ == "__main__":
    from library_workspace import library_workspace
    # Independent stable/nightly roles own their sources and embedded test paths
    # together; no binaries outlive the temporary source workspace.
    with library_workspace() as sources:
        ROOT = sources
        main()
