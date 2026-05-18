#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage:
  ./scripts/network-tier-public-testnet-readiness.sh --manifest <path> [--lanes-tsv <path>] [--out-dir <path>]

Purpose:
  Build one machine-readable readiness review for a formal `public_testnet`
  manifest, and distinguish:
  - `specified_skeleton_only`
  - `partial`
  - `block`
  - `ready_for_live_candidate`

TSV format:
  lane_id<TAB>owner<TAB>status<TAB>evidence_path<TAB>note

Status:
  pass | partial | block

Examples:
  ./scripts/network-tier-public-testnet-readiness.sh \
    --manifest doc/testing/templates/network-tier-public-testnet.example.json

  ./scripts/network-tier-public-testnet-readiness.sh \
    --manifest output/network-tiers/public-testnet-rehearsal.json \
    --lanes-tsv doc/testing/templates/public-testnet-readiness-lanes.example.tsv
USAGE
}

manifest_path=""
lanes_tsv=""
out_dir="output/public-testnet-readiness"

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

if [[ -z "$manifest_path" ]]; then
  echo "error: --manifest is required" >&2
  exit 2
fi

if [[ ! -f "$manifest_path" ]]; then
  echo "error: --manifest not found: $manifest_path" >&2
  exit 2
fi

if [[ -n "$lanes_tsv" && ! -f "$lanes_tsv" ]]; then
  echo "error: --lanes-tsv not found: $lanes_tsv" >&2
  exit 2
fi

./scripts/network-tier-manifest.sh validate --manifest "$manifest_path" >/dev/null

mkdir -p "$out_dir"
timestamp=$(date '+%Y%m%d-%H%M%S')
run_dir="$out_dir/public-testnet-$timestamp"
mkdir -p "$run_dir"

summary_json="$run_dir/summary.json"
summary_md="$run_dir/summary.md"

python3 - "$manifest_path" "$lanes_tsv" "$summary_json" "$summary_md" "$run_dir" <<'PY'
import csv
import json
import pathlib
import sys
from datetime import datetime, timezone

manifest_path = pathlib.Path(sys.argv[1]).resolve()
lanes_tsv_arg = sys.argv[2].strip()
summary_json_path = pathlib.Path(sys.argv[3]).resolve()
summary_md_path = pathlib.Path(sys.argv[4]).resolve()
run_dir = pathlib.Path(sys.argv[5]).resolve()

required_lanes = [
    "shared_devnet_pass",
    "public_rpc_ready",
    "explorer_public_ready",
    "faucet_guard_ready",
    "reset_policy_announced",
    "runtime_bootstrap",
    "claims_boundary_review",
]
status_rank = {"pass": 0, "partial": 1, "block": 2}

data = json.loads(manifest_path.read_text(encoding="utf-8"))
if data["tier"] != "public_testnet":
    raise SystemExit(
        f"network tier manifest {manifest_path} must use tier=public_testnet"
    )


def resolve_ref(raw: str) -> pathlib.Path:
    path = pathlib.Path(raw)
    if path.is_absolute():
        return path.resolve()
    manifest_relative = (manifest_path.parent / path).resolve()
    if manifest_relative.exists():
        return manifest_relative
    return (pathlib.Path.cwd() / path).resolve()


def is_placeholder_ref(raw: str) -> bool:
    lowered = raw.strip().lower()
    return (
        lowered == ""
        or "example.invalid" in lowered
        or "public-testnet-example" in lowered
        or "public-testnet-smoke" in lowered
        or lowered.endswith("public-testnet-skeleton-example.md")
        or lowered.endswith("public-testnet-rehearsal-template.md")
        or lowered.endswith("public-testnet-exit-review-template.md")
    )


def escape_markdown_cell(raw: str) -> str:
    return raw.replace("\\", "\\\\").replace("|", "\\|").replace("\n", "<br>")


lanes = []
missing_required_lanes = list(required_lanes)
manifest_blockers = []
lanes_tsv_path = None

bundle_ref = data["runtime_refs"]["release_candidate_bundle_ref"]
bundle_path = resolve_ref(bundle_ref)
if not bundle_path.is_file():
    manifest_blockers.append(
        f"release_candidate_bundle_ref_missing:{bundle_path}"
    )

endpoint_policy = data["endpoint_policy"]
for endpoint_name in ("rpc_ref", "explorer_ref", "faucet_ref"):
    raw = endpoint_policy.get(endpoint_name)
    if raw is None:
        manifest_blockers.append(f"{endpoint_name}_missing")
        continue
    if is_placeholder_ref(raw):
        manifest_blockers.append(f"{endpoint_name}_placeholder:{raw}")

if data["status"] not in {"specified_skeleton_only", "rehearsal", "live"}:
    manifest_blockers.append(f"unsupported_public_testnet_status:{data['status']}")

if lanes_tsv_arg:
    lanes_tsv_path = pathlib.Path(lanes_tsv_arg).resolve()
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
            if status not in status_rank:
                raise SystemExit(f"unsupported lane status `{status}` for {lane_id}")
            if not owner:
                raise SystemExit(f"lane `{lane_id}` owner cannot be empty")
            if not evidence_path:
                raise SystemExit(f"lane `{lane_id}` evidence path cannot be empty")
            evidence = resolve_ref(evidence_path)
            if not evidence.is_file():
                raise SystemExit(f"lane `{lane_id}` evidence path missing: {evidence}")
            if status == "pass" and is_placeholder_ref(evidence_path):
                raise SystemExit(
                    f"lane `{lane_id}` pass evidence cannot use placeholder/template ref: {evidence_path}"
                )
            seen_lane_ids.add(lane_id)
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
        lane_id for lane_id in required_lanes if lane_id not in seen_lane_ids
    ]

