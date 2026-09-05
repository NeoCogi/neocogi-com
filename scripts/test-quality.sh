#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../../.."

if ! cargo fmt --version >/dev/null 2>&1; then
  echo "rustfmt is required for quality checks" >&2
  echo "install it with: rustup component add rustfmt" >&2
  exit 1
fi

if ! cargo clippy --version >/dev/null 2>&1; then
  echo "Clippy is required for quality checks" >&2
  echo "install it with: rustup component add clippy" >&2
  exit 1
fi

if ! python3 --version >/dev/null 2>&1; then
  echo "Python 3 is required for Markdown link checks" >&2
  exit 1
fi

run_cargo() {
  local row="$1"
  shift

  echo
  echo "==> [quality] $row"
  cargo "$@"
}

run_rustdoc() {
  local row="$1"
  local target_dir="$2"
  shift 2

  echo
  echo "==> [rustdoc] $row"
  env RUSTDOCFLAGS="${RUSTDOCFLAGS:+$RUSTDOCFLAGS }-D warnings -D missing-docs" \
    cargo doc --no-deps --locked --target-dir "$target_dir" "$@"
}

run_cargo "format" fmt --all --check
run_cargo "Clippy, all targets and features" \
  clippy -p nc-com-core -p nc-com-macros -p nc-com-runtime-sys -p nc-com-runtime -p nc-com -p nc-com-inspect --all-targets --all-features --locked -- -D warnings
run_cargo "workspace with all features" \
  check -p nc-com-core -p nc-com-macros -p nc-com-runtime-sys -p nc-com-runtime -p nc-com -p nc-com-inspect --all-features --locked
run_cargo "nc-com without default features" \
  check -p nc-com --no-default-features --locked
run_cargo "nc-com with alloc only" \
  check -p nc-com --no-default-features --features alloc --locked
run_cargo "nc-com-core without default features" \
  check -p nc-com-core --no-default-features --locked
run_cargo "ABI v1 normative snippets" \
  test --manifest-path crates/com/tests/abi-spec/Cargo.toml --doc --locked \
  --target-dir target/abi-spec

echo
echo "==> [quality] Markdown links"
python3 crates/com/scripts/check-markdown-links.py

run_rustdoc "workspace with all features" target/quality-docs/all \
  -p nc-com-core -p nc-com-macros -p nc-com-runtime-sys -p nc-com-runtime -p nc-com -p nc-com-inspect --all-features
run_rustdoc "nc-com default features" target/quality-docs/default \
  -p nc-com
run_rustdoc "nc-com without default features" target/quality-docs/no-default \
  -p nc-com --no-default-features
run_rustdoc "nc-com with alloc only" target/quality-docs/alloc \
  -p nc-com --no-default-features --features alloc

echo
echo "quality suite passed"
