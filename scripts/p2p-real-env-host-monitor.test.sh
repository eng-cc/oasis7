#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT

python3 ./scripts/p2p-real-env-host-summary.py \
  --history-path fixtures/p2p_real_env_host_monitor/history.ndjson \
  --summary-json "$tmp_root/summary.json" \
  --summary-md "$tmp_root/summary.md" \
  --run-id test-run \
  --run-dir "$tmp_root/run"

python3 - "$tmp_root/summary.json" <<'PY'
import json
import sys
from pathlib import Path

summary = json.loads(Path(sys.argv[1]).read_text())
assert summary["aggregate"]["node_count"] == 3
assert summary["aggregate"]["alerted_node_count"] == 1
assert summary["aggregate"]["highest_runtime_cpu_node"] == "sequencer_ecs"
local_node = summary["nodes"]["local_node"]
assert local_node["status"]["runtime_cpu"] == "normal"
assert round(local_node["latest"]["runtime_cpu_percent"], 1) == 47.3
sequencer = summary["nodes"]["sequencer_ecs"]
assert sequencer["status"]["runtime_cpu"] == "hot"
assert "runtime_cpu_hot" in sequencer["alerts"]
assert round(sequencer["latest"]["runtime_cpu_core_ratio"], 3) == 0.785
storage = summary["nodes"]["storage_ecs"]
assert storage["status"]["memory"] == "normal"
assert round(storage["latest"]["mem_available_percent"], 1) == 55.0
PY