if data["status"] == "specified_skeleton_only" and not lanes:
    readiness_verdict = "specified_skeleton_only"
    live_candidate_allowed = False
    claim_recommendation = "hold_public_testnet_claims"
    gate_result = "specified_skeleton_only"
elif not lanes:
    readiness_verdict = "block"
    live_candidate_allowed = False
    claim_recommendation = "hold_public_testnet_claims"
    gate_result = "block"
    manifest_blockers.append("lanes_tsv_required_for_non_skeleton_review")
else:
    if data["status"] == "specified_skeleton_only":
        manifest_blockers.append(
            "manifest_status_specified_skeleton_only_requires_rehearsal_or_live"
        )
    if missing_required_lanes or manifest_blockers:
        gate_result = "block"
    else:
        worst_rank = max((status_rank[item["status"]] for item in lanes), default=2)
        if worst_rank == 2:
            gate_result = "block"
        elif worst_rank == 1:
            gate_result = "partial"
        else:
            gate_result = "pass"
    if gate_result == "pass":
        readiness_verdict = "ready_for_live_candidate"
        live_candidate_allowed = True
        claim_recommendation = "allow_controlled_public_testnet_claims"
    elif gate_result == "partial":
        readiness_verdict = "partial"
        live_candidate_allowed = False
        claim_recommendation = "hold_public_testnet_claims"
    else:
        readiness_verdict = "block"
        live_candidate_allowed = False
        claim_recommendation = "hold_public_testnet_claims"

summary = {
    "schema_version": "oasis7.public_testnet_readiness_review.v1",
    "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "manifest_path": str(manifest_path),
    "manifest_status": data["status"],
    "tier": data["tier"],
    "network_id": data["network_id"],
    "chain_id": data["chain_id"],
    "release_candidate_bundle_ref": bundle_ref,
    "release_candidate_bundle_resolved_path": str(bundle_path),
    "rpc_ref": endpoint_policy["rpc_ref"],
    "explorer_ref": endpoint_policy["explorer_ref"],
    "faucet_ref": endpoint_policy["faucet_ref"],
    "required_lanes": required_lanes,
    "missing_required_lanes": missing_required_lanes,
    "lanes_tsv_path": str(lanes_tsv_path) if lanes_tsv_path else None,
    "lane_count": len(lanes),
    "lanes": lanes,
    "manifest_blockers": manifest_blockers,
    "gate_result": gate_result,
    "readiness_verdict": readiness_verdict,
    "live_candidate_allowed": live_candidate_allowed,
    "claim_recommendation": claim_recommendation,
    "claims_policy": data["claims_policy"],
    "run_dir": str(run_dir),
}

summary_json_path.write_text(
    json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
    encoding="utf-8",
)

lines = [
    "# Public Testnet Readiness Review",
    "",
    f"- Manifest: `{manifest_path}`",
    f"- Manifest status: `{data['status']}`",
    f"- Network ID: `{data['network_id']}`",
    f"- Gate result: `{gate_result}`",
    f"- Readiness verdict: `{readiness_verdict}`",
    f"- Claim recommendation: `{claim_recommendation}`",
    "",
    "## Manifest Review",
    f"- release candidate bundle: `{bundle_ref}`",
    f"- resolved bundle path: `{bundle_path}`",
    f"- rpc ref: `{endpoint_policy['rpc_ref']}`",
    f"- explorer ref: `{endpoint_policy['explorer_ref']}`",
    f"- faucet ref: `{endpoint_policy['faucet_ref']}`",
]

if manifest_blockers:
    lines.extend(["", "## Manifest Blockers"])
    for blocker in manifest_blockers:
        lines.append(f"- `{blocker}`")

lines.extend(["", "## Required Lanes"])
for lane_id in required_lanes:
    marker = "missing" if lane_id in missing_required_lanes else "present"
    lines.append(f"- `{lane_id}`: `{marker}`")

if lanes:
    lines.extend(
        [
            "",
            "## Lane Status Table",
            "",
            "| Lane | Owner | Status | Evidence | Note |",
            "| --- | --- | --- | --- | --- |",
        ]
    )
    for lane in lanes:
        lines.append(
            "| `{lane}` | `{owner}` | `{status}` | `{evidence}` | {note} |".format(
                lane=escape_markdown_cell(lane["lane_id"]),
                owner=escape_markdown_cell(lane["owner"]),
                status=escape_markdown_cell(lane["status"]),
                evidence=escape_markdown_cell(lane["evidence_path"]),
                note=escape_markdown_cell(lane["note"]),
            )
        )

lines.extend(
    [
        "",
        "## Final Verdict",
        f"- `readiness_verdict={readiness_verdict}`",
        f"- `live_candidate_allowed={str(live_candidate_allowed).lower()}`",
    ]
)

summary_md_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

cat "$summary_json"
