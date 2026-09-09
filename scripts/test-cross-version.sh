#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../../.."

case "$(uname -s)" in
  Linux|Darwin|FreeBSD|OpenBSD|NetBSD)
    ;;
  *)
    echo "cross-version component tests are Unix-only; unsupported OS: $(uname -s)" >&2
    exit 1
    ;;
esac

require_toolchain() {
  local toolchain="$1"

  if ! rustup run "$toolchain" cargo --version >/dev/null 2>&1; then
    echo "required Rust toolchain '$toolchain' is not available" >&2
    echo "install it with: rustup toolchain install $toolchain" >&2
    exit 1
  fi
}

require_toolchain stable
require_toolchain nightly

target_dir="${CARGO_TARGET_DIR:-target}"
runtime_link_dir="$target_dir/debug"
runtime_deps_dir="$runtime_link_dir/deps"

case "$(uname -s)" in
  Darwin)
    dylib_path_var="DYLD_LIBRARY_PATH"
    ;;
  *)
    dylib_path_var="LD_LIBRARY_PATH"
    ;;
esac

rustflags="${RUSTFLAGS:-}"
if [[ -n "$rustflags" ]]; then
  rustflags+=" "
fi
rustflags+="-L native=$runtime_deps_dir -L native=$runtime_link_dir"

dylib_path="$runtime_deps_dir:$runtime_link_dir"
if [[ -n "${!dylib_path_var:-}" ]]; then
  dylib_path+=":${!dylib_path_var}"
fi

cargo build -p neocogi-com-runtime --offline

env \
  RUSTFLAGS="$rustflags" \
  "$dylib_path_var=$dylib_path" \
  cargo test -p neocogi-com --test cross_version_component \
    --no-default-features --features macros,std,runtime --offline "$@"
