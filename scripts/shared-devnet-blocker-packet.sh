#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage:
  ./scripts/shared-devnet-blocker-packet.sh \
    --window-id <id> \
    --candidate-bundle <bundle.json> \
    --candidate-gate-summary <summary.md> \
    --access-out <path> \
    --mixed-topology-out <path> \
    --rollback-out <path> \
    [shared access flags...] \
    [mixed-topology flags...] \
    [rollback flags...]

Purpose:
  Generate concrete markdown drafts for the current shared-devnet blockers:
  - shared access evidence
  - mixed-topology baseline evidence
  - rollback target evidence

Shared access flags:
  --viewer-url <url>
  --live-addr <host:port>
  --operator-contact-ref <ref>        Repeatable
  --independent-operator-ref <ref>    Repeatable
  --access-validated-by <text>
  --access-validated-at <text>
  --access-evidence-ref <ref>         Repeatable
  --access-lane-result <pass|partial|block>
  --access-reason <text>

Mixed-topology flags:
  --mixed-topology-baseline-ref <ref>
  --mixed-topology-shared-evidence-ref <ref>   Repeatable
  --mixed-topology-proxy-ref <ref>             Repeatable
  --mixed-topology-validated-by <text>
  --mixed-topology-validated-at <text>
  --mixed-topology-lane-result <pass|partial|block>
  --mixed-topology-reason <text>

Rollback flags:
  --fallback-candidate-bundle <bundle.json>
  --fallback-class <formal_pass_candidate|bootstrap_restore_ready>
  --fallback-gate-summary <summary.md>
  --fallback-owner-ref <ref>
  --restore-steps-ref <ref>           Repeatable
  --rollback-validated-by <text>
  --rollback-validated-at <text>
  --restoration-scope <text>
  --rollback-lane-result <pass|partial|block>
  --rollback-reason <text>

Examples:
  ./scripts/shared-devnet-blocker-packet.sh \
    --window-id shared-devnet-20260324-06 \
    --candidate-bundle output/release-candidates/shared-devnet-20260324-05.json \
    --candidate-gate-summary output/shared-network/shared-devnet-20260324-06/gate/shared_devnet-20260324-175501/summary.md \
    --access-out doc/testing/evidence/shared-network-shared-devnet-shared-access-draft-2026-03-24.md \
    --mixed-topology-out doc/testing/evidence/shared-network-shared-devnet-mixed-topology-draft-2026-04-03.md \
    --rollback-out doc/testing/evidence/shared-network-shared-devnet-rollback-target-draft-2026-03-24.md \
    --viewer-url https://example.invalid/viewer \
    --live-addr devnet.example.invalid:443 \
    --operator-contact-ref doc/ops/handoff.md \
    --independent-operator-ref doc/ops/oncall.md \
    --mixed-topology-baseline-ref doc/testing/evidence/p2p-mixed-topology-validation-matrix-2026-04-03.md \
    --fallback-candidate-bundle output/release-candidates/shared-devnet-20260324-05.json \
    --fallback-class bootstrap_restore_ready \
    --fallback-gate-summary output/shared-network/shared-devnet-20260324-06/gate/shared_devnet-20260324-175501/summary.md \
    --fallback-owner-ref doc/testing/evidence/shared-network-shared-devnet-short-window-promotion-record-2026-03-24.md
USAGE
}

require_non_empty() {
  local flag=$1
  local value=$2
  if [[ -z "$value" ]]; then
    echo "error: missing required option: $flag" >&2
    exit 2
  fi
}

require_file() {
  local flag=$1
  local value=$2
  if [[ ! -f "$value" ]]; then
    echo "error: $flag not found: $value" >&2
    exit 2
  fi
}

ensure_lane_result() {
  local flag=$1
  local value=$2
  case "$value" in
    pass|partial|block) ;;
    *)
      echo "error: unsupported $flag: $value" >&2
      exit 2
      ;;
  esac
}

bundle_field() {
  local bundle_path=$1
  local field=$2
  python3 - "$bundle_path" "$field" <<'PY'
import json
import sys

payload = json.load(open(sys.argv[1], encoding="utf-8"))
value = payload
for part in sys.argv[2].split("."):
    value = value.get(part) if isinstance(value, dict) else None
    if value is None:
        break
if value is None:
    print("")
elif isinstance(value, bool):
    print("true" if value else "false")
else:
    print(value)
PY
}

