#!/usr/bin/env bash
# All build policy, artifact checks and test discovery live in the common driver.
set -euo pipefail
exec python3 "$(dirname "${BASH_SOURCE[0]}")/compatibility.py" --suite linked "$@"
