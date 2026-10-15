#!/usr/bin/env python3
"""Source-only COM test fixture for stable/MSRV and independently selected compilers.

The development workspace includes nightly Cargo artifact dependencies. External
library consumers do not. This fixture copies the published-library boundaries
and their tests, with a small stable workspace manifest, so compatibility tests
exercise that consumer constraint without changing production build ownership.
It never resolves dependencies, generates native packages or stages applications.
"""
from contextlib import contextmanager
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[3]
MEMBERS = (
    "crates/com/core", "crates/com/macros", "crates/com/idl", "crates/com/api",
    "crates/com/runtime", "tools/idl/example-contracts",
    "tools/idl/example-consumer-contracts", "tools/neocogi-inspect",
    "tools/cargo-neocogi", "examples/com-plugin",
)


@contextmanager
def library_workspace():
    """Lend a temporary source fixture and remove it after its child processes exit."""
    with tempfile.TemporaryDirectory(prefix="neocogi-library-sources-") as directory:
        root = Path(directory)
        for relative in ("crates/com", "tools/idl", "tools/neocogi-inspect", "tools/cargo-neocogi",
                         "examples/com-plugin", "docs/contracts", "docs/guides", "docs/architecture"):
            shutil.copytree(ROOT / relative, root / relative,
                            ignore=shutil.ignore_patterns("target", "__pycache__"))
        for relative in ("README.md", "rustfmt.toml"):
            shutil.copy2(ROOT / relative, root / relative)
        manifest = (ROOT / "Cargo.toml").read_text()
        members = "members = [\n" + "".join(f'    "{member}",\n' for member in MEMBERS) + "]"
        manifest, count = re.subn(r'(?ms)^members = \[.*?^\]', members, manifest, count=1)
        if count != 1:
            raise RuntimeError("repository workspace member list changed; update the library test fixture")
        manifest = re.sub(r'(?m)^default-members = .*$', 'default-members = ["crates/com/api"]', manifest)
        (root / "Cargo.toml").write_text(manifest)
        yield root


def main():
    """Run a library test command in the fixture, preserving its original output/target locations."""
    if len(sys.argv) < 2:
        raise SystemExit("usage: library_workspace.py COMMAND [ARG ...]")
    env = dict(os.environ, NEOCOGI_LIBRARY_WORKSPACE="1")
    env.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/msrv"))
    with library_workspace() as root:
        raise SystemExit(subprocess.call(sys.argv[1:], cwd=root, env=env))


if __name__ == "__main__":
    main()
