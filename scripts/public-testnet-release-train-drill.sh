#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage:
  ./scripts/public-testnet-release-train-drill.sh [options]

Purpose:
  Run a controlled public_testnet release-train drill by composing the existing
  public_testnet readiness gate, fail-closed negative readiness packets, and the
  public_testnet_rehearsal track gate.

Options:
  --manifest <path>                 public_testnet manifest
  --lanes-tsv <path>                current public_testnet readiness lanes TSV
  --out-dir <path>                  output directory
  --window-id <id>                  drill window id
  --runtime-build-ref <path>        runtime build artifact for rehearsal bundle
  --world-snapshot-ref <path>       world snapshot artifact for rehearsal bundle
  --governance-manifest-ref <path>  governance manifest for rehearsal bundle
  --rehearsal-lanes-tsv <path>      operator-supplied public_testnet_rehearsal lanes
  --allow-dirty-worktree            allow local-only candidate bundle creation
  --smoke                           create synthetic local artifacts for smoke
  -h, --help                        show help

Default manifest:
  doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json

Default lanes:
  doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.tsv
USAGE
}

manifest_path="doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json"
lanes_tsv="doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.tsv"
out_dir="output/public-testnet-release-train-drill"
window_id=""
runtime_build_ref=""
world_snapshot_ref=""
governance_manifest_ref=""
rehearsal_lanes_input=""
allow_dirty_worktree=0
smoke=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --manifest)
      manifest_path=${2:-}
      shift 2
      ;;
    --lanes-tsv)
      lanes_tsv=${2:-}
      shift 2
      ;;
    --out-dir)
      out_dir=${2:-}
      shift 2
      ;;
    --window-id)
      window_id=${2:-}
      shift 2
      ;;
    --runtime-build-ref)
      runtime_build_ref=${2:-}
      shift 2
      ;;
    --world-snapshot-ref)
      world_snapshot_ref=${2:-}
      shift 2
      ;;
    --governance-manifest-ref)
      governance_manifest_ref=${2:-}
      shift 2
      ;;
    --rehearsal-lanes-tsv)
      rehearsal_lanes_input=${2:-}
      shift 2
      ;;
    --allow-dirty-worktree)
      allow_dirty_worktree=1
      shift
      ;;
    --smoke)
      smoke=1
      allow_dirty_worktree=1
      shift
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

require_file() {
  local flag=$1
  local value=$2
  if [[ ! -f "$value" ]]; then
    echo "error: $flag not found: $value" >&2
    exit 2
  fi
}

require_path() {
  local flag=$1
  local value=$2
  if [[ ! -e "$value" ]]; then
    echo "error: $flag not found: $value" >&2
    exit 2
  fi
}

require_file "--manifest" "$manifest_path"
require_file "--lanes-tsv" "$lanes_tsv"

if [[ -z "$window_id" ]]; then
  window_id="public-testnet-release-train-drill-$(date '+%Y%m%d-%H%M%S')"
fi

run_dir="$out_dir/$window_id"
mkdir -p "$run_dir"

summary_json="$run_dir/summary.json"
summary_md="$run_dir/summary.md"
negative_dir="$run_dir/negative-packets"
mkdir -p "$negative_dir"

latest_summary_json() {
  local search_dir=$1
  python3 - "$search_dir" <<'PY'
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
summaries = sorted(root.glob("**/summary.json"), key=lambda path: path.stat().st_mtime)
if not summaries:
    raise SystemExit(f"no summary.json found under {root}")
print(summaries[-1])
PY
}

