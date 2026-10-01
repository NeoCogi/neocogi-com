#!/usr/bin/env bash
# The driver requires both instrumented deployments and reports both outcomes.
set -euo pipefail
exec python3 "$(dirname "${BASH_SOURCE[0]}")/compatibility.py" --suite sanitizers "$@"
