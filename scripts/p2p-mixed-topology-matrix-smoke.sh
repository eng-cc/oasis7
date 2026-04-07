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
  and .evidence_contract.claim_readiness.mixed_topology_full_tier_status == "required_plan"
  and (.evidence_contract.claim_readiness.stronger_full_tier_truth_blockers | index("run_full_tier_proxy_execution")) != null
  and any(.cases[]; .case_id == "bootstrap_poisoning_dedupe")
  and any(.cases[]; .case_id == "relay_budget_detection")
  and any(.cases[]; .case_id == "path_failover_selection")
' "$required_summary" >/dev/null

run ./scripts/p2p-mixed-topology-matrix.sh \
  --tier full \
  --shared-window-evidence-ref doc/testing/evidence/shared-network-shared-devnet-follow-up-window-2026-03-24.md \
  --shared-window-evidence-ref doc/testing/evidence/shared-network-shared-devnet-short-window-pass-2026-03-24.md \
  --dedicated-lab-evidence-ref doc/testing/evidence/dedicated-mixed-topology-lab-placeholder.md \
  --pass-uplift-decision-ref DEC-P2P-MIXED-DRYRUN \
  --dry-run \
  --out-dir "$smoke_root/full"

full_summary=$(find "$smoke_root/full" -type f -name summary.json | sort | tail -n 1)
jq -e '
  .tier == "full"
  and .overall_status == "dry_run"
  and .totals.case_count == 9
  and .totals.proxy_case_count == 2
  and .evidence_contract.claim_readiness.mixed_topology_full_tier_status == "full_proxy_plan"
  and (.external_evidence.shared_window_evidence_refs | length) == 2
  and (.external_evidence.dedicated_lab_evidence_refs | length) == 1
  and .external_evidence.pass_uplift_decision_ref == "DEC-P2P-MIXED-DRYRUN"
  and (.evidence_contract.claim_readiness.shared_network_pass_blockers | index("execute_full_tier_live_run")) != null
  and any(.cases[]; .case_id == "sentry_loss_proxy_longrun" and (.command | contains("--no-prewarm") | not))
  and any(.cases[]; .case_id == "mixed_topology_release_proxy" and (.command | contains("--no-prewarm") | not))
  and any(.cases[]; .case_id == "sentry_loss_proxy_longrun" and (.command | contains("--base-port 16610")))
  and any(.cases[]; .case_id == "mixed_topology_release_proxy" and (.command | contains("--base-port 17610")))
  and any(.cases[]; .case_id == "sentry_loss_proxy_longrun" and .coverage == "proxy")
  and any(.cases[]; .case_id == "mixed_topology_release_proxy" and .coverage == "proxy")
' "$full_summary" >/dev/null

echo "p2p mixed-topology matrix smoke checks passed"