write_ref_block() {
  local file_path=$1
  shift
  local refs=("$@")
  if [[ "${#refs[@]}" -eq 0 ]]; then
    printf '  - `%s`\n' "<pending>" >>"$file_path"
    return
  fi
  local ref=""
  for ref in "${refs[@]}"; do
    printf '  - `%s`\n' "$ref" >>"$file_path"
  done
}

window_id=""
candidate_bundle=""
candidate_gate_summary=""
access_out=""
mixed_topology_out=""
rollback_out=""
viewer_url=""
live_addr=""
access_validated_by="<qa operator / runtime operator>"
access_validated_at="<YYYY-MM-DD HH:MM:SS TZ>"
access_lane_result="partial"
access_reason="shared access input is still draft; convert to pass only after independent operator access is verified"
mixed_topology_baseline_ref="doc/testing/evidence/p2p-mixed-topology-validation-matrix-2026-04-03.md"
mixed_topology_validated_by="<qa owner / runtime owner>"
mixed_topology_validated_at="<YYYY-MM-DD HH:MM:SS TZ>"
mixed_topology_lane_result="partial"
mixed_topology_reason="P2PARCH-6 matrix baseline is pinned, but same-window shared mixed-topology evidence is still missing or only proxy-level"
fallback_candidate_bundle=""
fallback_gate_summary=""
fallback_owner_ref=""
fallback_class="formal_pass_candidate"
rollback_validated_by="<liveops owner / runtime owner>"
rollback_validated_at="<YYYY-MM-DD HH:MM:SS TZ>"
restoration_scope="<runtime build | world snapshot | governance manifest>"
rollback_lane_result="partial"
rollback_reason="no audited formal fallback is pinned yet; for the first shared-devnet pass, provide a bootstrap_restore_ready fallback with restore steps, owner ref, and restoration scope"
declare -a operator_contact_refs=()
declare -a independent_operator_refs=()
declare -a access_evidence_refs=()
declare -a mixed_topology_shared_evidence_refs=()
declare -a mixed_topology_proxy_refs=()
declare -a restore_steps_refs=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --window-id)
      window_id=${2:-}
      shift 2
      ;;
    --candidate-bundle)
      candidate_bundle=${2:-}
      shift 2
      ;;
    --candidate-gate-summary)
      candidate_gate_summary=${2:-}
      shift 2
      ;;
    --access-out)
      access_out=${2:-}
      shift 2
      ;;
    --mixed-topology-out)
      mixed_topology_out=${2:-}
      shift 2
      ;;
    --rollback-out)
      rollback_out=${2:-}
      shift 2
      ;;
    --viewer-url)
      viewer_url=${2:-}
      shift 2
      ;;
    --live-addr)
      live_addr=${2:-}
      shift 2
      ;;
    --operator-contact-ref)
      operator_contact_refs+=("${2:-}")
      shift 2
      ;;
    --independent-operator-ref)
      independent_operator_refs+=("${2:-}")
      shift 2
      ;;
    --access-validated-by)
      access_validated_by=${2:-}
      shift 2
      ;;
    --access-validated-at)
      access_validated_at=${2:-}
      shift 2
      ;;
    --access-evidence-ref)
      access_evidence_refs+=("${2:-}")
      shift 2
      ;;
    --access-lane-result)
      access_lane_result=${2:-}
      shift 2
      ;;
    --access-reason)
      access_reason=${2:-}
      shift 2
      ;;
    --mixed-topology-baseline-ref)
      mixed_topology_baseline_ref=${2:-}
      shift 2
      ;;
    --mixed-topology-shared-evidence-ref)
      mixed_topology_shared_evidence_refs+=("${2:-}")
      shift 2
      ;;
    --mixed-topology-proxy-ref)
      mixed_topology_proxy_refs+=("${2:-}")
      shift 2
      ;;
    --mixed-topology-validated-by)
      mixed_topology_validated_by=${2:-}
      shift 2
      ;;
    --mixed-topology-validated-at)
      mixed_topology_validated_at=${2:-}
      shift 2
      ;;
    --mixed-topology-lane-result)
      mixed_topology_lane_result=${2:-}
      shift 2
      ;;
    --mixed-topology-reason)
      mixed_topology_reason=${2:-}
      shift 2
      ;;
    --fallback-candidate-bundle)
      fallback_candidate_bundle=${2:-}
      shift 2
      ;;
    --fallback-gate-summary)
      fallback_gate_summary=${2:-}
      shift 2
      ;;
    --fallback-owner-ref)
      fallback_owner_ref=${2:-}
      shift 2
      ;;
    --fallback-class)
      fallback_class=${2:-}
      shift 2
      ;;
    --restore-steps-ref)
      restore_steps_refs+=("${2:-}")
      shift 2
      ;;
    --rollback-validated-by)
      rollback_validated_by=${2:-}
      shift 2
      ;;
    --rollback-validated-at)
      rollback_validated_at=${2:-}
      shift 2
      ;;
    --restoration-scope)
      restoration_scope=${2:-}
      shift 2
      ;;
    --rollback-lane-result)
      rollback_lane_result=${2:-}
      shift 2
      ;;
    --rollback-reason)
      rollback_reason=${2:-}
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

