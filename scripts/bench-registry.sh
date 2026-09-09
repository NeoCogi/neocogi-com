#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../../.."

cargo test -p neocogi-com-runtime --release --locked \
  type_registry::tests::registry_scaling_benchmark -- \
  --ignored --exact --nocapture

cargo test -p neocogi-com-runtime --release --locked \
  registration::tests::plugin_registration_scaling_benchmark -- \
  --ignored --exact --nocapture
