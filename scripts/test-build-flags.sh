#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../../.."

case "$(uname -s)" in
  Linux|Darwin|FreeBSD|OpenBSD|NetBSD)
    ;;
  *)
    echo "build-flag compatibility tests are Unix-only; unsupported OS: $(uname -s)" >&2
    exit 1
    ;;
esac

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required for build-flag compatibility tests" >&2
  echo "install rustup and the stable and nightly Rust toolchains" >&2
  exit 1
fi

require_toolchain() {
  local toolchain="$1"

  if ! rustup run "$toolchain" cargo --version >/dev/null 2>&1; then
    echo "required Rust toolchain '$toolchain' is not available" >&2
    echo "install it with: rustup toolchain install $toolchain" >&2
    exit 1
  fi
}

run_workspace_tests() {
  local toolchain="$1"
  local row="$2"
  shift 2

  echo
  echo "==> [$toolchain] $row"
  # Do not use --all-features here: the runtime feature intentionally requires
  # an already built neocogi-com-runtime shared library on the native link path.
  cargo "+$toolchain" test -p neocogi-com-core -p neocogi-com-macros -p neocogi-com-runtime-sys -p neocogi-com-runtime -p neocogi-com -p neocogi-inspect "$@"
}

run_linked_facade_smoke_tests() {
  local toolchain="$1"
  local row="$2"
  local profile="$3"
  local target_dir="$4"
  shift 4
  local cargo_args=(--target-dir "$target_dir" "$@")

  echo
  echo "==> [$toolchain] $row"

  # cargo-neocogi normally supplies the runtime cdylib root. This direct-link
  # matrix creates the equivalent root for each compiler/profile combination.
  cargo "+$toolchain" rustc -p neocogi-com-runtime --lib --crate-type cdylib "${cargo_args[@]}"

  local runtime_dir
  runtime_dir="$(cd "$target_dir/$profile" && pwd)"
  local runtime_deps_dir="$runtime_dir/deps"
  local rustflags="${RUSTFLAGS:-}"
  if [[ -n "$rustflags" ]]; then
    rustflags+=" "
  fi
  rustflags+="-Lnative=$runtime_dir -Lnative=$runtime_deps_dir"

  local loader_path_var
  case "$(uname -s)" in
    Darwin)
      loader_path_var="DYLD_LIBRARY_PATH"
      ;;
    *)
      loader_path_var="LD_LIBRARY_PATH"
      ;;
  esac

  local loader_path="$runtime_dir:$runtime_deps_dir"
  if [[ -n "${!loader_path_var:-}" ]]; then
    loader_path+=":${!loader_path_var}"
  fi

  run_linked_cargo() {
    env \
      RUSTFLAGS="$rustflags" \
      NC_COM_RUNTIME_LINK_DIR="$runtime_dir" \
      "$loader_path_var=$loader_path" \
      cargo "+$toolchain" "$@"
  }

  local registration_test="generated_contract_factory_activates_without_a_class_table"
  local dynamic_plugin_test="dynamic_factory_plugin_links_the_exact_contract_dso"
  local listed
  listed="$(run_linked_cargo test -p neocogi-com --features runtime \
    --test factory_plugin_runtime "${cargo_args[@]}" -- --list)"
  printf '%s\n' "$listed"
  if ! grep -Fq "$registration_test: test" <<<"$listed"; then
    echo "linked build-flag test was not discovered: $registration_test" >&2
    exit 1
  fi
  if ! grep -Fq "$dynamic_plugin_test: test" <<<"$listed"; then
    echo "linked build-flag test was not discovered: $dynamic_plugin_test" >&2
    exit 1
  fi

  run_linked_cargo test -p neocogi-com --features runtime \
    --test factory_plugin_runtime "${cargo_args[@]}" "$registration_test" -- --exact
  run_linked_cargo test -p neocogi-com --features runtime \
    --test factory_plugin_runtime "${cargo_args[@]}" "$dynamic_plugin_test" -- --exact
}

run_toolchain_matrix() {
  local toolchain="$1"
  shift

  local target_root="target/build-flag-matrix/$toolchain"

  run_workspace_tests \
    "$toolchain" \
    "dev default" \
    --target-dir "$target_root/dev-default" \
    "$@"
  run_linked_facade_smoke_tests \
    "$toolchain" \
    "dev linked facade" \
    debug \
    "$target_root/dev-linked" \
    "$@"

  local panic_strategy
  local lto_label
  local lto_value
  for panic_strategy in unwind abort; do
    for lto_label in off thin fat; do
      case "$lto_label" in
        off)
          lto_value=false
          ;;
        thin|fat)
          lto_value="$lto_label"
          ;;
      esac

      (
        export CARGO_PROFILE_RELEASE_PANIC="$panic_strategy"
        export CARGO_PROFILE_RELEASE_LTO="$lto_value"
        run_workspace_tests \
          "$toolchain" \
          "release panic=$panic_strategy lto=$lto_label" \
          --release \
          --target-dir "$target_root/release-panic-$panic_strategy-lto-$lto_label" \
          "$@"
        run_linked_facade_smoke_tests \
          "$toolchain" \
          "release linked facade panic=$panic_strategy lto=$lto_label" \
          release \
          "$target_root/release-linked-panic-$panic_strategy-lto-$lto_label" \
          --release \
          "$@"
      )
    done
  done
}

require_toolchain stable
require_toolchain nightly

run_toolchain_matrix stable "$@"
run_toolchain_matrix nightly "$@"

echo
echo "build-flag compatibility matrix passed"