require_non_empty "--window-id" "$window_id"
require_non_empty "--candidate-bundle" "$candidate_bundle"
require_non_empty "--candidate-gate-summary" "$candidate_gate_summary"
require_non_empty "--access-out" "$access_out"
require_non_empty "--mixed-topology-out" "$mixed_topology_out"
require_non_empty "--rollback-out" "$rollback_out"
require_file "--candidate-bundle" "$candidate_bundle"
require_file "--candidate-gate-summary" "$candidate_gate_summary"
ensure_lane_result "--access-lane-result" "$access_lane_result"
ensure_lane_result "--mixed-topology-lane-result" "$mixed_topology_lane_result"
ensure_lane_result "--rollback-lane-result" "$rollback_lane_result"
if [[ -n "$mixed_topology_baseline_ref" ]]; then
  require_file "--mixed-topology-baseline-ref" "$mixed_topology_baseline_ref"
fi

./scripts/release-candidate-bundle.sh validate --bundle "$candidate_bundle" >/dev/null
candidate_id=$(bundle_field "$candidate_bundle" "candidate_id")

fallback_candidate_id="<previous-pass-candidate-id>"
if [[ -n "$fallback_candidate_bundle" ]]; then
  require_file "--fallback-candidate-bundle" "$fallback_candidate_bundle"
  ./scripts/release-candidate-bundle.sh validate --bundle "$fallback_candidate_bundle" >/dev/null
  fallback_candidate_id=$(bundle_field "$fallback_candidate_bundle" "candidate_id")
fi
if [[ -n "$fallback_gate_summary" ]]; then
  require_file "--fallback-gate-summary" "$fallback_gate_summary"
fi
case "$fallback_class" in
  formal_pass_candidate|bootstrap_restore_ready)
    ;;
  *)
    echo "error: unsupported --fallback-class: $fallback_class" >&2
    exit 2
    ;;
esac

mkdir -p "$(dirname "$access_out")" "$(dirname "$mixed_topology_out")" "$(dirname "$rollback_out")"

cat >"$access_out" <<EOF
# Shared Network Shared Access Check

审计轮次: 1

