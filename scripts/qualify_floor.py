#!/usr/bin/env python3
"""Execute semantic integration regressions under the supported compiler floor."""

import json
import os
from pathlib import Path
import sys

from compatibility import ABI_TESTS, execute_tests, run


def main() -> None:
    """Retain the active floor toolchain in every nested compiler-spawning test."""
    toolchain, target = sys.argv[1:]
    env = dict(os.environ, RUSTUP_TOOLCHAIN=toolchain, CARGO_TARGET_DIR=str(Path(target).resolve()))
    version = run(["rustup", "run", toolchain, "rustc", "-vV"], env=env)
    rows = [
        ("neocogi-com-tests", "client_views", ABI_TESTS["client_views"]),
        ("neocogi-com-tests", "client_view_admission", "client_views_cannot_replace_capability_storage"),
        *(("neocogi-com-tests", name, ABI_TESTS[name]) for name in ("checked_values", "output_buffers", "output_completion")),
        ("neocogi-idl", "unit_values", "unit_values_use_checked_storage_in_copied_calls"),
        ("neocogi-idl", "parser", ("rejects_explicit_receiver_lifetimes",
                                  "receiver_lifetime_diagnostics_preserve_source_parts")),
    ]
    evidence = []
    for package, name, sentinel in rows:
        command = ["rustup", "run", toolchain, "cargo", "test", "-p", package,
                   "--all-features", "--locked", "--test", name, "--"]
        evidence.append({"package": package, "target": name, **execute_tests(
            lambda options: run([*command, *options], env=env), sentinel)})
    print(json.dumps({"compiler": version, "executed": evidence}))


if __name__ == "__main__":
    main()
