#!/usr/bin/env python3
"""Isolated COM source fixture for independently selected compiler builds.

The standalone repository already uses stable Cargo. Each compatibility run
still owns its copied source and target paths so embedded test paths stay valid.
"""
from contextlib import contextmanager
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
@contextmanager
def library_workspace():
    """Lend a temporary source fixture and remove it after its child processes exit."""
    with tempfile.TemporaryDirectory(prefix="neocogi-library-sources-") as directory:
        root = Path(directory)
        for relative in ("crates", "schemas", "tests", "scripts", "tools/neocogi-inspect",
                         "examples/com-plugin", "examples/com-host", "examples/schemas", "docs"):
            shutil.copytree(ROOT / relative, root / relative,
                            ignore=shutil.ignore_patterns("target", "__pycache__"))
        for relative in ("README.md", "rustfmt.toml", "Cargo.lock"):
            shutil.copy2(ROOT / relative, root / relative)
        shutil.copy2(ROOT / "Cargo.toml", root / "Cargo.toml")
        yield root


def main():
    """Run a command with source and build artifacts owned by one temporary fixture."""
    if len(sys.argv) < 2:
        raise SystemExit("usage: library_workspace.py COMMAND [ARG ...]")
    env = dict(os.environ, NEOCOGI_LIBRARY_WORKSPACE="1")
    with library_workspace() as root:
        # Test binaries embed this source owner's manifest paths. Reusing a
        # persistent target after the owner is deleted can execute stale binaries
        # pointing into a previous fixture. Source and artifacts retire together,
        # as in the archive verifier, while children share this run's one target.
        env["CARGO_TARGET_DIR"] = str(root / "target")
        print(f"library fixture target: {env['CARGO_TARGET_DIR']}", flush=True)
        raise SystemExit(subprocess.call(sys.argv[1:], cwd=root, env=env))


if __name__ == "__main__":
    main()
