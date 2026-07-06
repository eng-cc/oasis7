#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT

bash ./scripts/oasis7-node-observability-report.sh \
  --status-json-path fixtures/p2p_real_env_observability/local_status.json \
  --node-label local_node \
  --summary-json "$tmp_root/summary.json" \
  --summary-md "$tmp_root/summary.md"

python3 - "$tmp_root/summary.json" "$tmp_root/summary.md" <<'PY'
import json
import sys
from pathlib import Path

summary = json.loads(Path(sys.argv[1]).read_text())
markdown = Path(sys.argv[2]).read_text()

runtime_perf = summary["runtime_perf"]
assert runtime_perf["health"] == "warn"
assert runtime_perf["bottleneck"] == "decision"
assert runtime_perf["decision"]["p95_ms"] == 24.2
assert runtime_perf["decision"]["over_budget_ratio_ppm"] == 125000
assert "## Runtime Performance" in markdown
assert "health: `warn`" in markdown
assert "bottleneck: `decision`" in markdown
assert "decision: p95=`24.2`" in markdown
PY
