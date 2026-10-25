#!/usr/bin/env python3
"""Validate repository-local links in tracked Markdown files."""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit


ROOT = Path(__file__).resolve().parents[1]
INLINE_LINK = re.compile(r"!?\[[^\]]*\]\((?P<target><[^>]+>|[^\s)]+)(?:\s+[^)]*)?\)")
REFERENCE_LINK = re.compile(r"^\s*\[[^\]]+\]:\s*(?P<target><[^>]+>|\S+)")
HTML_LINK = re.compile(r"\b(?:href|src)=[\"'](?P<target>[^\"']+)[\"']")
HEADING = re.compile(r"^\s{0,3}#{1,6}\s+(?P<text>.*?)\s*#*\s*$")
EXTERNAL_SCHEMES = {"http", "https", "mailto", "tel"}


def tracked_markdown_files() -> list[Path]:
    """Include the complete COM documentation, even before the first commit."""
    roots = [ROOT / relative for relative in ("crates", "tools", "examples", "docs", "schemas", "tests")]
    files = [ROOT / "README.md", ROOT / "CHANGELOG.md"]
    files += [path for directory in roots for path in directory.rglob("*.md")
              if not {"target", ".git", "__pycache__"}.intersection(path.relative_to(ROOT).parts)]
    return sorted(path for path in files if path.is_file())


def github_anchor(text: str) -> str:
    text = re.sub(r"<[^>]*>", "", text)
    text = re.sub(r"!?\[([^\]]+)\]\([^)]*\)", r"\1", text)
    text = text.replace("`", "").replace("*", "").replace("_", "")
    text = text.strip().lower()
    text = re.sub(r"[^\w\- ]", "", text)
    return re.sub(r"\s+", "-", text)


def markdown_anchors(path: Path) -> set[str]:
    anchors: set[str] = set()
    occurrences: dict[str, int] = {}
    in_fence = False
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.lstrip().startswith("```"):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        match = HEADING.match(line)
        if match is None:
            continue
        base = github_anchor(match.group("text"))
        duplicate = occurrences.get(base, 0)
        occurrences[base] = duplicate + 1
        anchors.add(base if duplicate == 0 else f"{base}-{duplicate}")
    return anchors


def destinations(path: Path) -> list[tuple[int, str]]:
    found: list[tuple[int, str]] = []
    in_fence = False
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if line.lstrip().startswith("```"):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        for pattern in (INLINE_LINK, HTML_LINK):
            found.extend((line_number, match.group("target")) for match in pattern.finditer(line))
        reference = REFERENCE_LINK.match(line)
        if reference is not None:
            found.append((line_number, reference.group("target")))
    return found


def display_path(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def validate_link(source: Path, raw_target: str) -> str | None:
    target = raw_target.removeprefix("<").removesuffix(">")
    split = urlsplit(target)
    if split.scheme.lower() in EXTERNAL_SCHEMES or target.startswith("//"):
        return None
    if split.scheme or split.netloc:
        return f"unsupported link scheme in {raw_target!r}"

    relative_path = unquote(split.path)
    destination = source if not relative_path else source.parent / relative_path
    destination = destination.resolve()
    if not destination.exists():
        return f"missing target {raw_target!r} (resolved to {display_path(destination)})"

    fragment = unquote(split.fragment)
    if not fragment or destination.suffix.lower() != ".md" or re.fullmatch(r"L\d+(?:-L\d+)?", fragment):
        return None
    if fragment not in markdown_anchors(destination):
        return f"missing Markdown anchor #{fragment} in {display_path(destination)}"
    return None


def main() -> int:
    failures: list[str] = []
    checked = 0
    files = tracked_markdown_files()
    for source in files:
        for line, target in destinations(source):
            split = urlsplit(target.removeprefix("<").removesuffix(">"))
            if split.scheme.lower() in EXTERNAL_SCHEMES or target.startswith("//"):
                continue
            checked += 1
            error = validate_link(source, target)
            if error is not None:
                failures.append(f"{source.relative_to(ROOT)}:{line}: {error}")

    if failures:
        print("Markdown link check failed:", file=sys.stderr)
        for failure in failures:
            print(f"  {failure}", file=sys.stderr)
        return 1

    print(f"checked {checked} repository-local links across {len(files)} Markdown files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
