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
  and .totals.manual_lab_case_count == 0
  and .totals.unsupported_case_count == 0
  and (.path_behavior_taxonomy.evidence_classes.manual_lab.claim_scope | contains("physical NAT"))
  and (.path_behavior_taxonomy.evidence_classes.unsupported.claim_scope | contains("must not be reported as pass"))
  and .path_behavior_taxonomy.proxy_claim_guard == true
  and .evidence_contract.claim_readiness.mixed_topology_full_tier_status == "required_plan"
  and (.evidence_contract.claim_readiness.stronger_full_tier_truth_blockers | index("run_full_tier_proxy_execution")) != null
  and any(.cases[]; .case_id == "bootstrap_poisoning_dedupe")
  and any(.cases[]; .case_id == "relay_budget_detection")
  and any(.cases[]; .case_id == "path_failover_selection")
  and all(.cases[]; has("reachability_pair") and has("degradation_class") and has("path_expectation") and has("expected_route") and has("supported_status") and has("claim_boundary"))
  and any(.cases[]; .case_id == "cgnat_relay_path_ranking" and .path_expectation == "may_direct_must_recover" and .expected_route == "prefer_direct_then_hole_punch_then_relay" and .degradation_class == "none" and .claim_boundary == "exact_path_ranking_not_physical_cgnat_truth")
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
  and .path_behavior_taxonomy.proxy_claim_guard == true
  and (.path_behavior_taxonomy.physical_nat_truth_requires | index("dedicated_lab_evidence_ref_or_real_env_ref")) != null
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
  and any(.cases[]; .case_id == "sentry_loss_proxy_longrun" and .path_expectation == "may_direct_must_recover" and .expected_route == "must_recover_via_remaining_paths" and .degradation_class == "sentry_loss" and .claim_boundary == "proxy_not_dedicated_sentry_or_nat_lab_truth")
  and any(.cases[]; .case_id == "mixed_topology_release_proxy" and .path_expectation == "may_direct_must_recover" and .expected_route == "must_recover_without_physical_nat_claim" and .degradation_class == "restart_pause_disconnect" and .claim_boundary == "proxy_not_physical_nat_or_cgnat_truth")
' "$full_summary" >/dev/null

echo "p2p mixed-topology matrix smoke checks passed"