## Meta
- \`window_id\`:
  - \`$window_id\`
- \`track\`:
  - \`shared_devnet\`
- \`candidate_id\`:
  - \`$candidate_id\`
- \`owner\`:
  - \`qa_engineer\`

## Shared Endpoint
- \`viewer_url\`:
  - \`${viewer_url:-<https://... | http://...>}\`
- \`live_addr\`:
  - \`${live_addr:-<host:port>}\`
- \`operator_contact_ref\`:
EOF
write_ref_block "$access_out" "${operator_contact_refs[@]}"
cat >>"$access_out" <<EOF
- \`independent_operator_ref\`:
EOF
write_ref_block "$access_out" "${independent_operator_refs[@]}"
cat >>"$access_out" <<EOF

## Access Validation
- \`access_mode\`:
  - \`shared_multi_operator\`
- \`validated_by\`:
  - \`$access_validated_by\`
- \`validated_at\`:
  - \`$access_validated_at\`
- \`validation_steps\`:
  - \`independent operator opened viewer endpoint\`
  - \`independent operator reached live endpoint\`
  - \`candidate_id matched bundle truth\`
- \`candidate_bundle_ref\`:
  - \`$candidate_bundle\`
- \`candidate_gate_summary_ref\`:
  - \`$candidate_gate_summary\`
- \`evidence_ref\`:
EOF
write_ref_block "$access_out" "${access_evidence_refs[@]}"
cat >>"$access_out" <<EOF

## Verdict
- \`lane_result\`:
  - \`$access_lane_result\`
- \`reason\`:
  - $access_reason

## Notes
- \`pass\` only if access is not single-owner local-only rehearsal.
- \`partial\` if endpoint exists but still depends on one local operator or one private machine.
- \`block\` if endpoint is unreachable, candidate truth mismatches, or owner handoff is missing.
EOF

cat >"$mixed_topology_out" <<EOF
# Shared Network Mixed-Topology Gate Check

审计轮次: 1

## Meta
- \`window_id\`:
  - \`$window_id\`
- \`track\`:
  - \`shared_devnet\`
- \`candidate_id\`:
  - \`$candidate_id\`
- \`owner\`:
  - \`qa_engineer\`

## Candidate Truth
- \`candidate_bundle_ref\`:
  - \`$candidate_bundle\`
- \`candidate_gate_summary_ref\`:
  - \`$candidate_gate_summary\`

## Mixed-Topology Inputs
- \`baseline_evidence_ref\`:
  - \`${mixed_topology_baseline_ref:-<doc/testing/evidence/p2p-mixed-topology-validation-matrix-YYYY-MM-DD.md>}\`
- \`same_window_shared_evidence_ref\`:
EOF
write_ref_block "$mixed_topology_out" "${mixed_topology_shared_evidence_refs[@]}"
cat >>"$mixed_topology_out" <<EOF
- \`proxy_drill_ref\`:
EOF
write_ref_block "$mixed_topology_out" "${mixed_topology_proxy_refs[@]}"
cat >>"$mixed_topology_out" <<EOF

## Validation
- \`validated_by\`:
  - \`$mixed_topology_validated_by\`
- \`validated_at\`:
  - \`$mixed_topology_validated_at\`
- \`validation_expectations\`:
  - \`baseline candidate_id and role boundary still match current shared-devnet bundle truth\`
  - \`same-window mixed-topology evidence is explicitly linked when claiming pass\`
  - \`pass uplift includes an audited producer/QA decision ref, not just a lane status flip\`
  - \`proxy drill evidence is called out as approximation, not dedicated sentry/NAT lab truth\`

## Verdict
- \`lane_result\`:
  - \`$mixed_topology_lane_result\`
- \`reason\`:
  - $mixed_topology_reason

## Notes
- \`pass\` only if same-window shared mixed-topology evidence is pinned, reviewed against the current candidate truth, and backed by an audited producer/QA pass-uplift decision ref.
- \`partial\` if only the P2PARCH-6 baseline or proxy drill evidence is available.
- \`block\` if there is no credible mixed-topology basis for the current candidate or the evidence contradicts the gate claim.
EOF

cat >"$rollback_out" <<EOF
# Shared Network Rollback Target

审计轮次: 1

## Meta
- \`window_id\`:
  - \`$window_id\`
- \`track\`:
  - \`shared_devnet\`
- \`candidate_id\`:
  - \`$candidate_id\`
- \`owner\`:
  - \`liveops_community\`

## Current Candidate
- \`candidate_bundle_ref\`:
  - \`$candidate_bundle\`
- \`candidate_gate_ref\`:
  - \`$candidate_gate_summary\`

## Fallback Candidate
- \`fallback_candidate_id\`:
  - \`$fallback_candidate_id\`
- \`fallback_class\`:
  - \`$fallback_class\`
- \`fallback_candidate_bundle_ref\`:
  - \`${fallback_candidate_bundle:-<output/release-candidates/fallback.json>}\`
- \`fallback_gate_ref\`:
  - \`${fallback_gate_summary:-<output/shared-network/.../gate/.../summary.md>}\`
- \`fallback_track_result\`:
  - \`pass\`
- \`fallback_owner_ref\`:
  - \`${fallback_owner_ref:-<promotion record | incident review | approval record>}\`

## Rollback Readiness
- \`restore_steps_ref\`:
EOF
write_ref_block "$rollback_out" "${restore_steps_refs[@]}"
cat >>"$rollback_out" <<EOF
- \`validated_by\`:
  - \`$rollback_validated_by\`
- \`validated_at\`:
  - \`$rollback_validated_at\`
- \`restoration_scope\`:
  - \`$restoration_scope\`

## Verdict
- \`lane_result\`:
  - \`$rollback_lane_result\`
- \`reason\`:
  - $rollback_reason

## Notes
- \`pass\` if:
  - \`fallback_class=formal_pass_candidate\` and fallback candidate is a formal previous shared-devnet \`pass\` candidate; or
  - \`fallback_class=bootstrap_restore_ready\`, current track is still pursuing the first shared-devnet \`pass\`, and \`restore_steps_ref\` + \`fallback_owner_ref\` + \`restoration_scope\` are all pinned and audited.
- \`partial\` if there is only a local/provisional fallback, or a bootstrap fallback is named but the audited restore contract is incomplete.
- \`block\` if fallback truth is missing, inconsistent, or not restorable.
EOF

echo "shared access draft: $access_out"
echo "mixed-topology draft: $mixed_topology_out"
echo "rollback target draft: $rollback_out"
