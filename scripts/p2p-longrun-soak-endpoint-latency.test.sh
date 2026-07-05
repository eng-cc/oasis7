#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if (( BASH_VERSINFO[0] < 4 )); then
  for candidate in /opt/homebrew/bin/bash /usr/local/bin/bash; do
    if [[ -x "$candidate" ]]; then
      exec "$candidate" "$0" "$@"
    fi
  done
fi

tmp_root=$(mktemp -d "${TMPDIR:-/tmp}/oasis7-p2p-soak-endpoint-latency.XXXXXX")
trap 'rm -rf "$tmp_root"' EXIT

"$BASH" ./scripts/p2p-longrun-soak.sh \
  --dry-run \
  --topologies triad \
  --duration-secs 1 \
  --out-dir "$tmp_root/out"

summary_json=$(find "$tmp_root/out" -type f -name summary.json | sort | tail -n 1)
[[ -n "$summary_json" && -f "$summary_json" ]] || {
  echo "missing summary.json" >&2
  exit 1
}

python3 - "$summary_json" <<'PY'
import json
import pathlib
import sys

summary = json.loads(pathlib.Path(sys.argv[1]).read_text())
topology = summary["topologies"][0]
latency = topology["metrics"]["endpoint_latency"]
for endpoint in ("health", "status", "balances"):
    metrics = latency[endpoint]
    assert metrics["sample_count"] == 0
    assert metrics["p50_ms"] == 0
    assert metrics["p95_ms"] == 0
    assert metrics["max_ms"] == 0
PY

grep -Fq "health_p95_ms" "$(jq -r '.artifacts.summary_md' "$summary_json")"

echo "p2p longrun endpoint latency summary schema smoke checks passed"
