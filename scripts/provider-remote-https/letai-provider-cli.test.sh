#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

python_bin="${PYTHON_BIN:-}"
if [[ -z "$python_bin" ]]; then
  for candidate in python3.12 python3.11 python3.10 python3; do
    if command -v "$candidate" >/dev/null 2>&1; then
      if "$candidate" - <<'PY' >/dev/null 2>&1
import sys
raise SystemExit(0 if sys.version_info >= (3, 10) else 1)
PY
      then
        python_bin="$candidate"
        break
      fi
    fi
  done
fi

if [[ -z "$python_bin" ]]; then
  echo "error: provider remote HTTPS tests require Python 3.10+" >&2
  exit 1
fi

"$python_bin" scripts/provider-remote-https/letai_provider_cli.test.py
