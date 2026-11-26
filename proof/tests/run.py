"""Compile the Rust reference probes with the installed supported compilers."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


TESTS = Path(__file__).resolve().parent
TOOLCHAINS = ("stable", "1.85.0")


def compilers():
    """Find supported toolchains without triggering a rustup download."""
    if not shutil.which("rustup"):
        print("rustup unavailable. Checking the installed rustc.", flush=True)
        return [["rustc"]]

    installed = subprocess.run(
        ["rustup", "toolchain", "list"], capture_output=True, text=True, check=True
    ).stdout
    names = [line.split()[0] for line in installed.splitlines() if line.strip()]
    commands = []
    for toolchain in TOOLCHAINS:
        if any(name == toolchain or name.startswith(toolchain + "-") for name in names):
            commands.append(["rustup", "run", toolchain, "rustc"])
        else:
            print(f"Skipping Rust {toolchain}. Toolchain is not installed.", flush=True)
    if not commands:
        raise SystemExit("No supported Rust toolchain is installed.")
    return commands


def expected_failure(source):
    """Read the one diagnostic marker and its source line."""
    markers = [
        (line, text.split("// ERROR ", 1)[1])
        for line, text in enumerate(source.read_text(encoding="utf-8").splitlines(), 1)
        if "// ERROR " in text
    ]
    if len(markers) != 1:
        raise SystemExit(f"{source.name}: expected one // ERROR marker")
    return markers[0]


def check_rejection(compiler, source, output):
    """Require the expected compiler error on the annotated expression."""
    line, expected = expected_failure(source)
    result = subprocess.run(
        compiler + [
            "--edition=2024", "--crate-type=lib", "--emit=metadata",
            "--error-format=json", str(source), "-o", str(output),
        ],
        capture_output=True,
        text=True,
    )
    diagnostics = [
        json.loads(message)
        for message in result.stderr.splitlines()
        if message.startswith("{")
    ]
    errors = [
        diagnostic for diagnostic in diagnostics
        if diagnostic["level"] == "error" and diagnostic["spans"]
    ]
    if result.returncode != 0 and len(errors) == 1:
        error = errors[0]
        code = (error["code"] or {}).get("code")
        primary_lines = {
            span["line_start"] for span in error["spans"]
            if span["is_primary"] and Path(span["file_name"]) == source
        }
        if line in primary_lines and expected in (code, error["message"]):
            print(f"  {source.name} rejected as expected ({expected})", flush=True)
            return

    observed = "".join(error["rendered"] for error in errors) or result.stderr
    raise SystemExit(f"{source.name}:{line}: expected {expected}\n{observed}")


def main():
    """Run the executable positive case and check all compile-fail cases."""
    failures = sorted((TESTS / "fail").glob("*.rs"))
    with tempfile.TemporaryDirectory(prefix="neocogi-rust-probes-") as temporary:
        temp = Path(temporary)
        executable = temp / ("pass.exe" if os.name == "nt" else "pass")
        for compiler in compilers():
            subprocess.run(compiler + ["--version"], check=True)
            subprocess.run(
                compiler + ["--edition=2024", str(TESTS / "pass.rs"), "-o", str(executable)],
                check=True,
            )
            subprocess.run([str(executable)], check=True)
            print("  pass.rs compiled and ran", flush=True)
            for source in failures:
                check_rejection(compiler, source, temp / "fail.rmeta")


if __name__ == "__main__":
    main()