assert_positive_readiness() {
  local summary=$1
  python3 - "$summary" <<'PY'
import json
import sys

summary = json.loads(open(sys.argv[1], encoding="utf-8").read())
required = [
    "public_rpc_ready",
    "explorer_public_ready",
    "faucet_guard_ready",
    "reset_policy_announced",
    "runtime_bootstrap",
    "claims_boundary_review",
    "world_resource_provenance_ready",
    "provider_resource_provenance_ready",
    "resource_delta_replay_ready",
    "api_viewer_projection_ready",
    "same_world_hosted_entry_ready",
]
if summary.get("gate_result") != "pass":
    raise SystemExit(f"positive readiness gate_result must pass: {summary.get('gate_result')}")
if summary.get("readiness_verdict") != "ready_for_live_candidate":
    raise SystemExit("positive readiness verdict must be ready_for_live_candidate")
if summary.get("live_candidate_allowed") is not True:
    raise SystemExit("positive readiness must allow live candidate")
if summary.get("claim_recommendation") != "allow_controlled_public_testnet_claims":
    raise SystemExit("positive readiness claim recommendation mismatch")
lanes = {item.get("lane_id"): item for item in summary.get("lanes", [])}
missing = [lane_id for lane_id in required if lane_id not in lanes]
if missing:
    raise SystemExit("positive readiness missing required lanes: " + ",".join(missing))
not_pass = [lane_id for lane_id in required if lanes[lane_id].get("status") != "pass"]
if not_pass:
    raise SystemExit("positive readiness required lanes not pass: " + ",".join(not_pass))
print("positive readiness asserted")
PY
}

assert_negative_readiness() {
  local case_id=$1
  local summary=$2
  python3 - "$case_id" "$summary" <<'PY'
import json
import sys

case_id = sys.argv[1]
summary = json.loads(open(sys.argv[2], encoding="utf-8").read())
if summary.get("gate_result") == "pass":
    raise SystemExit(f"{case_id} unexpectedly passed")
if summary.get("live_candidate_allowed") is not False:
    raise SystemExit(f"{case_id} must deny live candidate")
if summary.get("claim_recommendation") != "hold_public_testnet_claims":
    raise SystemExit(f"{case_id} must hold public testnet claims")
print(f"{case_id} asserted fail-closed")
PY
}

positive_out_dir="$run_dir/readiness/positive"
./scripts/network-tier-public-testnet-readiness.sh \
  --manifest "$manifest_path" \
  --lanes-tsv "$lanes_tsv" \
  --out-dir "$positive_out_dir" >/dev/null
positive_summary_json=$(latest_summary_json "$positive_out_dir")
assert_positive_readiness "$positive_summary_json" >/dev/null

python3 - "$manifest_path" "$lanes_tsv" "$negative_dir" <<'PY'
import csv
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
lanes_path = pathlib.Path(sys.argv[2])
negative_dir = pathlib.Path(sys.argv[3])

def resolve_ref(raw: str) -> pathlib.Path:
    path = pathlib.Path(raw)
    if path.is_absolute():
        return path.resolve()
    manifest_relative = (manifest_path.parent / path).resolve()
    if manifest_relative.exists():
        return manifest_relative
    return (pathlib.Path.cwd() / path).resolve()

with lanes_path.open("r", encoding="utf-8", newline="") as fh:
    rows = list(csv.reader(fh, delimiter="\t"))

def write_tsv(path: pathlib.Path, rows_to_write):
    with path.open("w", encoding="utf-8", newline="") as fh:
        writer = csv.writer(fh, delimiter="\t", lineterminator="\n")
        writer.writerows(rows_to_write)

def replace_status(lane_id: str, status: str, note: str):
    out = []
    changed = False
    for row in rows:
        if row and not row[0].startswith("#") and row[0] == lane_id:
            new_row = list(row)
            new_row[2] = status
            new_row[4] = note
            out.append(new_row)
            changed = True
        else:
            out.append(row)
    if not changed:
        raise SystemExit(f"lane not found for negative packet: {lane_id}")
    return out

write_tsv(
    negative_dir / "freshness-drift.partial.tsv",
    replace_status(
        "public_rpc_ready",
        "partial",
        "synthetic_negative_gate_smoke: public RPC freshness re-sampling drift must hold promotion",
    ),
)

write_tsv(
    negative_dir / "fork-readiness-drift.block.tsv",
    replace_status(
        "api_viewer_projection_ready",
        "block",
        "synthetic_negative_gate_smoke: API/viewer same-window fork-like projection drift must block promotion",
    ),
)

