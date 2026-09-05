#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../../.."

case "$(uname -s)" in
  Linux)
    ;;
  *)
    echo "sanitizer tests are Unix-only and currently require Linux; unsupported OS: $(uname -s)" >&2
    exit 1
    ;;
esac

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required for sanitizer tests" >&2
  echo "install rustup, nightly Rust, and the nightly rust-src component" >&2
  exit 1
fi

if ! rustup run nightly cargo --version >/dev/null 2>&1; then
  echo "required Rust toolchain 'nightly' is not available" >&2
  echo "install it with: rustup toolchain install nightly" >&2
  exit 1
fi

if ! rustup component list --toolchain nightly --installed | grep -Eq '^rust-src([ -]|$)'; then
  echo "nightly rust-src component is required for sanitizer tests" >&2
  echo "install it with: rustup +nightly component add rust-src" >&2
  exit 1
fi

target="$(rustup run nightly rustc -vV | awk '/^host: / { print $2 }')"
case "$target" in
  x86_64-unknown-linux-gnu)
    ;;
  *)
    echo "sanitizer tests currently support x86_64-unknown-linux-gnu only" >&2
    echo "current nightly host target: $target" >&2
    exit 1
    ;;
esac

rustflags_env="CARGO_TARGET_${target^^}_RUSTFLAGS"
rustflags_env="${rustflags_env//-/_}"

run_sanitizer() {
  local sanitizer="$1"
  local label="$2"
  local target_dir="target/sanitizers/$sanitizer"
  shift 2

  run_sanitized_test "$sanitizer" "$label" "$target_dir" \
    "nc-com-core library" \
    -p nc-com-core --lib --all-features "$@"
  run_sanitized_test "$sanitizer" "$label" "$target_dir" \
    "nc-com-runtime library" \
    -p nc-com-runtime --lib --all-features "$@"
  run_sanitized_build "$sanitizer" "$label" "$target_dir" \
    "nc-com-runtime shared library" \
    -p nc-com-runtime --all-features
  run_sanitized_build "$sanitizer" "$label" "$target_dir" \
    "multi-plugin shared libraries" \
    -p nc-com --example component_plugin_counter --example component_plugin_flag \
    --example component_plugin_extra --example component_plugin_metric \
    --example component_plugin_state \
    --no-default-features --features macros,std,runtime
  run_sanitized_run "$sanitizer" "$label" "$target_dir" \
    "multi-plugin host example" \
    -p nc-com --example multi_plugin_host \
    --no-default-features --features macros,std,runtime -- \
    "$target_dir/$target/debug/examples/libcomponent_plugin_counter.so" \
    "$target_dir/$target/debug/examples/libcomponent_plugin_flag.so" \
    "$target_dir/$target/debug/examples/libcomponent_plugin_metric.so" \
    "$target_dir/$target/debug/examples/libcomponent_plugin_state.so" \
    "$target_dir/$target/debug/examples/libcomponent_plugin_extra.so"
  run_sanitized_test "$sanitizer" "$label" "$target_dir" \
    "nc-com library" \
    -p nc-com --lib --no-default-features --features macros,std,runtime "$@"
  run_sanitized_test "$sanitizer" "$label" "$target_dir" \
    "pure Rust COM lifecycle integration tests" \
    -p nc-com --test com_usage --no-default-features --features macros,std,runtime "$@"
}

run_sanitized_build() {
  local sanitizer="$1"
  local label="$2"
  local target_dir="$3"
  local target_debug="$target_dir/$target/debug"
  local target_deps="$target_debug/deps"
  local row="$4"
  shift 4

  echo
  echo "==> [nightly/$label] $row"
  env \
    ASAN_OPTIONS=detect_leaks=0 \
    LSAN_OPTIONS=detect_leaks=0 \
    "LD_LIBRARY_PATH=$target_deps:$target_debug${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
    "$rustflags_env=-Zsanitizer=$sanitizer -L native=$target_deps -L native=$target_debug" \
    cargo +nightly build -Zbuild-std \
      --target "$target" \
      --target-dir "$target_dir" \
      "$@"
}

run_sanitized_run() {
  local sanitizer="$1"
  local label="$2"
  local target_dir="$3"
  local target_debug="$target_dir/$target/debug"
  local target_deps="$target_debug/deps"
  local row="$4"
  shift 4

  echo
  echo "==> [nightly/$label] $row"
  env \
    ASAN_OPTIONS=detect_leaks=0 \
    LSAN_OPTIONS=detect_leaks=0 \
    "LD_LIBRARY_PATH=$target_deps:$target_debug${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
    "$rustflags_env=-Zsanitizer=$sanitizer -L native=$target_deps -L native=$target_debug" \
    cargo +nightly run -Zbuild-std \
      --target "$target" \
      --target-dir "$target_dir" \
      "$@"
}

run_sanitized_test() {
  local sanitizer="$1"
  local label="$2"
  local target_dir="$3"
  local target_debug="$target_dir/$target/debug"
  local target_deps="$target_debug/deps"
  local row="$4"
  shift 4

  echo
  echo "==> [nightly/$label] $row"
  env \
    ASAN_OPTIONS=detect_leaks=0 \
    LSAN_OPTIONS=detect_leaks=0 \
    "LD_LIBRARY_PATH=$target_deps:$target_debug${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
    "$rustflags_env=-Zsanitizer=$sanitizer -L native=$target_deps -L native=$target_debug" \
    cargo +nightly test -Zbuild-std \
      --target "$target" \
      --target-dir "$target_dir" \
      "$@"
}

run_sanitizer address asan "$@"
run_sanitizer thread tsan "$@"

echo
echo "sanitizer tests passed"
