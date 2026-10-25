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

    def test_required_case_must_execute_before_complete_target_is_admitted(self):
        """Real ignored/failed/excluded semantic cases cannot hide behind helpers."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "cases.rs"
            source.write_text('''
                #[test]
                #[cfg_attr(ignored_required, ignore)]
                fn required_case() { assert!(!cfg!(failing_required)); }
                #[test]
                fn required_case_suffix() {}
                #[test]
                fn helper() { std::fs::write(std::env::var_os("HELPER_MARKER").unwrap(), b"ran").unwrap(); }
                #[test] #[ignore]
                fn unrelated_ignored() {}
            ''')
            for mode, flags in (("ignored", ["--cfg", "ignored_required"]),
                                ("failed", ["--cfg", "failing_required"]), ("active", [])):
                binary = root / (mode + (".exe" if os.name == "nt" else ""))
                built = subprocess.run(["rustc", "--edition=2024", "--test", str(source),
                                        *flags, "-o", str(binary)], capture_output=True, text=True)
                self.assertEqual(built.returncode, 0, built.stderr)
                marker = root / (mode + ".marker")
                env = {**os.environ, "HELPER_MARKER": str(marker)}

                def execute(options):
                    """Use the real process-status and harness-result boundary."""
                    return driver.run([str(binary), *options], env=env)

                with redirect_stdout(io.StringIO()):
                    if mode != "active":
                        with self.assertRaises(RuntimeError):
                            driver.execute_tests(execute, "required_case")
                    else:
                        result = driver.execute_tests(execute, "required_case")
                        self.assertEqual(result["exact"], "passed")
                        self.assertEqual(marker.read_bytes(), b"ran", "complete target must still run helpers")
                        for missing in ("missing_case", "required_cas"):
                            with self.assertRaises(RuntimeError):
                                driver.execute_tests(execute, missing)
                        with self.assertRaises(RuntimeError):
                            driver.execute_tests(execute, "required_case", ("--skip", "required_case"))
                        driver.execute_tests(execute, "required_case", ("--skip", "helper"))
                        driver.execute_tests(execute, ("required_case", "required_case_suffix"))
                        for second in ("missing_case", "unrelated_ignored"):
                            with self.assertRaises(RuntimeError):
                                driver.execute_tests(execute, ("required_case", second))
                        with self.assertRaises(RuntimeError):
                            driver.execute_tests(execute, ("required_case", "helper"), ("--skip", "helper"))

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

    def test_qualification_artifacts_admit_exact_fresh_and_cached_case_roots(self):
        """Cargo supplies paths; both cache states must establish the same assembly."""
        with tempfile.TemporaryDirectory() as directory:
            manifest, records = self.shared_records(Path(directory))
            expected = {row["target"]["name"]: Path(row["executable"]) for row in records}
            for fresh in (False, True):
                for record in records:
                    record["fresh"] = fresh
                output = "compiler progress\n" + "\n".join(map(json.dumps, records))
                self.assertEqual(driver.qualification_artifacts(output, manifest), expected)

    @staticmethod
    def shared_records(root):
        """Two nonempty executables with Cargo's actual manifest/source identity fields."""
        manifest = root / "fixtures/host-runtime/Cargo.toml"
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
                    driver.qualification_artifacts("\n".join(map(json.dumps, rows)), manifest)

    def test_cached_host_implementation_is_ordinary_build_evidence(self):
        """The final host now links nc_com; cached rlib records are expected."""
        with tempfile.TemporaryDirectory() as directory:
            manifest, records = self.shared_records(Path(directory))
            for fresh in (False, True):
                runtime = {"reason": "compiler-artifact", "target": {"name": "nc_com"},
                           "fresh": fresh, "executable": None}
                admitted = driver.qualification_artifacts("\n".join(map(json.dumps, [*records, runtime])), manifest)
                self.assertEqual(admitted.keys(), driver.RUNTIME_TESTS.keys())

    def test_qualification_execution_runs_complete_suites_without_a_runtime_dso(self):
        """Discovery is followed by an unfiltered execution, never a sentinel-only pass."""
        with tempfile.TemporaryDirectory() as directory:
            manifest, records = self.shared_records(Path(directory))
            artifacts = driver.qualification_artifacts("\n".join(map(json.dumps, records)), manifest)
            outputs = []
            for name in artifacts:
                outputs.extend([driver.RUNTIME_TESTS[name] + ": test\n",
                                "test result: ok. 1 passed; 0 failed;", "test result: ok. 4 passed; 0 failed;"])
            with patch.object(driver, "run", side_effect=outputs) as run:
                evidence = driver.qualification_tests(artifacts, {}, manifest.parent)
            self.assertEqual(evidence.keys(), artifacts.keys())
            expected = []
            for name in artifacts:
                expected.extend([["--test-threads=1", "--list"],
                                 ["--test-threads=1", "--exact", driver.RUNTIME_TESTS[name]],
                                 ["--test-threads=1"]])
            self.assertEqual([call.args[0][1:] for call in run.call_args_list], expected)
            for output in ("0 tests", "test result: ok. 0 passed; 0 failed;"):
                with patch.object(driver, "run", return_value=output), self.assertRaises(RuntimeError):
                    driver.qualification_tests(artifacts, {}, manifest.parent)

    def test_ordinary_abi_cases_do_not_enable_native_runtime_clients(self):
        """Sanitizer exports must not turn a pure test into an unresolved native importer."""
        outputs = []
        for sentinel in driver.ABI_TESTS.values():
            outputs.extend([sentinel + ": test\n", "test result: ok. 1 passed; 0 failed;",
                            "test result: ok. 2 passed; 0 failed;"])
        outputs.extend(["native_layout: test\n", "test result: ok. 1 passed; 0 failed;"])
        with patch.object(driver.platform, "system", return_value="Linux"), patch.object(driver, "cargo", side_effect=outputs) as cargo:
            driver.ordinary_tests(driver.Build("stable"), Path("target"), {}, "x86_64-unknown-linux-gnu", None)
        for call in cargo.call_args_list[:3 * len(driver.ABI_TESTS)]:
            self.assertNotIn("--features", call.args[2])
            self.assertEqual(call.args[3], {})

    def test_windows_native_path_case_requires_exact_and_complete_execution(self):
        """The Windows linked route must actually admit its platform regression."""
        sentinel = "embedded_nul_path_is_rejected_before_descriptor_getter"
        outputs = [sentinel + ": test\n", "test result: ok. 1 passed; 0 failed;", "test result: ok. 2 passed; 0 failed;"]
        for required in driver.ABI_TESTS.values():
            outputs.extend([required + ": test\n", "test result: ok. 1 passed; 0 failed;", "test result: ok. 2 passed; 0 failed;"])
        outputs.extend(["native_layout: test\n", "test result: ok. 1 passed; 0 failed;"])
        with patch.object(driver.platform, "system", return_value="Windows"), patch.object(driver, "cargo", side_effect=outputs) as cargo:
            evidence = driver.ordinary_tests(driver.Build("stable"), Path("target"), {}, "x86_64-pc-windows-msvc", None)
        self.assertEqual(evidence["windows_plugin_paths"]["exact"], "passed")
        calls = cargo.call_args_list[:3]
        for call in calls:
            self.assertEqual(call.args[2], ["test", "-p", "neocogi-com-tests", "--features", "runtime", "--test", "windows_plugin_paths"])
        self.assertEqual([call.kwargs["harness"] for call in calls], [("--list",), ("--exact", sentinel), ()])
        with patch.object(driver.platform, "system", return_value="Windows"), patch.object(driver, "cargo", side_effect=[sentinel + ": test\n", "test result: ok. 0 passed; 0 failed; 1 ignored;"]), self.assertRaises(RuntimeError):
            driver.ordinary_tests(driver.Build("stable"), Path("target"), {}, "x86_64-pc-windows-msvc", None)

    def test_ambient_compiler_flags_profiles_and_loader_paths_are_removed(self):
        """The build case owns these settings; an invoking Cargo process cannot select them."""
        overrides = {name: "stale" for name in (
            "CARGO", "RUSTC", "RUSTUP_TOOLCHAIN", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_TARGET_DIR", "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS",
            "CARGO_PROFILE_RELEASE_PANIC", "CARGO_BUILD_TARGET", "CARGO_BUILD_RUSTC",
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

    def test_miri_keeps_memory_targets_and_limits_native_only_filters(self):
        """Source-policy parsing stays native while Miri discovers and executes every memory target."""
        libraries = {
            "neocogi-com-core": "abi::array_tests::array_signature_matches_known_sha256_and_distinguishes_shapes",
            "neocogi-com-runtime": "loader::plan::tests::orders_dependencies_before_dependents_deterministically",
            "neocogi-com": ("tests::metadata_identity_query_checks_and_releases_a_concrete_projection",
                            "object::tests::lease_release_does_not_keep_a_protected_value_reference"),
        }
        interfaces = {
            "local_com": "local_and_foreign_calls_share_one_gate_and_one_model",
            "com_main": "copied_access::copied_access_outlives_the_outer_record_and_shares_state_across_threads",
            "borrowed_values": "owned_message_outlives_producer_guard_and_source",
            **{name: driver.ABI_TESTS[name] for name in ("client_views", "checked_values", "output_buffers", "output_completion", "exclusive_com", "weak_com")},
        }
        interfaces["exclusive_com"] = (interfaces["exclusive_com"], "thread_safe_owner_drop_waits_for_admitted_readers_and_writers")

        def output(arguments, **_):
            """Supply genuine harness shapes so the normal admission checks remain active."""
            if "miri" not in arguments or "test" not in arguments:
                return "selected nightly compiler\n"
            if "--list" not in arguments:
                return "test result: ok. 1 passed; 0 failed;\n"
            if "--test" in arguments:
                sentinel = interfaces[arguments[arguments.index("--test") + 1]]
            else:
                sentinel = libraries[arguments[arguments.index("-p") + 1]]
            names = (sentinel,) if isinstance(sentinel, str) else sentinel
            return "".join(name + ": test\n" for name in names)

        with patch.object(driver, "run", side_effect=output) as run:
            evidence = driver.safety()
        commands = [call.args[0] for call in run.call_args_list if "miri" in call.args[0] and "test" in call.args[0]]
        self.assertEqual(evidence["targets"], len(libraries) + len(interfaces))
        self.assertEqual(len(commands), 3 * evidence["targets"] + 2)
        exact = [command[command.index("--exact") + 1] for command in commands if "--exact" in command]
        self.assertIn("object::tests::lease_release_does_not_keep_a_protected_value_reference", exact)
        self.assertIn("thread_safe_owner_drop_waits_for_admitted_readers_and_writers", exact)
        expected_skips = {
            "buffer_count_includes_each_nested_record",
            "buffers_count_selected_variants_without_charging_larger_alternatives",
            "maximum_variant_cost_is_admitted_before_dispatch_on_both_sides",
            "provider_validates_all_converted_outputs_before_publishing_any",
            "method_names::tests::every_public_handle_receiver_is_reserved",
        }
        for command in commands:
            skipped = {command[index + 1] for index, argument in enumerate(command) if argument == "--skip"}
            self.assertEqual(skipped, expected_skips)

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