missing_rows = [
    row for row in rows
    if not (row and not row[0].startswith("#") and row[0] == "same_world_hosted_entry_ready")
]
write_tsv(negative_dir / "missing-required-lane.block.tsv", missing_rows)

manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
promotion_policy = manifest.setdefault("promotion_policy", {})
required_gates = promotion_policy.setdefault("required_gates", [])
if "unsupported_public_launch_gate" not in required_gates:
    required_gates.append("unsupported_public_launch_gate")
(negative_dir / "unsupported-promotion-gate.manifest.json").write_text(
    json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
    encoding="utf-8",
)

unsupported_rows = []
same_world_evidence_cloned = False
for row in rows:
    if row and not row[0].startswith("#") and row[0] == "same_world_hosted_entry_ready":
        source = resolve_ref(row[3])
        evidence = json.loads(source.read_text(encoding="utf-8"))
        evidence["manifest_ref"] = str(negative_dir / "unsupported-promotion-gate.manifest.json")
        cloned_ref = negative_dir / "unsupported-promotion-gate.same-world-hosted-entry.json"
        cloned_ref.write_text(
            json.dumps(evidence, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        new_row = list(row)
        new_row[3] = str(cloned_ref)
        new_row[4] = "synthetic_negative_gate_smoke: all lanes stay pass while manifest declares unsupported promotion gate"
        unsupported_rows.append(new_row)
        same_world_evidence_cloned = True
    else:
        unsupported_rows.append(row)
if not same_world_evidence_cloned:
    raise SystemExit("same_world_hosted_entry_ready lane not found for unsupported gate negative")
write_tsv(negative_dir / "unsupported-promotion-gate.pass-lanes.tsv", unsupported_rows)
PY

declare -a negative_cases=(
  "freshness_drift_partial|$manifest_path|$negative_dir/freshness-drift.partial.tsv"
  "fork_readiness_drift_block|$manifest_path|$negative_dir/fork-readiness-drift.block.tsv"
  "missing_required_lane_block|$manifest_path|$negative_dir/missing-required-lane.block.tsv"
  "unsupported_manifest_promotion_gate_block|$negative_dir/unsupported-promotion-gate.manifest.json|$negative_dir/unsupported-promotion-gate.pass-lanes.tsv"
)

negative_summaries_tsv="$run_dir/negative-summaries.tsv"
: >"$negative_summaries_tsv"

for item in "${negative_cases[@]}"; do
  IFS='|' read -r case_id case_manifest case_lanes <<<"$item"
  case_out_dir="$run_dir/readiness/negative/$case_id"
  ./scripts/network-tier-public-testnet-readiness.sh \
    --manifest "$case_manifest" \
    --lanes-tsv "$case_lanes" \
    --out-dir "$case_out_dir" >/dev/null
  case_summary=$(latest_summary_json "$case_out_dir")
  assert_negative_readiness "$case_id" "$case_summary" >/dev/null
  printf '%s\t%s\t%s\t%s\n' "$case_id" "$case_manifest" "$case_lanes" "$case_summary" >>"$negative_summaries_tsv"
done

if [[ "$smoke" -eq 1 ]]; then
  smoke_dir="$run_dir/smoke-artifacts"
  mkdir -p "$smoke_dir/runtime" "$smoke_dir/world" "$smoke_dir/governance"
  printf 'synthetic release-train smoke runtime artifact\n' >"$smoke_dir/runtime/oasis7_chain_runtime"
  printf 'synthetic release-train smoke world snapshot\n' >"$smoke_dir/world/snapshot.txt"
  cat >"$smoke_dir/governance/public_manifest.json" <<'JSON'
{
  "schema_version": "oasis7.synthetic_public_testnet_governance.v1",
  "mode": "synthetic_negative_gate_smoke",
  "claim_boundary": "local drill artifact only; not live public launch evidence"
}
JSON
  runtime_build_ref="$smoke_dir/runtime/oasis7_chain_runtime"
  world_snapshot_ref="$smoke_dir/world/snapshot.txt"
  governance_manifest_ref="$smoke_dir/governance/public_manifest.json"
fi

aggregate_mode="skipped_missing_candidate_artifacts"
candidate_bundle_ref=""
rehearsal_lanes_tsv=""
track_summary_json=""
track_summary_md=""
track_gate_result="skipped"
track_promotion_recommendation="hold_promotion"

if [[ -n "$runtime_build_ref" || -n "$world_snapshot_ref" || -n "$governance_manifest_ref" ]]; then
  require_file "--runtime-build-ref" "$runtime_build_ref"
  require_path "--world-snapshot-ref" "$world_snapshot_ref"
  require_file "--governance-manifest-ref" "$governance_manifest_ref"

  aggregate_mode="synthetic_gate_smoke"
  if [[ "$smoke" -eq 0 ]]; then
    aggregate_mode="operator_supplied_candidate_refs"
    if [[ -z "$rehearsal_lanes_input" ]]; then
      aggregate_mode="operator_supplied_candidate_refs_missing_rehearsal_lanes"
    else
      require_file "--rehearsal-lanes-tsv" "$rehearsal_lanes_input"
    fi
  fi

  candidate_bundle_ref="$run_dir/release-candidate-bundle.json"
  create_args=(
    ./scripts/release-candidate-bundle.sh create
    --bundle "$candidate_bundle_ref"
    --candidate-id "$window_id"
    --track public_testnet_rehearsal
    --runtime-build-ref "$runtime_build_ref"
    --world-snapshot-ref "$world_snapshot_ref"
    --governance-manifest-ref "$governance_manifest_ref"
    --evidence-ref "$positive_summary_json"
    --evidence-ref "$negative_summaries_tsv"
    --note "public_testnet release-train drill; controlled live-candidate evidence only"
  )
  if [[ "$allow_dirty_worktree" -eq 1 ]]; then
    create_args+=(--allow-dirty-worktree)
  fi
  "${create_args[@]}" >/dev/null

  if [[ "$smoke" -eq 1 ]]; then
    track_evidence_dir="$run_dir/rehearsal-track-evidence"
    mkdir -p "$track_evidence_dir"
    for lane_id in \
      candidate_bundle_integrity \
      shared_access \
      multi_entry_closure \
      mixed_topology_baseline \
      governance_live_drill \
      short_window_longrun \
      rollback_target_ready
    do
      {
        printf '# %s\n\n' "$lane_id"
        printf -- '- mode: `%s`\n' "$aggregate_mode"
        printf -- '- positive_readiness_summary: `%s`\n' "$positive_summary_json"
        printf -- '- negative_summaries_tsv: `%s`\n' "$negative_summaries_tsv"
        printf -- '- claim_boundary: controlled public_testnet live-candidate drill only; not mainnet or live public launch evidence\n'
      } >"$track_evidence_dir/$lane_id.md"
    done

    rehearsal_lanes_tsv="$run_dir/public-testnet-rehearsal-lanes.tsv"
    {
      printf 'candidate_bundle_integrity\tblockchain_ops_engineer\tpass\t%s\tcandidate bundle validates for local release-train drill smoke\n' "$track_evidence_dir/candidate_bundle_integrity.md"
      printf 'shared_access\tblockchain_ops_engineer\tpass\t%s\tcurrent public endpoints represented by 11-lane readiness packet\n' "$track_evidence_dir/shared_access.md"
      printf 'multi_entry_closure\truntime_engineer\tpass\t%s\tAPI/viewer/same-world lanes linked by positive readiness packet\n' "$track_evidence_dir/multi_entry_closure.md"
      printf 'mixed_topology_baseline\tblockchain_ops_engineer\tpass\t%s\tmixed-topology lane is synthetic local gate smoke evidence\n' "$track_evidence_dir/mixed_topology_baseline.md"
      printf 'governance_live_drill\tblockchain_ops_engineer\tpass\t%s\tgovernance manifest ref included in synthetic candidate bundle for smoke\n' "$track_evidence_dir/governance_live_drill.md"
      printf 'short_window_longrun\tqa_engineer\tpass\t%s\tshort-window release-train gate path exercised as synthetic local smoke\n' "$track_evidence_dir/short_window_longrun.md"
      printf 'rollback_target_ready\tblockchain_ops_engineer\tpass\t%s\tsynthetic freshness/fork/readiness negatives consumed; real rollback evidence still required before launch\n' "$track_evidence_dir/rollback_target_ready.md"
    } >"$rehearsal_lanes_tsv"
  elif [[ -n "$rehearsal_lanes_input" ]]; then
    rehearsal_lanes_tsv="$rehearsal_lanes_input"
  fi

  if [[ -n "$rehearsal_lanes_tsv" ]]; then
    track_out_dir="$run_dir/rehearsal-track-gate"
    ./scripts/network-rehearsal-track-gate.sh \
      --track public_testnet_rehearsal \
      --candidate-bundle "$candidate_bundle_ref" \
      --lanes-tsv "$rehearsal_lanes_tsv" \
      --out-dir "$track_out_dir" >/dev/null
    track_summary_json=$(latest_summary_json "$track_out_dir")
    track_summary_md=${track_summary_json%.json}.md
    read -r track_gate_result track_promotion_recommendation < <(
      python3 - "$track_summary_json" <<'PY'
import json
import sys
summary = json.loads(open(sys.argv[1], encoding="utf-8").read())
print(summary.get("gate_result", "unknown"), summary.get("promotion_recommendation", "unknown"))
PY
    )
  else
    track_gate_result="skipped_missing_rehearsal_lanes"
    track_promotion_recommendation="hold_promotion"
  fi
fi

python3 - \
  "$window_id" \
  "$manifest_path" \
  "$lanes_tsv" \
  "$positive_summary_json" \
  "$negative_summaries_tsv" \
  "$aggregate_mode" \
  "$candidate_bundle_ref" \
  "$rehearsal_lanes_tsv" \
  "$track_summary_json" \
  "$track_summary_md" \
  "$track_gate_result" \
  "$track_promotion_recommendation" \
  "$summary_json" \
  "$summary_md" <<'PY'
import json
import pathlib
import sys
from datetime import datetime, timezone

(
    window_id,
    manifest_path,
    lanes_tsv,
    positive_summary_json,
    negative_summaries_tsv,
    aggregate_mode,
    candidate_bundle_ref,
    rehearsal_lanes_tsv,
    track_summary_json,
    track_summary_md,
    track_gate_result,
    track_promotion_recommendation,
    summary_json,
    summary_md,
) = sys.argv[1:]

positive = json.loads(pathlib.Path(positive_summary_json).read_text(encoding="utf-8"))
negative_cases = []
for line in pathlib.Path(negative_summaries_tsv).read_text(encoding="utf-8").splitlines():
    case_id, case_manifest, case_lanes, case_summary = line.split("\t")
    case_data = json.loads(pathlib.Path(case_summary).read_text(encoding="utf-8"))
    negative_cases.append(
        {
            "case_id": case_id,
            "packet_manifest_ref": case_manifest,
            "packet_lanes_tsv_ref": case_lanes,
            "summary_ref": case_summary,
            "expected_result": "hold_or_block",
            "observed_result": "hold_or_block" if case_data.get("gate_result") != "pass" else "unexpected_pass",
            "gate_result": case_data.get("gate_result"),
            "readiness_verdict": case_data.get("readiness_verdict"),
            "live_candidate_allowed": case_data.get("live_candidate_allowed"),
            "claim_recommendation": case_data.get("claim_recommendation"),
            "missing_required_lanes": case_data.get("missing_required_lanes", []),
            "manifest_blockers": case_data.get("manifest_blockers", []),
        }
    )

summary = {
    "schema_version": "oasis7.public_testnet_release_train_drill.v1",
    "window_id": window_id,
    "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "manifest_ref": manifest_path,
    "lanes_tsv_ref": lanes_tsv,
    "claim_boundary": {
        "allowed": [
            "controlled_public_testnet_live_candidate_only",
            "non_mainnet",
            "resettable",
            "guarded_faucet",
        ],
        "denied": [
            "live_public_testnet_already_online",
            "mainnet_grade",
            "production_oc_settlement",
            "public_validator_admission",
            "full_light_client",
            "live_arbitrary_state_proof",
            "multi_client_equivalence",
        ],
    },
    "positive_readiness": {
        "summary_ref": positive_summary_json,
        "gate_result": positive.get("gate_result"),
        "readiness_verdict": positive.get("readiness_verdict"),
        "live_candidate_allowed": positive.get("live_candidate_allowed"),
        "claim_recommendation": positive.get("claim_recommendation"),
        "lane_count": positive.get("lane_count"),
        "required_lane_count": len(positive.get("required_lanes", [])),
        "missing_required_lanes": positive.get("missing_required_lanes", []),
    },
    "negative_cases": negative_cases,
    "release_train_aggregate": {
        "mode": aggregate_mode,
        "candidate_bundle_ref": candidate_bundle_ref or None,
        "lanes_tsv_ref": rehearsal_lanes_tsv or None,
        "gate_summary_ref": track_summary_json or None,
        "gate_summary_md_ref": track_summary_md or None,
        "gate_result": track_gate_result,
        "promotion_recommendation": track_promotion_recommendation,
        "promotion_scope": "public_testnet_rehearsal_only",
        "launch_promotion_allowed": False,
    },
    "public_launch_allowed": False,
    "residual_risk": [
        "synthetic negative packets prove fail-closed gate behavior only; they are not live incident or rollback evidence",
        "release-train drill does not claim mainnet-grade readiness, public validator admission, full light-client security, or live arbitrary state proof coverage",
        "operator must replace smoke artifacts with immutable runtime/world/governance refs before any live candidate window",
    ],
}

pathlib.Path(summary_json).write_text(
    json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
    encoding="utf-8",
)

lines = [
    "# Public Testnet Release-Train Drill Summary",
    "",
    f"- Window ID: `{window_id}`",
    f"- Manifest: `{manifest_path}`",
    f"- Lanes TSV: `{lanes_tsv}`",
    f"- Positive readiness: `{summary['positive_readiness']['gate_result']}` / `{summary['positive_readiness']['readiness_verdict']}`",
    f"- Release-train aggregate mode: `{aggregate_mode}`",
    f"- Release-train gate: `{track_gate_result}`",
    f"- Release-train promotion scope: `{summary['release_train_aggregate']['promotion_scope']}`",
    f"- Launch promotion allowed: `{summary['release_train_aggregate']['launch_promotion_allowed']}`",
    f"- Public launch allowed: `{summary['public_launch_allowed']}`",
    "",
    "## Claim Boundary",
    "",
    "- Allowed: `controlled_public_testnet_live_candidate_only`, `non_mainnet`, `resettable`, `guarded_faucet`",
    "- Denied: `live_public_testnet_already_online`, `mainnet_grade`, `production_oc_settlement`, `public_validator_admission`, `full_light_client`, `live_arbitrary_state_proof`, `multi_client_equivalence`",
    "",
    "## Negative Cases",
    "",
    "| Case | Gate | Observed | Summary |",
    "| --- | --- | --- | --- |",
]
for case in negative_cases:
    lines.append(
        f"| `{case['case_id']}` | `{case['gate_result']}` | `{case['observed_result']}` | `{case['summary_ref']}` |"
    )

lines.extend(
    [
        "",
        "## Residual Risk",
        "",
    ]
)
for item in summary["residual_risk"]:
    lines.append(f"- {item}")

pathlib.Path(summary_md).write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

echo "public testnet release-train drill summary: $summary_md"
echo "public testnet release-train drill summary json: $summary_json"
