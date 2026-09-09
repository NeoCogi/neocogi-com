#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../../.."

msrv="1.85.0"
target_dir="${CARGO_TARGET_DIR:-target/msrv}"

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required for MSRV tests" >&2
  echo "install rustup, then run: rustup toolchain install $msrv --profile minimal" >&2
  exit 1
fi

if ! rustup run "$msrv" cargo --version >/dev/null 2>&1; then
  echo "required Rust toolchain '$msrv' is not available" >&2
  echo "install it with: rustup toolchain install $msrv --profile minimal" >&2
  exit 1
fi

run_msrv_cargo() {
  local row="$1"
  shift

  echo
  echo "==> [rust $msrv] $row"
  cargo "+$msrv" "$@" --target-dir "$target_dir"
}

run_msrv_cargo "all workspace targets and features" \
  check -p neocogi-com-core -p neocogi-com-macros -p neocogi-com-runtime-sys -p neocogi-com-runtime -p neocogi-com -p neocogi-inspect --all-targets --all-features --locked
run_msrv_cargo "workspace library tests" \
  test -p neocogi-com-core -p neocogi-com-macros -p neocogi-com-runtime-sys -p neocogi-com-runtime -p neocogi-com -p neocogi-inspect --lib --locked
run_msrv_cargo "ABI v1 normative snippets" \
  test --manifest-path crates/com/tests/abi-spec/Cargo.toml --doc --locked
run_msrv_cargo "inspector binary tests" \
  test -p neocogi-inspect --bin neocogi-inspect --locked
run_msrv_cargo "neocogi-com default features" \
  check -p neocogi-com --locked
run_msrv_cargo "neocogi-com without default features" \
  check -p neocogi-com --no-default-features --locked
run_msrv_cargo "neocogi-com with alloc only" \
  check -p neocogi-com --no-default-features --features alloc --locked
run_msrv_cargo "neocogi-com-core without default features" \
  check -p neocogi-com-core --no-default-features --locked

echo
echo "Rust $msrv MSRV suite passed"
