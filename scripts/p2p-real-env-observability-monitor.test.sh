#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT

python3 ./scripts/p2p-real-env-observability-summary.py \
  --snapshot-summary fixtures/p2p_real_env_observability/snapshot_summary.json \
  --host-summary fixtures/p2p_real_env_observability/host_summary.json \
  --traffic-summary fixtures/p2p_real_env_observability/traffic_summary.json \
  --observer-wasm-summary fixtures/p2p_real_env_observability/observer_wasm_summary.json \
  --sequencer-wasm-summary fixtures/p2p_real_env_observability/sequencer_wasm_summary.json \
  --storage-wasm-summary fixtures/p2p_real_env_observability/storage_wasm_summary.json \
  --summary-json "$tmp_root/summary.json" \
  --summary-md "$tmp_root/summary.md" \
  --run-id test-run \
  --run-dir "$tmp_root/run"

python3 - "$tmp_root/summary.json" <<'PY'
import json
import sys
from pathlib import Path

summary = json.loads(Path(sys.argv[1]).read_text())
assert summary["snapshot"]["claim_status"] == "pass_candidate"
assert summary["overall"]["status"] == "pass_with_resource_alerts"
assert "sequencer_ecs" in summary["host"]["aggregate"]["alerted_nodes"]
observer = summary["nodes"]["observer_local"]
assert observer["role"] == "observer"
assert observer["host"]["runtime_cpu_percent"] == 47.3
assert observer["wasm"]["top_hotspot"] == "executor.entrypoint_call_ms_total"
sequencer = summary["nodes"]["sequencer_ecs"]
assert "runtime_cpu_hot" in sequencer["alerts"]
assert sequencer["traffic"]["control_plane_total_events"] == 178
storage = summary["nodes"]["storage_ecs"]
assert storage["wasm"]["window_available"] is True
assert summary["traffic"]["aggregate"]["total_payload_bytes"] == 780646
PY
