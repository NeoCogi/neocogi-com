#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../../.."

target="${NC_COM_32_BIT_TARGET:-i686-unknown-linux-gnu}"

echo
echo "==> [32-bit ABI] core layout and golden-vector tests ($target)"
cargo test -p neocogi-com-core --target "$target" --no-default-features --locked

echo
echo "==> [32-bit ABI] generated Rust ABI called from native C ($target)"
cargo test \
  --manifest-path crates/com/api/integration-tests/fixtures/native-c-abi/Cargo.toml \
  --target "$target" \
  --target-dir target/native-c-abi-fixture-32 \
  --locked

echo
echo "32-bit ABI execution suite passed"
