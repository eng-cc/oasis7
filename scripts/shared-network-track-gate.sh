#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage:
  ./scripts/shared-network-track-gate.sh \
    --track <shared_devnet|staging|canary> \
    --candidate-bundle <bundle.json> \
    --lanes-tsv <lanes.tsv> \
    [--out-dir <path>]

Purpose:
  Build one machine-readable QA gate summary for shared network / release train
  tracks, with explicit pass/partial/block semantics and required lane checks.

TSV format:
  lane_id<TAB>owner<TAB>status<TAB>evidence_path<TAB>note

Status:
  pass | partial | block

Examples:
  ./scripts/shared-network-track-gate.sh \
    --track shared_devnet \
    --candidate-bundle output/release-candidates/shared-devnet-01.json \
    --lanes-tsv doc/testing/templates/shared-network-track-gate-lanes.shared_devnet.template.tsv \
    --out-dir output/shared-network-gates
USAGE
}

track=""
candidate_bundle=""
lanes_tsv=""
out_dir="output/shared-network-gates"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --track)
      track=${2:-}
      shift 2
      ;;
    --candidate-bundle)
      candidate_bundle=${2:-}
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

require_non_empty "--track" "$track"
require_non_empty "--candidate-bundle" "$candidate_bundle"
require_non_empty "--lanes-tsv" "$lanes_tsv"
require_file "--candidate-bundle" "$candidate_bundle"
require_file "--lanes-tsv" "$lanes_tsv"

case "$track" in
  shared_devnet|staging|canary)
    ;;
  *)
    echo "error: unsupported --track: $track" >&2
    exit 2
    ;;
esac

mkdir -p "$out_dir"
timestamp=$(date '+%Y%m%d-%H%M%S')
run_dir="$out_dir/$track-$timestamp"
mkdir -p "$run_dir"

candidate_validation_json="$run_dir/candidate_validation.json"
./scripts/release-candidate-bundle.sh validate --bundle "$candidate_bundle" >"$candidate_validation_json"

summary_json="$run_dir/summary.json"
summary_md="$run_dir/summary.md"

python3 - "$track" "$candidate_bundle" "$candidate_validation_json" "$lanes_tsv" "$summary_json" "$summary_md" "$run_dir" <<'PY'
import csv
import json
import pathlib
import sys
from datetime import datetime, timezone

track = sys.argv[1]
candidate_bundle_path = pathlib.Path(sys.argv[2]).resolve()
candidate_validation_path = pathlib.Path(sys.argv[3]).resolve()
lanes_tsv_path = pathlib.Path(sys.argv[4]).resolve()
summary_json_path = pathlib.Path(sys.argv[5]).resolve()
summary_md_path = pathlib.Path(sys.argv[6]).resolve()
run_dir = pathlib.Path(sys.argv[7]).resolve()

required_lanes = {
    "shared_devnet": [
        "candidate_bundle_integrity",
        "shared_access",
        "multi_entry_closure",
        "mixed_topology_baseline",
        "governance_live_drill",
        "short_window_longrun",
        "rollback_target_ready",
    ],
    "staging": [
        "candidate_bundle_integrity",
        "shared_access",
        "unified_candidate_gate",
        "mixed_topology_rehearsal",
        "governance_live_drill",
        "upgrade_rehearsal",
        "rollback_rehearsal",
        "incident_template",
    ],
    "canary": [
        "candidate_bundle_integrity",
        "promotion_record",
        "canary_window",
        "mixed_topology_claim_review",
        "rollback_rehearsal",
        "incident_review",
        "exit_decision",
    ],
}

status_rank = {"pass": 0, "partial": 1, "block": 2}

with candidate_bundle_path.open("r", encoding="utf-8") as fh:
    candidate_bundle = json.load(fh)
with candidate_validation_path.open("r", encoding="utf-8") as fh:
    candidate_validation = json.load(fh)

lanes = []
seen_lane_ids = set()

with lanes_tsv_path.open("r", encoding="utf-8", newline="") as fh:
    reader = csv.reader(fh, delimiter="\t")
    for row_no, row in enumerate(reader, start=1):
        if not row:
            continue
        if row[0].strip().startswith("#"):
            continue
        if len(row) != 5:
            raise SystemExit(
                f"invalid lanes tsv row {row_no}: expected 5 columns, got {len(row)}"
            )
        lane_id, owner, status, evidence_path, note = [item.strip() for item in row]
        if lane_id in seen_lane_ids:
            raise SystemExit(f"duplicate lane_id in lanes tsv: {lane_id}")
        seen_lane_ids.add(lane_id)
        if status not in status_rank:
            raise SystemExit(f"unsupported lane status `{status}` for {lane_id}")
        evidence = pathlib.Path(evidence_path).expanduser().resolve()
        if not evidence.exists():
            raise SystemExit(f"lane `{lane_id}` evidence path missing: {evidence}")
        lanes.append(
            {
                "lane_id": lane_id,
                "owner": owner,
                "status": status,
                "evidence_path": evidence_path,
                "resolved_evidence_path": str(evidence),
                "note": note,
            }
        )

missing_required_lanes = [
    lane_id for lane_id in required_lanes[track] if lane_id not in seen_lane_ids
]

if missing_required_lanes:
    gate_result = "block"
else:
    worst_rank = max((status_rank[item["status"]] for item in lanes), default=2)
    if worst_rank == 2:
        gate_result = "block"
    elif worst_rank == 1:
        gate_result = "partial"
    else:
        gate_result = "pass"

promotion_recommendation = (
    "eligible_for_promotion" if gate_result == "pass" else "hold_promotion"
)

summary = {
    "schema_version": "oasis7.shared_network_track_gate.v1",
    "track": track,
    "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "candidate_bundle_path": str(candidate_bundle_path),
    "candidate_id": candidate_bundle.get("candidate_id"),
    "candidate_track": candidate_bundle.get("track"),
    "candidate_git_commit": candidate_bundle.get("git_commit"),
    "candidate_validation": candidate_validation,
    "lanes_tsv_path": str(lanes_tsv_path),
    "required_lanes": required_lanes[track],
    "missing_required_lanes": missing_required_lanes,
    "lane_count": len(lanes),
    "lanes": lanes,
    "gate_result": gate_result,
    "promotion_recommendation": promotion_recommendation,
    "run_dir": str(run_dir),
}

summary_json_path.write_text(
    json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
    encoding="utf-8",
)

lines = [
    "# Shared Network Track Gate Summary",
    "",
    f"- Track: `{track}`",
    f"- Candidate ID: `{summary['candidate_id']}`",
    f"- Candidate bundle: `{candidate_bundle_path}`",
    f"- Gate result: `{gate_result}`",
    f"- Promotion recommendation: `{promotion_recommendation}`",
    "",
    "## Required Lanes",
]

for lane_id in required_lanes[track]:
    marker = "missing" if lane_id in missing_required_lanes else "present"
    lines.append(f"- `{lane_id}`: `{marker}`")

lines.extend(["", "## Lane Status Table", "", "| Lane | Owner | Status | Evidence | Note |", "| --- | --- | --- | --- | --- |"])

for lane in lanes:
    lines.append(
        f"| `{lane['lane_id']}` | `{lane['owner']}` | `{lane['status']}` | `{lane['evidence_path']}` | {lane['note']} |"
    )

if missing_required_lanes:
    lines.extend(["", "## Blocking Notes", ""])
    for lane_id in missing_required_lanes:
        lines.append(f"- missing required lane: `{lane_id}`")

summary_md_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

echo "shared network track gate summary: $summary_md"
echo "shared network track gate summary json: $summary_json"
