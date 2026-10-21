#!/usr/bin/env python3
"""Execute independently selected COM builds; successful compilation alone never passes a case.

Only Python's standard library and installed Rust toolchains are required. Targets
are temporary, cleaned after each artifact is copied, and absent when the staged
host runs. Shell/PowerShell/CI entry points delegate build policy to this file.
"""
from __future__ import annotations

import argparse
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
ROOT = Path(__file__).resolve().parents[3]
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
    """Three independently built roles that must communicate through one declared C ABI."""
    name: str  # Stable report/CLI identity; unknown names fail before any build.
    runtime: Build  # Exactly one shared process runtime is staged.
    host: Build  # Owns the application queue and executes all behavioral assertions.
    plugin: Build  # Owns the object implementation and its allocator-local destruction.


# A small deliberate matrix replaces duplicated same-toolchain/profile loops.
CASES = (
    Case("linked", Build("stable"), Build("stable"), Build("stable")),
    Case("stable-host", Build("stable"), Build("stable"), Build("nightly", "release", "abort", "fat", True)),
    Case("nightly-host", Build("nightly"), Build("nightly", "release", "abort", "thin", True), Build("stable")),
    Case("stable-release", Build("stable", "release", "abort", "fat"), Build("stable", "release", "unwind", "thin"), Build("stable", "release", "abort", "fat")),
)
# Keep semantic sentinels next to their actual target, not stale names in several scripts.
RUNTIME_TESTS = {
    "factory_plugin_runtime": "generated_contract_factory_acquires_without_a_class_table",
    "object_persistence": "registry_ownership_is_local_and_releases_retained_factories",
}
# These targets exercise generated calls and ownership without native runtime imports.
ABI_TESTS = {
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
            "NC_COM_RUNTIME_LINK_DIR", "LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH", "DYLD_FALLBACK_LIBRARY_PATH",
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


def cargo(build: Build, target: Path, arguments: list[str], env: dict[str, str], native_target: str, sanitizer: str | None = None, harness: tuple[str, ...] = ()) -> str:
    """Apply one build policy to the final artifact and all its Rust dependencies."""
    flags = ["-Zrandomize-layout", "-Zlayout-seed=107"] if build.randomize else []
    extra = ["--target", native_target]
    if sanitizer:
        # Instrument std and all three artifacts consistently; sanitizer runs do
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


def ordinary_tests(build: Build, target: Path, env: dict[str, str], native_target: str, sanitizer: str | None) -> None:
    """Execute runtime-free ABI cases and explicitly embedded library owners.

    These are valuable regressions, but their test counts do not prove shared
    linkage. They receive no staged-runtime loader or link-directory overrides.
    """
    for target_name, sentinel in ABI_TESTS.items():
        args = ["test", "-p", "neocogi-com", "--features", "runtime", "--test", target_name]
        require_tests(cargo(build, target, args, env, native_target, sanitizer, ("--list",)), sentinel)
        require_execution(cargo(build, target, args, env, native_target, sanitizer))
    if sanitizer:
        # Preserve pure library coverage without the retired multi-plugin examples.
        for package in ("neocogi-com-core", "neocogi-com-runtime", "neocogi-com"):
            args = ["test", "-p", package, "--lib"]
            match package:
                case "neocogi-com-core": args += ["--features", "std"]
                case "neocogi-com": args += ["--features", "runtime"]
            require_tests(cargo(build, target, args, env, native_target, sanitizer, ("--list",)))
            require_execution(cargo(build, target, args, env, native_target, sanitizer))
    else:
        # Execute the C fixture directly so its target is owned and cleaned here.
        # It checks native C layout; it does not instrument the C compiler itself.
        manifest = ROOT / "crates/com/api/integration-tests/fixtures/native-c-abi/Cargo.toml"
        args = ["test", "--manifest-path", str(manifest)]
        require_tests(cargo(build, target, args, env, native_target, harness=("--list",)))
        require_execution(cargo(build, target, args, env, native_target))


def shared_artifacts(output: str, manifest: Path) -> dict[str, Path]:
    """Admit the two actual Cargo test artifacts and exclude an embedded runtime.

    Cached compiler-artifact records count just as fresh ones: admitting only
    records rebuilt this time would hide a previously compiled runtime rlib.
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
        if target["name"] == "nc_com":
            raise RuntimeError("shared qualification contains the nc_com runtime implementation")
        if not record.get("executable"):
            continue
        name = target["name"]
        if (Path(record["manifest_path"]).resolve() != manifest.resolve()
                or name not in RUNTIME_TESTS or target["kind"] != ["test"]
                or not record["profile"]["test"]
                or Path(target["src_path"]).resolve() != (manifest.parent / f"../../support/{name}_cases.rs").resolve()
                or name in artifacts):
            raise RuntimeError(f"unexpected shared qualification executable: {name}")
        artifacts[name] = require_file(Path(record["executable"]))
    if artifacts.keys() != RUNTIME_TESTS.keys():
        raise RuntimeError("missing shared qualification test artifacts")
    if len(set(artifacts.values())) != len(artifacts):
        raise RuntimeError("shared qualification executables must be distinct")
    return artifacts


def require_runtime_dependency(executable: Path, runtime: Path, env: dict[str, str], cwd: Path) -> None:
    """Withhold only the staged runtime and reject discovery from the same binary.

    Positive discovery must have happened first. Always restore the runtime,
    including on process launch errors; never launch another Cargo build here.
    A failed process that still lists tests does not prove a loader dependency.
    """
    require_file(executable)
    require_file(runtime)
    withheld = runtime.parent.parent / ("withheld-" + runtime.name)
    if withheld.exists():
        raise RuntimeError(f"runtime withholding path already exists: {withheld}")
    runtime.rename(withheld)
    try:
        arguments = [str(executable), "--list"]
        print("+ (runtime withheld)", subprocess.list2cmdline(arguments), flush=True)
        process = subprocess.run(arguments, env=env, cwd=cwd, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        print(process.stdout, end="", flush=True)
        if process.returncode == 0 or any(line.endswith(": test") for line in process.stdout.splitlines()):
            raise RuntimeError("shared qualification still discovers tests without the staged runtime")
    finally:
        withheld.rename(runtime)


def shared_tests(artifacts: dict[str, Path], runtime: Path, env: dict[str, str], cwd: Path) -> dict:
    """Execute complete relocated suites, then prove the same bytes require nc_com."""
    evidence = {}
    for name, executable in artifacts.items():
        digest = hashlib.sha256(require_file(executable).read_bytes()).hexdigest()
        require_tests(run([str(executable), "--list"], env=env, cwd=cwd), RUNTIME_TESTS[name])
        # No sentinel filter: every maintained case, including raw persistence,
        # executes against the shared implementation in a single-threaded process.
        require_execution(run([str(executable), "--test-threads=1"], env=env, cwd=cwd))
        require_runtime_dependency(executable, runtime, env, cwd)
        if hashlib.sha256(require_file(executable).read_bytes()).hexdigest() != digest:
            raise RuntimeError(f"qualification executable changed during execution: {name}")
        evidence[name] = {"sha256": digest, "runtime_withheld": "discovery failed"}
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
        (["-p", "neocogi-com", "--lib", "--features", "runtime"], "runtime::tests::table_rejects_a_short_allocation_before_forming_a_full_reference"),
    ]
    targets.extend((["-p", "neocogi-com", "--features", "runtime", "--test", name], sentinel) for name, sentinel in (
        ("local_com", "local_and_foreign_calls_share_one_gate_and_one_model"),
        ("borrowed_values", "owned_message_outlives_producer_guard_and_source"),
        # Name required targets explicitly: reordering the native table must
        # not silently substitute a different Miri workload after a migration.
        *((name, ABI_TESTS[name]) for name in ("checked_values", "output_buffers", "output_completion", "exclusive_com", "weak_com")),
    ))
    # These tests traverse hundreds of thousands of values at the public budget
    # boundary. They run in linked/ASan/TSan suites; Miri covers allocation faults,
    # ownership completion, and malformed fields using the small checked cases.
    native_budget_tests = (
        "buffer_count_includes_each_nested_record",
        "buffers_count_selected_variants_without_charging_larger_alternatives",
        "maximum_variant_cost_is_admitted_before_dispatch_on_both_sides",
        "provider_validates_all_converted_outputs_before_publishing_any",
    )
    harness = [argument for name in native_budget_tests for argument in ("--skip", name)]
    with tempfile.TemporaryDirectory(prefix="neocogi-com-miri-") as directory:
        for args, sentinel in targets:
            command = ["rustup", "run", "nightly", "cargo", "miri", "test", *args, "--locked", "--target-dir", directory, "--", *harness]
            require_tests(run([*command, "--list"], env=env), sentinel)
            require_execution(run(command, env=env))
        # Miri owns a private cache layout below this directory, not a normal
        # Cargo target root. Drop exactly our temporary directory; cargo miri
        # clean would additionally remove the user's shared Miri sysroot cache.
    return {"suite": "safety", "compiler": version, "status": "passed", "targets": len(targets)}


def execute(case: Case, *, linked: bool = False, sanitizer: str | None = None) -> dict:
    """Build, remove build outputs, then execute a three-file relocated deployment."""
    print(json.dumps({"case": asdict(case), "sanitizer": sanitizer}), flush=True)
    env = clean_environment()
    if not (ROOT / "Cargo.lock").is_file():
        # The source-only fixture starts with no lock; resolution happens once,
        # using an explicitly selected compiler, before all locked role builds.
        run(["rustup", "run", case.runtime.toolchain, "cargo", "generate-lockfile"], env=env)
    versions = {name: run(["rustup", "run", name, "rustc", "-vV"], env=env)
                for name in sorted({case.runtime.toolchain, case.host.toolchain, case.plugin.toolchain})}
    # Different target ABIs are not interchangeable build flags. Reject the case
    # rather than leaving this implicit in whichever compiler fails first.
    triples = {version.split("host: ")[1].splitlines()[0] for version in versions.values()}
    if len(triples) != 1:
        raise RuntimeError("host/runtime/plugin compilers must target the same native C ABI")
    with tempfile.TemporaryDirectory(prefix="neocogi-com-compat-") as directory:
        work = Path(directory)
        stage, link = work / "relocated", work / "link"
        stage.mkdir()
        link.mkdir()
        executable = "neocogi-example-com" + (".exe" if os.name == "nt" else "")
        for role, build, arguments, filename in (
            ("runtime", case.runtime, ["rustc", "-p", "neocogi-com-runtime", "--lib", "--crate-type", "cdylib"], library("nc_com")),
            ("plugin", case.plugin, ["rustc", "-p", "neocogi-example-com-plugin", "--lib", "--crate-type", "cdylib", "--features", "dynamic-plugin"], library("nc_example_com")),
            ("host", case.host, ["build", "-p", "neocogi-example-com-plugin", "--bin", "neocogi-example-com", "--features", "host"], executable),
        ):
            target = work / role
            cargo(build, target, arguments, {**env, "NC_COM_RUNTIME_LINK_DIR": str(link)}, next(iter(triples)), sanitizer)
            output = target / next(iter(triples)) / ("debug" if build.profile == "dev" else build.profile)
            shutil.copy2(require_file(output / filename), stage / filename)
            if role == "runtime":
                shutil.copy2(stage / filename, link / filename)
                if os.name == "nt":
                    # The facade imports Rust's MSVC nc_com.dll.lib name verbatim.
                    imports = next((p for p in (output / "nc_com.dll.lib", output / "nc_com.lib", output / "libnc_com.dll.a") if p.is_file()), None)
                    if imports is None:
                        raise RuntimeError("runtime import library was not produced")
                    shutil.copy2(imports, link / ("nc_com.dll.lib" if imports.suffix == ".lib" else imports.name))
            # Nothing in subsequent execution may resolve back into build outputs.
            run(["rustup", "run", build.toolchain, "cargo", "clean", "--target-dir", str(target)], env=env)
        runtime_env = {**env, "LD_LIBRARY_PATH": str(stage), "DYLD_LIBRARY_PATH": str(stage),
                       "PATH": str(stage) + os.pathsep + env.get("PATH", "")}
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
            ordinary_tests(case.host, work / "tests", ordinary_env, next(iter(triples)), sanitizer)
            run(["rustup", "run", case.host.toolchain, "cargo", "clean", "--target-dir", str(work / "tests")], env=env)
            groups["ordinary_abi_and_embedded"] = "passed"
            manifest = ROOT / "crates/com/api/integration-tests/fixtures/shared-runtime/Cargo.toml"
            require_file(manifest.with_name("Cargo.lock"))
            output = cargo(case.host, work / "tests", ["test", "--manifest-path", str(manifest), "--tests", "--no-run", "--message-format=json"],
                           {**env, "NC_COM_RUNTIME_LINK_DIR": str(link)}, next(iter(triples)), sanitizer)
            qualification = work / "qualification"
            qualification.mkdir()
            artifacts = {}
            for name, original in shared_artifacts(output, manifest).items():
                artifacts[name] = qualification / original.name
                shutil.copy2(original, artifacts[name])
            run(["rustup", "run", case.host.toolchain, "cargo", "clean", "--target-dir", str(work / "tests")], env=env)
        shutil.rmtree(link)
        if linked or sanitizer:
            groups["shared_runtime_regressions"] = shared_tests(artifacts, stage / library("nc_com"), runtime_env, qualification)
        expected = {executable, library("nc_com"), library("nc_example_com")}
        if {path.name for path in stage.iterdir()} != expected:
            raise RuntimeError("deployment must contain exactly host, runtime and plugin")
        output = run([str(stage / executable), str(stage / library("nc_example_com"))], env=runtime_env, cwd=stage)
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
            selected = tuple(Case(name, Build("nightly"), Build("nightly"), Build("nightly")) for name in names)
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
    # Independent stable/nightly roles use the library source boundary, not the
    # development workspace's nightly artifact dependency declarations.
    with library_workspace() as sources:
        ROOT = sources
        main()
