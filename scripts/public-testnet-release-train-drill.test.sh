#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

out_dir=".tmp/public-testnet-release-train-drill-test"
rm -rf "$out_dir"

bash scripts/public-testnet-release-train-drill.sh \
  --smoke \
  --window-id public-testnet-release-train-drill-smoke \
  --out-dir "$out_dir" >/dev/null

summary_json="$out_dir/public-testnet-release-train-drill-smoke/summary.json"

python3 - "$summary_json" <<'PY'
import json
import sys

summary = json.loads(open(sys.argv[1], encoding="utf-8").read())
positive = summary["positive_readiness"]
if positive["gate_result"] != "pass":
    raise SystemExit("positive readiness did not pass")
if positive["readiness_verdict"] != "ready_for_live_candidate":
    raise SystemExit("positive readiness verdict mismatch")
if positive["live_candidate_allowed"] is not True:
    raise SystemExit("positive readiness must allow controlled live candidate")
if positive["claim_recommendation"] != "allow_controlled_public_testnet_claims":
    raise SystemExit("positive claim recommendation mismatch")
if positive["required_lane_count"] != 11:
    raise SystemExit("expected 11 required lanes")
if positive["missing_required_lanes"]:
    raise SystemExit("positive readiness has missing lanes")

cases = {case["case_id"]: case for case in summary["negative_cases"]}
expected = {
    "freshness_drift_partial",
    "fork_readiness_drift_block",
    "missing_required_lane_block",
    "unsupported_manifest_promotion_gate_block",
}
if set(cases) != expected:
    raise SystemExit(f"negative case set mismatch: {sorted(cases)}")
for case in cases.values():
    if case["observed_result"] != "hold_or_block":
        raise SystemExit(f"negative case did not hold/block: {case['case_id']}")
    if case["live_candidate_allowed"] is not False:
        raise SystemExit(f"negative case allowed live candidate: {case['case_id']}")
    if case["claim_recommendation"] != "hold_public_testnet_claims":
        raise SystemExit(f"negative case claim recommendation mismatch: {case['case_id']}")
unsupported = cases["unsupported_manifest_promotion_gate_block"]
expected_blocker = "manifest_declares_unsupported_required_gates:unsupported_public_launch_gate"
if expected_blocker not in unsupported["manifest_blockers"]:
    raise SystemExit("unsupported promotion-gate negative did not report expected manifest blocker")

aggregate = summary["release_train_aggregate"]
if aggregate["mode"] != "synthetic_gate_smoke":
    raise SystemExit("smoke aggregate mode mismatch")
if aggregate["gate_result"] != "pass":
    raise SystemExit("rehearsal track gate did not pass in smoke")
if aggregate["promotion_recommendation"] != "eligible_for_promotion":
    raise SystemExit("rehearsal track promotion recommendation mismatch")
if aggregate["promotion_scope"] != "public_testnet_rehearsal_only":
    raise SystemExit("rehearsal track promotion scope mismatch")
if aggregate["launch_promotion_allowed"] is not False:
    raise SystemExit("rehearsal track must not allow launch promotion")
if summary["public_launch_allowed"] is not False:
    raise SystemExit("drill must not allow public launch")
denied = set(summary["claim_boundary"]["denied"])
for claim in ("mainnet_grade", "public_validator_admission", "live_arbitrary_state_proof"):
    if claim not in denied:
        raise SystemExit(f"missing denied claim: {claim}")
print("public-testnet release-train drill smoke passed")
PY
