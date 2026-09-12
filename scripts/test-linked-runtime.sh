#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../../.."

case "$(uname -s)" in
  Linux|Darwin|FreeBSD|OpenBSD|NetBSD)
    ;;
  *)
    echo "linked-runtime tests require the PowerShell script on this platform" >&2
    exit 1
    ;;
esac

if ! command -v rustup >/dev/null 2>&1 || ! rustup run nightly cargo --version >/dev/null 2>&1; then
  echo "stable and nightly Rust toolchains are required" >&2
  exit 1
fi

target_dir="${CARGO_TARGET_DIR:-target/linked-runtime}"
# The runtime package is normally an rlib consumed by cargo-neocogi's generated
# root. This direct-link fixture supplies that one cdylib root itself.
cargo rustc -p neocogi-com-runtime --lib --crate-type cdylib --locked --target-dir "$target_dir"

runtime_dir="$(cd "$target_dir/debug" && pwd)"
runtime_deps_dir="$runtime_dir/deps"
rustflags="${RUSTFLAGS:-}"
if [[ -n "$rustflags" ]]; then
  rustflags+=" "
fi
rustflags+="-Lnative=$runtime_dir -Lnative=$runtime_deps_dir"

case "$(uname -s)" in
  Darwin)
    loader_path_var="DYLD_LIBRARY_PATH"
    ;;
  *)
    loader_path_var="LD_LIBRARY_PATH"
    ;;
esac
loader_path="$runtime_dir:$runtime_deps_dir"
if [[ -n "${!loader_path_var:-}" ]]; then
  loader_path+=":${!loader_path_var}"
fi

run_linked_cargo() {
  env \
    RUSTFLAGS="$rustflags" \
    NC_COM_RUNTIME_LINK_DIR="$runtime_dir" \
    "$loader_path_var=$loader_path" \
    cargo "$@"
}

listed="$(run_linked_cargo test -p neocogi-com --features runtime --locked \
  --target-dir "$target_dir" -- --list)"
printf '%s\n' "$listed"

required_tests=(
  "generated_contract_factory_activates_without_a_class_table"
  "dynamic_factory_plugin_links_the_exact_contract_dso"
)
for required_test in "${required_tests[@]}"; do
  if ! grep -Fq "$required_test: test" <<<"$listed"; then
    echo "linked-runtime test was not discovered: $required_test" >&2
    exit 1
  fi
done

run_linked_cargo test -p neocogi-com --features runtime --locked --target-dir "$target_dir"

echo "linked-runtime test suite passed"
