#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../../.."

for command in cargo tar cmp mktemp; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "$command is required for package verification" >&2
    exit 1
  fi
done

version="$(awk -F '"' '/^version = "/ { print $2; exit }' crates/com/nc-com/Cargo.toml)"
if [[ -z "$version" ]]; then
  echo "failed to read the workspace package version" >&2
  exit 1
fi

packages=(
  nc-com-core
  nc-com-macros
  nc-com-runtime-sys
  nc-com-runtime
  nc-com
  nc-com-inspect
)

for package in "${packages[@]}"; do
  package_license="crates/com/$package/LICENSE-MIT"
  if [[ "$package" == nc-com-inspect ]]; then package_license="tools/nc-com-inspect/LICENSE-MIT"; fi
  if [[ ! -f "$package_license" ]] || ! cmp -s crates/com/LICENSE-MIT "$package_license"; then
    echo "$package_license must be an exact copy of the root LICENSE-MIT" >&2
    exit 1
  fi
done

echo "==> [packages] fetch locked registry dependencies"
cargo fetch --locked

echo "==> [packages] build workspace archives"
cargo package -p nc-com-core -p nc-com-macros -p nc-com-runtime-sys -p nc-com-runtime -p nc-com -p nc-com-inspect --no-verify --allow-dirty --locked

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

verify_root="$(mktemp -d "${TMPDIR:-/tmp}/nc-com-package-verify.XXXXXX")"
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

# This is the crates.io publication order. Each package may resolve nc-com
# dependencies only from archives verified earlier in this sequence.
verify_package nc-com-core
verify_package nc-com-macros \
  --config "patch.crates-io.nc-com-core.path='$verify_root/nc-com-core-$version'"
verify_package nc-com-runtime-sys \
  --config "patch.crates-io.nc-com-core.path='$verify_root/nc-com-core-$version'"
verify_package nc-com-runtime \
  --config "patch.crates-io.nc-com-core.path='$verify_root/nc-com-core-$version'"
verify_package nc-com \
  --config "patch.crates-io.nc-com-core.path='$verify_root/nc-com-core-$version'" \
  --config "patch.crates-io.nc-com-macros.path='$verify_root/nc-com-macros-$version'" \
  --config "patch.crates-io.nc-com-runtime-sys.path='$verify_root/nc-com-runtime-sys-$version'"
verify_package nc-com-inspect \
  --config "patch.crates-io.nc-com-core.path='$verify_root/nc-com-core-$version'" \
  --config "patch.crates-io.nc-com-runtime.path='$verify_root/nc-com-runtime-$version'"

echo
echo "package archives and publish order passed"
