#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT_DIR"

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

sample_dir="$TMPDIR/sample-window"
sample_summary="$TMPDIR/sample-window-summary.json"
window_evidence="$TMPDIR/light-client-continuity-window-evidence.json"
lanes="$TMPDIR/lanes.tsv"
summary="$TMPDIR/readiness-summary.json"
stderr="$TMPDIR/stderr.txt"

cargo run -p oasis7_proto --quiet --bin oasis7_world_head_proof_verify -- \
  --write-sample-window-json "$sample_dir" \
  --json >"$sample_summary"

window_path=$(jq -r '.window_path' "$sample_summary")
anchor_hash=$(jq -r '.trusted_anchor.block_hash' "$sample_summary")

./scripts/network-tier-light-client-continuity-window.sh \
  --manifest doc/testing/templates/network-tier-public-testnet.example.json \
  --proof-window "$window_path" \
  --world-id world-a \
  --expect-from-height 40 \
  --expect-to-height 42 \
  --expect-anchor-hash "$anchor_hash" \
  --sample-started-at 2026-07-06T00:00:00Z \
  --sample-ended-at 2026-07-06T00:01:00Z \
  --out "$window_evidence" >/dev/null

jq -e '
  .evidence_schema == "oasis7.light_client_continuity_window.v1"
  and .status == "pass"
  and .verifier_mode == "proof_window_continuity"
  and .continuity_result == "accepted"
  and .verified_range.from_height == 40
  and .verified_range.to_height == 42
  and .verified_range.proof_count == 3
  and .window_verifier.claim_boundary == "proof_window_continuity_evidence_only_not_full_light_client_or_mainnet_readiness"
' "$window_evidence" >/dev/null

cat >"$lanes" <<EOF
light_client_continuity_window_ready	blockchain_ops_engineer	pass	$window_evidence	optional proof-window lane must not promote public_testnet readiness by itself
EOF

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest doc/testing/templates/network-tier-public-testnet.example.json \
  --lanes-tsv "$lanes" \
  --out-dir "$TMPDIR/readiness" >"$summary"

jq -e '
  .live_candidate_allowed == false
  and (.required_lanes | index("light_client_continuity_window_ready") | not)
  and (.ignored_lanes | any(.lane_id == "light_client_continuity_window_ready"))
' "$summary" >/dev/null

if ./scripts/network-tier-light-client-continuity-window.sh \
  --manifest doc/testing/templates/network-tier-public-testnet.example.json \
  --proof-window "$window_path" \
  --world-id world-a \
  --expect-from-height 40 \
  --expect-to-height 42 \
  --expect-anchor-hash wrong-anchor \
  --sample-started-at 2026-07-06T00:00:00Z \
  --sample-ended-at 2026-07-06T00:01:00Z \
  --out "$TMPDIR/should-not-write-anchor-mismatch.json" >"$TMPDIR/anchor-mismatch.stdout" 2>"$stderr"; then
  echo "expected wrapper to reject anchor mismatch" >&2
  exit 1
fi
grep -q "anchor hash mismatch" "$stderr"

run_invalid_readiness_case() {
  local case_name=$1
  local expected_error=$2
  local invalid_evidence="$TMPDIR/light-client-continuity-window-invalid-$case_name.json"
  python3 - "$window_evidence" "$invalid_evidence" "$case_name" <<'PY'
import json
import pathlib
import sys

data = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
case_name = sys.argv[3]

if case_name == "missing_denials":
    data["does_not_claim"] = ["ready_for_live_candidate"]
elif case_name == "db_access":
    data["node_db_access_used"] = True
elif case_name == "claim_boundary":
    data["window_verifier"]["claim_boundary"] = "full_light_client_ready"
elif case_name == "proof_count":
    data["verified_range"]["proof_count"] = 2
elif case_name == "head_mismatch":
    data["window_verifier"]["head"]["block_hash"] = "wrong-head"
elif case_name == "empty_proof_ref":
    data["proof_refs"][0] = ""
else:
    raise SystemExit(f"unknown invalid case: {case_name}")

pathlib.Path(sys.argv[2]).write_text(
    json.dumps(data, indent=2) + "\n",
    encoding="utf-8",
)
PY

  cat >"$lanes" <<EOF
light_client_continuity_window_ready	blockchain_ops_engineer	pass	$invalid_evidence	invalid proof-window lane must fail closed
EOF

  if ./scripts/network-tier-public-testnet-readiness.sh \
    --manifest doc/testing/templates/network-tier-public-testnet.example.json \
    --lanes-tsv "$lanes" \
    --out-dir "$TMPDIR/readiness-invalid-$case_name" \
    >"$TMPDIR/invalid-summary-$case_name.json" 2>"$stderr"; then
    echo "expected invalid light-client continuity window lane to fail: $case_name" >&2
    exit 1
  fi

  grep -q "$expected_error" "$stderr"
}

run_invalid_readiness_case "missing_denials" "light_client_continuity_window_ready does_not_claim missing"
run_invalid_readiness_case "db_access" "light_client_continuity_window_ready node_db_access_used must be false"
run_invalid_readiness_case "claim_boundary" "light_client_continuity_window_ready window_verifier.claim_boundary mismatch"
run_invalid_readiness_case "proof_count" "light_client_continuity_window_ready verified_range.proof_count must match height span"
run_invalid_readiness_case "head_mismatch" "light_client_continuity_window_ready window_verifier.head.block_hash must match observed_head.hash"
run_invalid_readiness_case "empty_proof_ref" "light_client_continuity_window_ready proof_refs cannot contain empty values"

echo "light-client continuity window lane test passed"
