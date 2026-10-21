"""Driver regressions reject the false evidence patterns that motivated this repair.

Real compilers and relocated execution are exercised by compatibility.py itself;
these tests check driver failures without expensive duplicate Cargo builds.
"""
from contextlib import redirect_stdout
import io
import json
import os
import copy
from pathlib import Path
import sys
import tempfile
import subprocess
import unittest
from unittest.mock import patch

import compatibility as driver


class AdmissionTests(unittest.TestCase):
    """Missing tests/artifacts and ambient configuration cannot count as coverage."""

    def test_child_failure_cannot_be_hidden_by_a_success_marker(self):
        """The process exit is authoritative even when stdout contains the expected marker."""
        with redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(RuntimeError, "command exited 7"):
                driver.run([sys.executable, "-c", f"print({driver.SUCCESS!r}); raise SystemExit(7)"], env=driver.clean_environment())
            output = driver.run([sys.executable, "-c", "print('required: test')"], env=driver.clean_environment())
        self.assertEqual(output, "required: test\n")

    def test_discovery_requires_an_exact_name_and_nonempty_listing(self):
        """The renamed dynamic test and arbitrary substring matches must fail closed."""
        for listing in ("", "0 tests, 0 benchmarks", "prefix_required: test", "required: benchmark"):
            with self.assertRaises(RuntimeError):
                driver.require_tests(listing, "required")
        driver.require_tests("required: test\nother: test\n", "required")
        with self.assertRaises(RuntimeError):
            driver.require_tests("0 tests")

    def test_zero_execution_is_not_success(self):
        """A successful Cargo exit with every test filtered/ignored proves nothing."""
        for output in ("", "test result: ok. 0 passed; 0 failed; 9 ignored;", "test result: FAILED. 0 passed; 1 failed;"):
            with self.assertRaises(RuntimeError):
                driver.require_execution(output)
        driver.require_execution("test result: ok. 3 passed; 0 failed; 0 ignored;")

    def test_expected_artifact_must_exist_and_contain_bytes(self):
        """Do not discover an unrelated DSO elsewhere when the requested build produced none."""
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "runtime.so"
            with self.assertRaises(RuntimeError):
                driver.require_file(path)
            path.touch()
            with self.assertRaises(RuntimeError):
                driver.require_file(path)
            path.write_bytes(b"artifact")
            self.assertEqual(driver.require_file(path), path)

    def test_shared_artifacts_admit_exact_fresh_and_cached_case_roots(self):
        """Cargo supplies paths; both cache states must establish the same assembly."""
        with tempfile.TemporaryDirectory() as directory:
            manifest, records = self.shared_records(Path(directory))
            expected = {row["target"]["name"]: Path(row["executable"]) for row in records}
            for fresh in (False, True):
                for record in records:
                    record["fresh"] = fresh
                output = "compiler progress\n" + "\n".join(map(json.dumps, records))
                self.assertEqual(driver.shared_artifacts(output, manifest), expected)

    @staticmethod
    def shared_records(root):
        """Two nonempty executables with Cargo's actual manifest/source identity fields."""
        manifest = root / "fixtures/shared-runtime/Cargo.toml"
        manifest.parent.mkdir(parents=True)
        manifest.write_text("[package]")
        records = []
        for name in driver.RUNTIME_TESTS:
            executable = root / (name + ".exe")
            executable.write_bytes(name.encode())
            records.append({"reason": "compiler-artifact", "manifest_path": str(manifest),
                            "target": {"name": name, "kind": ["test"],
                                       "src_path": str(manifest.parent / f"../../support/{name}_cases.rs")},
                            "profile": {"test": True}, "executable": str(executable), "fresh": False})
        return manifest, records

    def test_shared_admission_rejects_incomplete_or_foreign_executables(self):
        """Names/counts cannot stand in for package, kind, maintained roots or actual files."""
        with tempfile.TemporaryDirectory() as directory:
            manifest, records = self.shared_records(Path(directory))
            invalid = [records[:1], records + records[:1]]
            for field, value in (("manifest_path", str(manifest.with_name("foreign.toml"))),
                                 ("executable", str(manifest.with_name("missing.exe")))):
                changed = copy.deepcopy(records)
                changed[0][field] = value
                invalid.append(changed)
            for field, value in (("name", "renamed"), ("kind", ["bin"]), ("src_path", str(manifest))):
                changed = copy.deepcopy(records)
                changed[0]["target"][field] = value
                invalid.append(changed)
            changed = copy.deepcopy(records)
            changed[0]["profile"]["test"] = False
            invalid.append(changed)
            changed = copy.deepcopy(records)
            changed[0]["executable"] = changed[1]["executable"]
            invalid.append(changed)
            for rows in invalid:
                with self.subTest(rows=rows), self.assertRaises(RuntimeError):
                    driver.shared_artifacts("\n".join(map(json.dumps, rows)), manifest)

    def test_shared_admission_rejects_cached_runtime_with_matching_names(self):
        """Adding an implementation dependency must fail even if both test artifacts match."""
        with tempfile.TemporaryDirectory() as directory:
            manifest, records = self.shared_records(Path(directory))
            for fresh in (False, True):
                runtime = {"reason": "compiler-artifact", "target": {"name": "nc_com"},
                           "fresh": fresh, "executable": None}
                with self.assertRaisesRegex(RuntimeError, "runtime implementation"):
                    driver.shared_artifacts("\n".join(map(json.dumps, [*records, runtime])), manifest)

    def test_runtime_withholding_rejects_discovery_and_always_restores(self):
        """Failure after listing or launch failure cannot leave the staged runtime removed."""
        with tempfile.TemporaryDirectory() as directory, redirect_stdout(io.StringIO()):
            root = Path(directory)
            stage = root / "stage"
            stage.mkdir()
            runtime, executable = stage / "nc_com", root / "test.exe"
            runtime.write_bytes(b"runtime")
            executable.write_bytes(b"test")
            for status, output, valid in ((127, "loader failed", True), (0, "", False),
                                          (1, "sentinel: test\n", False)):
                with patch.object(driver.subprocess, "run", return_value=subprocess.CompletedProcess([], status, output)):
                    if valid:
                        driver.require_runtime_dependency(executable, runtime, {}, root)
                    else:
                        with self.assertRaises(RuntimeError):
                            driver.require_runtime_dependency(executable, runtime, {}, root)
                self.assertEqual(runtime.read_bytes(), b"runtime")
                self.assertFalse((root / "withheld-nc_com").exists())
            with patch.object(driver.subprocess, "run", side_effect=OSError("launch failure")):
                with self.assertRaises(OSError):
                    driver.require_runtime_dependency(executable, runtime, {}, root)
            self.assertEqual(runtime.read_bytes(), b"runtime")

    def test_shared_execution_runs_complete_suites_before_negative_control(self):
        """Discovery is followed by an unfiltered execution, never a sentinel-only pass."""
        with tempfile.TemporaryDirectory() as directory:
            manifest, records = self.shared_records(Path(directory))
            artifacts = driver.shared_artifacts("\n".join(map(json.dumps, records)), manifest)
            outputs = []
            for name in artifacts:
                outputs.extend([driver.RUNTIME_TESTS[name] + ": test\n", "test result: ok. 4 passed; 0 failed;"])
            with patch.object(driver, "run", side_effect=outputs) as run, patch.object(driver, "require_runtime_dependency") as negative:
                evidence = driver.shared_tests(artifacts, Path("runtime"), {}, manifest.parent)
            self.assertEqual(evidence.keys(), artifacts.keys())
            self.assertEqual(negative.call_count, 2)
            self.assertEqual([call.args[0][1:] for call in run.call_args_list],
                             [["--list"], ["--test-threads=1"], ["--list"], ["--test-threads=1"]])
            for output in ("0 tests", "test result: ok. 0 passed; 0 failed;"):
                with patch.object(driver, "run", return_value=output), self.assertRaises(RuntimeError):
                    driver.shared_tests(artifacts, Path("runtime"), {}, manifest.parent)

    def test_ordinary_abi_cases_do_not_enable_native_runtime_clients(self):
        """Sanitizer exports must not turn a pure test into an unresolved native importer."""
        outputs = []
        for sentinel in driver.ABI_TESTS.values():
            outputs.extend([sentinel + ": test\n", "test result: ok. 1 passed; 0 failed;"])
        outputs.extend(["native_layout: test\n", "test result: ok. 1 passed; 0 failed;"])
        with patch.object(driver, "cargo", side_effect=outputs) as cargo:
            driver.ordinary_tests(driver.Build("stable"), Path("target"), {}, "x86_64-unknown-linux-gnu", None)
        for call in cargo.call_args_list[:2 * len(driver.ABI_TESTS)]:
            self.assertNotIn("--features", call.args[2])
            self.assertEqual(call.args[3], {})

    def test_ambient_compiler_flags_profiles_and_loader_paths_are_removed(self):
        """The build case owns these settings; an invoking Cargo process cannot select them."""
        overrides = {name: "stale" for name in (
            "CARGO", "RUSTC", "RUSTUP_TOOLCHAIN", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_TARGET_DIR", "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS",
            "CARGO_PROFILE_RELEASE_PANIC", "CARGO_BUILD_TARGET", "CARGO_BUILD_RUSTC", "NC_COM_RUNTIME_LINK_DIR",
            "LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH", "MIRIFLAGS", "ASAN_OPTIONS", "TSAN_OPTIONS",
        )}
        with patch.dict(os.environ, {**overrides, "PATH": "tool-search"}, clear=True):
            self.assertEqual(driver.clean_environment(), {"PATH": "tool-search"})

    def test_each_role_has_explicit_compiler_profile_target_and_flags(self):
        """No shared build environment can accidentally make two intended roles identical."""
        build = driver.Build("nightly", "release", "abort", "fat", True)
        with patch.object(driver, "run", return_value="") as run:
            driver.cargo(build, Path("target with spaces"), ["build", "--lib"], {}, "x86_64-unknown-linux-gnu")
        args = run.call_args.args[0]
        env = run.call_args.kwargs["env"]
        self.assertEqual(args[:4], ["rustup", "run", "nightly", "cargo"])
        self.assertEqual(args[args.index("--target-dir") + 1], "target with spaces")
        self.assertEqual(args[args.index("--profile") + 1], "release")
        self.assertEqual(args[args.index("--target") + 1], "x86_64-unknown-linux-gnu")
        self.assertEqual(env["CARGO_PROFILE_RELEASE_PANIC"], "abort")
        self.assertEqual(env["CARGO_PROFILE_RELEASE_LTO"], "fat")
        self.assertIn("-Zrandomize-layout", env["CARGO_ENCODED_RUSTFLAGS"])

    def test_sanitized_discovery_keeps_cargo_options_before_harness_options(self):
        """Instrumentation applies to the build, not accidentally to the test filter."""
        with patch.object(driver, "run", return_value="") as run:
            driver.cargo(driver.Build("nightly"), Path("target"), ["test", "--lib"], {},
                         "x86_64-unknown-linux-gnu", "thread", ("--list",))
        args = run.call_args.args[0]
        self.assertEqual(args[-2:], ["--", "--list"])
        self.assertLess(args.index("-Zbuild-std"), args.index("--"))
        flags = run.call_args.kwargs["env"]["CARGO_ENCODED_RUSTFLAGS"]
        self.assertIn("-Zsanitizer=thread", flags)
        self.assertIn("-Clink-arg=-Wl,--export-dynamic", flags)
        with self.assertRaises(RuntimeError):
            driver.cargo(driver.Build("nightly"), Path("target"), ["test"], {}, "aarch64-apple-darwin", "thread")

    def test_combined_sanitizers_fail_if_only_the_first_case_passes(self):
        """A successful ASan case cannot conceal failed TSan or omit it from execution."""
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "result.json"
            with patch("sys.argv", ["compatibility.py", "--suite", "sanitizers", "--report", str(report)]):
                with patch.object(driver, "execute", side_effect=[{"status": "passed"}, RuntimeError("thread failure")]) as execute:
                    with self.assertRaises(RuntimeError):
                        driver.main()
            self.assertEqual([call.kwargs["sanitizer"] for call in execute.call_args_list], ["address", "thread"])
            result = json.loads(report.read_text())
            self.assertEqual(result["status"], "failed")
            self.assertEqual(len(result["cases"]), 1)
            self.assertEqual(result["error"], "thread failure")

    def test_failed_run_overwrites_a_previous_green_report(self):
        """A stale success JSON must never survive a failed current invocation."""
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "result.json"
            report.write_text('{"status": "passed"}')
            with patch("sys.argv", ["compatibility.py", "--suite", "linked", "--report", str(report)]):
                with patch.object(driver, "execute", side_effect=RuntimeError("missing artifact")):
                    with self.assertRaises(RuntimeError):
                        driver.main()
            result = json.loads(report.read_text())
            self.assertEqual(result["status"], "failed")
            self.assertEqual(result["cases"], [])
            self.assertEqual(result["error"], "missing artifact")


if __name__ == "__main__":
    unittest.main()
