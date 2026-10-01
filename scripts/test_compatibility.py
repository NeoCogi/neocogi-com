"""Driver regressions reject the false evidence patterns that motivated this repair.

Real compilers and relocated execution are exercised by compatibility.py itself;
these tests check driver failures without expensive duplicate Cargo builds.
"""
from contextlib import redirect_stdout
import io
import json
import os
from pathlib import Path
import sys
import tempfile
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
