#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../../.."

case "$(uname -s)" in
  Linux|Darwin|FreeBSD|OpenBSD|NetBSD)
    ;;
  *)
    echo "safety tests are Unix-only; unsupported OS: $(uname -s)" >&2
    exit 1
    ;;
esac

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required for safety tests" >&2
  echo "install rustup, nightly Rust, and the nightly miri and rust-src components" >&2
  exit 1
fi

if ! rustup run nightly cargo --version >/dev/null 2>&1; then
  echo "required Rust toolchain 'nightly' is not available" >&2
  echo "install it with: rustup toolchain install nightly" >&2
  exit 1
fi

if ! rustup component list --toolchain nightly --installed | grep -Eq '^miri([ -]|$)'; then
  echo "nightly miri component is required for safety tests" >&2
  echo "install it with: rustup +nightly component add miri rust-src" >&2
  exit 1
fi

if ! rustup component list --toolchain nightly --installed | grep -Eq '^rust-src([ -]|$)'; then
  echo "nightly rust-src component is required for miri safety tests" >&2
  echo "install it with: rustup +nightly component add rust-src" >&2
  exit 1
fi

if ! rustup run nightly cargo miri --version >/dev/null 2>&1; then
  echo "cargo miri is not runnable on the nightly toolchain" >&2
  echo "install it with: rustup +nightly component add miri rust-src" >&2
  exit 1
fi

run_miri() {
  local row="$1"
  shift

  echo
  echo "==> [nightly/miri] $row"
  cargo +nightly miri test "$@"
}

require_miri_test() {
  local required_test="$1"
  shift

  local listed
  listed="$(cargo +nightly miri test "$@" -- --list)"
  printf '%s\n' "$listed"
  if ! grep -Fq "$required_test: test" <<<"$listed"; then
    echo "Miri safety test was not discovered: $required_test" >&2
    exit 1
  fi
}

require_miri_test \
  "five_plugin_dependency_chain_plans_and_registers_in_process" \
  -p nc-com-runtime --lib --all-features "$@"
require_miri_test \
  "multiple_generated_objects_keep_independent_lifetimes" \
  -p nc-com --test com_usage --no-default-features --features macros,std,runtime "$@"
require_miri_test \
  "table_rejects_a_short_allocation_before_forming_a_full_reference" \
  -p nc-com --lib --no-default-features --features macros,std,runtime "$@"
echo "Miri safety sentinels discovered: 3"

run_miri "nc-com-core library" -p nc-com-core --lib --all-features "$@"
run_miri "nc-com-runtime library" -p nc-com-runtime --lib --all-features "$@"
run_miri "nc-com library" \
  -p nc-com --lib --no-default-features --features macros,std,runtime "$@"
run_miri "pure Rust COM lifecycle integration tests" \
  -p nc-com --test com_usage --no-default-features --features macros,std,runtime "$@"

echo
echo "safety tests passed"
