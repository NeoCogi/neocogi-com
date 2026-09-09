#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../../.."

for command in cargo tar cmp mktemp; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "$command is required for package verification" >&2
    exit 1
  fi
done

version="$(awk -F '"' '/^version = "/ { print $2; exit }' crates/com/api/Cargo.toml)"
if [[ -z "$version" ]]; then
  echo "failed to read the workspace package version" >&2
  exit 1
fi

packages=(
  neocogi-com-core
  neocogi-com-macros
  neocogi-com-runtime-sys
  neocogi-com-runtime
  neocogi-com
  neocogi-inspect
)

for package in "${packages[@]}"; do
  package_license="crates/com/$package/LICENSE-MIT"
  if [[ "$package" == neocogi-inspect ]]; then package_license="tools/neocogi-inspect/LICENSE-MIT"; fi
  if [[ ! -f "$package_license" ]] || ! cmp -s crates/com/LICENSE-MIT "$package_license"; then
    echo "$package_license must be an exact copy of the root LICENSE-MIT" >&2
    exit 1
  fi
done

echo "==> [packages] fetch locked registry dependencies"
cargo fetch --locked

echo "==> [packages] build workspace archives"
cargo package -p neocogi-com-core -p neocogi-com-macros -p neocogi-com-runtime-sys -p neocogi-com-runtime -p neocogi-com -p neocogi-inspect --no-verify --allow-dirty --locked

package_dir="${CARGO_TARGET_DIR:-target}/package"
for package in "${packages[@]}"; do
  archive="$package_dir/$package-$version.crate"
  prefix="$package-$version"
  if [[ ! -f "$archive" ]]; then
    echo "missing package archive: $archive" >&2
    exit 1
  fi

  listing="$(tar -tzf "$archive")"
  for required in Cargo.toml Cargo.toml.orig README.md LICENSE-MIT; do
    if ! grep -Fxq "$prefix/$required" <<<"$listing"; then
      echo "$archive is missing $required" >&2
      exit 1
    fi
  done
  if ! grep -Fq "$prefix/src/" <<<"$listing"; then
    echo "$archive does not contain source files" >&2
    exit 1
  fi
  if grep -Eq '(^|/)(target|\.git)(/|$)' <<<"$listing"; then
    echo "$archive contains repository-only files" >&2
    exit 1
  fi

  echo "archive contents passed: $package"
done

verify_root="$(mktemp -d "${TMPDIR:-/tmp}/neocogi-com-package-verify.XXXXXX")"
trap 'rm -rf "$verify_root"' EXIT

for package in "${packages[@]}"; do
  tar -xzf "$package_dir/$package-$version.crate" -C "$verify_root"
done

verify_package() {
  local package="$1"
  shift
  local manifest="$verify_root/$package-$version/Cargo.toml"

  echo
  echo "==> [packages] verify $package"
  cargo check \
    --manifest-path "$manifest" \
    --all-targets \
    --all-features \
    --offline \
    --target-dir "$verify_root/target/$package" \
    "$@"
}

# This is the crates.io publication order. Each package may resolve neocogi-com
# dependencies only from archives verified earlier in this sequence.
verify_package neocogi-com-core
verify_package neocogi-com-macros \
  --config "patch.crates-io.neocogi-com-core.path='$verify_root/neocogi-com-core-$version'"
verify_package neocogi-com-runtime-sys \
  --config "patch.crates-io.neocogi-com-core.path='$verify_root/neocogi-com-core-$version'"
verify_package neocogi-com-runtime \
  --config "patch.crates-io.neocogi-com-core.path='$verify_root/neocogi-com-core-$version'"
verify_package neocogi-com \
  --config "patch.crates-io.neocogi-com-core.path='$verify_root/neocogi-com-core-$version'" \
  --config "patch.crates-io.neocogi-com-macros.path='$verify_root/neocogi-com-macros-$version'" \
  --config "patch.crates-io.neocogi-com-runtime-sys.path='$verify_root/neocogi-com-runtime-sys-$version'"
verify_package neocogi-inspect \
  --config "patch.crates-io.neocogi-com-core.path='$verify_root/neocogi-com-core-$version'" \
  --config "patch.crates-io.neocogi-com-runtime.path='$verify_root/neocogi-com-runtime-$version'"

echo
echo "package archives and publish order passed"
