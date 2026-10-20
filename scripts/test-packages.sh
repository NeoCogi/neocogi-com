#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../../.."

for command in cargo tar cmp mktemp python3; do
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
  neocogi-idl
  neocogi-com-codegen
  neocogi-com-macros
  neocogi-com-runtime
  neocogi-com
  neocogi-inspect
)

for package in "${packages[@]}"; do
  # Published package names intentionally differ from their workspace folders.
  case "$package" in
    neocogi-com-core) package_license="crates/com/core/LICENSE-MIT" ;;
    neocogi-idl) package_license="crates/com/idl/LICENSE-MIT" ;;
    neocogi-com-codegen) package_license="crates/com/codegen/LICENSE-MIT" ;;
    neocogi-com-macros) package_license="crates/com/macros/LICENSE-MIT" ;;
    neocogi-com-runtime) package_license="crates/com/runtime/LICENSE-MIT" ;;
    neocogi-com) package_license="crates/com/api/LICENSE-MIT" ;;
    neocogi-inspect) package_license="tools/neocogi-inspect/LICENSE-MIT" ;;
  esac
  if [[ ! -f "$package_license" ]] || ! cmp -s crates/com/LICENSE-MIT "$package_license"; then
    echo "$package_license must be an exact copy of the root LICENSE-MIT" >&2
    exit 1
  fi
done

echo "==> [packages] build workspace archives"
cargo package -p neocogi-com-core -p neocogi-idl -p neocogi-com-codegen -p neocogi-com-macros -p neocogi-com-runtime -p neocogi-com -p neocogi-inspect --no-verify --allow-dirty --locked --offline

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

# Only extracted archives supply SDK dependencies. Cargo and nested consumer
# tests discover this same patch configuration from the verification directory;
# no repository sibling or source-only workspace is restored around the archives.
python3 - "$verify_root" "$version" <<'PYCONFIG'
import json
from pathlib import Path
import sys
root, version = Path(sys.argv[1]), sys.argv[2]
packages = ("neocogi-com-core", "neocogi-idl", "neocogi-com-codegen", "neocogi-com-macros",
            "neocogi-com-runtime", "neocogi-com", "neocogi-inspect")
(root / ".cargo").mkdir()
(root / ".cargo/config.toml").write_text("[patch.crates-io]\n" + "".join(
    f'{name} = {{ path = {json.dumps(str(root / (name + "-" + version)))} }}\n'
    for name in packages))
PYCONFIG

# Share dependencies within this run, then discard compiled artifacts with the
# extraction. Reusing an earlier run's binaries can retain its deleted manifest
# paths in test-only env! expansions even when Cargo reuses their fingerprints.
export CARGO_TARGET_DIR="$verify_root/target"
cd "$verify_root"

for package in "${packages[@]}"; do
  manifest="$verify_root/$package-$version/Cargo.toml"
  echo "==> [packages] Rust 1.85 library/binary check: $package"
  cargo +1.85.0 check --manifest-path "$manifest" --all-targets --all-features --offline
  echo "==> [packages] all shipped tests: $package"
  cargo +1.85.0 test --manifest-path "$manifest" --tests --all-features --offline
  if [[ "$package" != neocogi-inspect ]]; then
    echo "==> [packages] published documentation examples: $package"
    cargo +1.85.0 test --manifest-path "$manifest" --doc --all-features --offline
  fi
done

# The public runtime build guide promises both ordinary Cargo outputs. Check
# actual files, rather than a copied manifest description or an explicit override.
echo "==> [packages] ordinary runtime build emits its declared rlib and shared library"
cargo +1.85.0 build --manifest-path "$verify_root/neocogi-com-runtime-$version/Cargo.toml" --offline
python3 - <<'PYRUNTIME'
import os
from pathlib import Path
import platform
output = Path(os.environ["CARGO_TARGET_DIR"]) / "debug"
shared = {"Linux": "libnc_com.so", "Darwin": "libnc_com.dylib", "Windows": "nc_com.dll"}[platform.system()]
for name in ("libnc_com.rlib", shared):
    artifact = output / name
    if not artifact.is_file() or artifact.stat().st_size == 0:
        raise SystemExit(f"ordinary runtime build did not emit {artifact}")
print("ordinary runtime outputs passed: rlib and shared library")
PYRUNTIME

# Ordinary downstream usage: generate interfaces at build time, implement one
# with the published macros, then execute its actual checked C-ABI dispatch.
mkdir -p consumer/src consumer/idl/proof
cat > consumer/Cargo.toml <<'MANIFEST'
[package]
name = "archive-consumer"
version = "0.0.0"
edition = "2024"
# Cargo owner encodes the exact IDL module `proof.archive` as UTF-8 hex.
# Demangle by decoding hex pairs after nc_contract_; this is not a .so name.
links = "nc_contract_70726f6f662e61726368697665"
[dependencies]
MANIFEST
printf 'neocogi-com = "=%s"\n[build-dependencies]\nneocogi-idl = "=%s"\n' "$version" "$version" >> consumer/Cargo.toml
cat > consumer/build.rs <<'BUILD'
//! A normal downstream build script; no source-tree layout assumptions.
fn main() {
    neocogi_idl::cargo::Module::new("idl", "proof/archive.ncidl").build().unwrap();
}
BUILD
cat > consumer/idl/proof/archive.ncidl <<'IDL'
/// Minimal construction-independent interface for the extracted SDK proof.
#[version(1)] interface Counter { fn read(&self) -> Result<u32>; }
IDL
cat > consumer/src/main.rs <<'RUST'
include!(concat!(env!("OUT_DIR"), "/proof_archive.include.rs"));
use neocogi_com::{com_object, NcStatus};
#[com_object(Counter)]
struct Value;
impl Counter for Value {
    fn read(&self) -> Result<u32, NcStatus> { Ok(42) }
}
fn main() {
    let counter = Value::from_value(Value).query::<CounterAbi>().unwrap();
    assert_eq!(counter.read(), Ok(42));
    assert_eq!(neocogi_com::NC_COM_RUNTIME_ABI_VERSION, 1);
    assert_eq!(neocogi_com::NC_PLUGIN_DESCRIPTOR_ABI_VERSION, 1);
}
RUST
cargo +1.85.0 run --manifest-path consumer/Cargo.toml --offline
cargo +1.85.0 check --manifest-path "$verify_root/neocogi-com-$version/Cargo.toml" --no-default-features --offline
cargo +1.85.0 check --manifest-path "$verify_root/neocogi-com-$version/Cargo.toml" --no-default-features --features alloc --offline

echo "package archive targets, shipped tests and downstream COM call passed"
