#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

run() {
  echo "+ $*"
  "$@"
}

smoke_root=".tmp/p2p_mixed_topology_smoke"
rm -rf "$smoke_root"
mkdir -p "$smoke_root"

run ./scripts/p2p-mixed-topology-matrix.sh \
  --tier required \
  --dry-run \
  --out-dir "$smoke_root/required"

required_summary=$(find "$smoke_root/required" -type f -name summary.json | sort | tail -n 1)
jq -e '
  .tier == "required"
  and .overall_status == "dry_run"
  and .totals.case_count == 7
  and .totals.proxy_case_count == 0
  and any(.cases[]; .case_id == "bootstrap_poisoning_dedupe")
  and any(.cases[]; .case_id == "relay_budget_detection")
  and any(.cases[]; .case_id == "path_failover_selection")
' "$required_summary" >/dev/null

run ./scripts/p2p-mixed-topology-matrix.sh \
  --tier full \
  --dry-run \
  --out-dir "$smoke_root/full"

full_summary=$(find "$smoke_root/full" -type f -name summary.json | sort | tail -n 1)
jq -e '
  .tier == "full"
  and .overall_status == "dry_run"
  and .totals.case_count == 9
  and .totals.proxy_case_count == 2
  and any(.cases[]; .case_id == "sentry_loss_proxy_longrun" and .coverage == "proxy")
  and any(.cases[]; .case_id == "mixed_topology_release_proxy" and .coverage == "proxy")
' "$full_summary" >/dev/null

echo "p2p mixed-topology matrix smoke checks passed"
