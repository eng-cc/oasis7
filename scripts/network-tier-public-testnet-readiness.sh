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
import ipaddress
import json
import pathlib
import sys
from datetime import datetime, timezone
from urllib.parse import urlparse

repo_root = pathlib.Path.cwd().resolve()
manifest_path = pathlib.Path(sys.argv[1]).resolve()
lanes_tsv_arg = sys.argv[2].strip()
summary_json_path = pathlib.Path(sys.argv[3]).resolve()
summary_md_path = pathlib.Path(sys.argv[4]).resolve()
run_dir = pathlib.Path(sys.argv[5]).resolve()

active_required_lanes = [
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
required_lanes = list(active_required_lanes)
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
    hostname = urlparse(lowered).hostname
    if hostname:
        placeholder_hosts = {
            "example.com",
            "example.net",
            "example.org",
            "example.invalid",
        }
        if (
            hostname in placeholder_hosts
            or hostname.endswith(".example")
            or hostname.endswith(".example.com")
            or hostname.endswith(".example.net")
            or hostname.endswith(".example.org")
            or hostname.endswith(".example.invalid")
        ):
            return True
    return (
        lowered == ""
        or "example.invalid" in lowered
        or "public-testnet-example" in lowered
        or "public-testnet-smoke" in lowered
        or lowered.endswith("public-testnet-skeleton-example.md")
        or lowered.endswith("public-testnet-skeleton-evidence.example.md")
        or lowered.endswith("public-testnet-rehearsal-template.md")
        or lowered.endswith("public-testnet-exit-review-template.md")
    )


def is_non_public_endpoint(raw: str) -> bool:
    hostname = urlparse(raw.strip().lower()).hostname
    if not hostname:
        return False
    if hostname in {"localhost", "localhost.localdomain"}:
        return True
    if (
        hostname.endswith(".localhost")
        or hostname.endswith(".local")
        or hostname.endswith(".localdomain")
        or hostname.endswith(".internal")
        or hostname.endswith(".home.arpa")
    ):
        return True
    try:
        ip = ipaddress.ip_address(hostname)
    except ValueError:
        return False
    return not ip.is_global


def is_template_ref(raw: str, resolved=None) -> bool:
    lowered = raw.strip().lower()
    if (
        "/templates/" in lowered
        or lowered.startswith("doc/testing/templates/")
        or lowered.endswith("-template.md")
    ):
        return True
    if resolved is None:
        return False
    try:
        relative = resolved.resolve().relative_to(repo_root)
    except ValueError:
        return False
    return relative.parts[:2] == ("doc", "testing") and "templates" in relative.parts


def validate_api_viewer_projection_pass_evidence(raw: str, evidence: pathlib.Path) -> list[str]:
    blockers: list[str] = []
    try:
        data = json.loads(evidence.read_text(encoding="utf-8"))
    except Exception as exc:
        return [f"api_viewer_projection_ready evidence must be JSON with api_viewer_projection object: {raw} ({exc})"]

    projection = data.get("api_viewer_projection")
    if not isinstance(projection, dict):
        projection = data.get("s10_summary", {}).get("api_viewer_projection") if isinstance(data.get("s10_summary"), dict) else None
    if not isinstance(projection, dict):
        return [f"api_viewer_projection_ready evidence missing api_viewer_projection object: {raw}"]

    if projection.get("status") != "pass":
        blockers.append(f"api_viewer_projection_ready status must be pass: {raw}")
    if projection.get("same_window_required") is not True:
        blockers.append(f"api_viewer_projection_ready same_window_required must be true: {raw}")
    if not str(projection.get("chain_status_samples_ref") or "").strip():
        blockers.append(f"api_viewer_projection_ready chain_status_samples_ref missing: {raw}")
    if not str(projection.get("api_projection_ref") or "").strip():
        blockers.append(f"api_viewer_projection_ready api_projection_ref missing: {raw}")
    if not str(projection.get("viewer_projection_ref") or "").strip():
        blockers.append(f"api_viewer_projection_ready viewer_projection_ref missing: {raw}")
    if projection.get("world_state_projection_match") is not True:
        blockers.append(f"api_viewer_projection_ready world_state_projection_match must be true: {raw}")
    return blockers


def ref_matches(actual: str, expected_raw: str, expected_resolved: pathlib.Path) -> bool:
    if actual == expected_raw or actual == str(expected_resolved):
        return True
    try:
        return resolve_ref(actual) == expected_resolved
    except Exception:
        return False


def validate_same_world_hosted_entry_pass_evidence(
    raw: str,
    evidence: pathlib.Path,
    manifest_path: pathlib.Path,
    manifest_ref: str,
    manifest_data: dict,
) -> list[str]:
    blockers: list[str] = []
    try:
        data = json.loads(evidence.read_text(encoding="utf-8"))
    except Exception as exc:
        return [f"same_world_hosted_entry_ready evidence must be JSON: {raw} ({exc})"]

    if data.get("evidence_schema") != "oasis7.same_world_hosted_entry.v1":
        blockers.append(f"same_world_hosted_entry_ready evidence_schema mismatch: {raw}")
    if data.get("status") != "pass":
        blockers.append(f"same_world_hosted_entry_ready status must be pass: {raw}")
    if data.get("same_window_required") is not True:
        blockers.append(f"same_world_hosted_entry_ready same_window_required must be true: {raw}")

    network_tier = data.get("network_tier")
    if not isinstance(network_tier, dict):
        blockers.append(f"same_world_hosted_entry_ready network_tier object missing: {raw}")
    else:
        if network_tier.get("tier") != "public_testnet":
            blockers.append(f"same_world_hosted_entry_ready network_tier.tier must be public_testnet: {raw}")
        if network_tier.get("network_id") != manifest_data.get("network_id"):
            blockers.append(f"same_world_hosted_entry_ready network_tier.network_id must match manifest: {raw}")
        if network_tier.get("chain_id") != manifest_data.get("chain_id"):
            blockers.append(f"same_world_hosted_entry_ready network_tier.chain_id must match manifest: {raw}")
        for key in ("network_id", "chain_id", "world_id"):
            if not str(network_tier.get(key) or "").strip():
                blockers.append(f"same_world_hosted_entry_ready network_tier.{key} missing: {raw}")

    expected_refs = [
        (
            ("manifest_ref",),
            manifest_ref,
            manifest_path,
        ),
        (
            ("genesis_ref",),
            manifest_data["runtime_refs"]["genesis_ref"],
            resolve_ref(manifest_data["runtime_refs"]["genesis_ref"]),
        ),
        (
            ("bootstrap_peer_ref", "bootstrap_peers_ref"),
            manifest_data["runtime_refs"]["bootstrap_peer_ref"],
            resolve_ref(manifest_data["runtime_refs"]["bootstrap_peer_ref"]),
        ),
    ]
    for keys, expected_raw, expected_resolved in expected_refs:
        key_label = "/".join(keys)
        actual = ""
        for key in keys:
            actual = str(data.get(key) or "").strip()
            if actual:
                break
        if not actual:
            blockers.append(f"same_world_hosted_entry_ready {key_label} missing: {raw}")
        elif not ref_matches(actual, expected_raw, expected_resolved):
            blockers.append(f"same_world_hosted_entry_ready {key_label} must match manifest: {raw}")

    required_refs = [
        "chain_status_samples_ref",
        "hosted_entry_ref",
        "launcher_config_ref",
        "viewer_config_ref",
        "pure_api_config_ref",
    ]
    for key in required_refs:
        if not str(data.get(key) or "").strip():
            blockers.append(f"same_world_hosted_entry_ready {key} missing: {raw}")

    if data.get("node_joined_public_testnet") is not True:
        blockers.append(f"same_world_hosted_entry_ready node_joined_public_testnet must be true: {raw}")
    if data.get("height_progressing") is not True:
        blockers.append(f"same_world_hosted_entry_ready height_progressing must be true: {raw}")
    if data.get("hosted_entry_reads_same_world_state") is not True:
        blockers.append(f"same_world_hosted_entry_ready hosted_entry_reads_same_world_state must be true: {raw}")
    if data.get("manual_checkpoint_or_data_copy_used") is not False:
        blockers.append(f"same_world_hosted_entry_ready manual_checkpoint_or_data_copy_used must be false: {raw}")

    does_not_claim = data.get("does_not_claim")
    if not isinstance(does_not_claim, list):
        blockers.append(f"same_world_hosted_entry_ready does_not_claim must be an array: {raw}")
    else:
        required_denials = {
            "mainnet-grade",
            "production OC settlement",
            "public validator onboarding open",
        }
        missing_denials = sorted(required_denials.difference(set(does_not_claim)))
        if missing_denials:
            blockers.append(
                "same_world_hosted_entry_ready does_not_claim missing: "
                + ",".join(missing_denials)
                + f": {raw}"
            )

    return blockers


def validate_chain_proof_evidence_pass_evidence(raw: str, evidence: pathlib.Path) -> list[str]:
    blockers: list[str] = []
    try:
        data = json.loads(evidence.read_text(encoding="utf-8"))
    except Exception as exc:
        return [f"chain_proof_evidence_ready evidence must be JSON: {raw} ({exc})"]

    if data.get("evidence_schema") != "oasis7.chain_proof_evidence.v1":
        blockers.append(f"chain_proof_evidence_ready evidence_schema mismatch: {raw}")
    if data.get("proof_contract") != "WorldHeadProofV1":
        blockers.append(f"chain_proof_evidence_ready proof_contract must be WorldHeadProofV1: {raw}")
    if data.get("proof_closure_status") != "proof_complete":
        blockers.append(f"chain_proof_evidence_ready proof_closure_status must be proof_complete for pass lane: {raw}")

    network_tier = data.get("network_tier")
    if not isinstance(network_tier, dict):
        blockers.append(f"chain_proof_evidence_ready network_tier object missing: {raw}")
    else:
        if network_tier.get("tier") != "public_testnet":
            blockers.append(f"chain_proof_evidence_ready network_tier.tier must be public_testnet: {raw}")
        for key in ("status", "chain_id", "network_id"):
            if not str(network_tier.get(key) or "").strip():
                blockers.append(f"chain_proof_evidence_ready network_tier.{key} missing: {raw}")

    proof = data.get("world_head_proof_v1")
    if not isinstance(proof, dict):
        blockers.append(f"chain_proof_evidence_ready world_head_proof_v1 object missing: {raw}")
    else:
        if proof.get("schema_version") != 1:
            blockers.append(f"chain_proof_evidence_ready world_head_proof_v1.schema_version must be 1: {raw}")
        if proof.get("claim_boundary") != "head_execution_checkpoint_evidence_only_not_light_client_or_mainnet_readiness":
            blockers.append(f"chain_proof_evidence_ready claim_boundary mismatch: {raw}")
        try:
            proof_height = int(proof.get("height") or 0)
        except (TypeError, ValueError):
            proof_height = 0
        if proof_height <= 0:
            blockers.append(f"chain_proof_evidence_ready proof height must be positive: {raw}")
        for key in ("world_id", "proof_hash", "world_head_proof_ref"):
            if not str(proof.get(key) or "").strip():
                blockers.append(f"chain_proof_evidence_ready world_head_proof_v1.{key} missing: {raw}")

    linkage = data.get("readiness_linkage")
    if not isinstance(linkage, dict):
        blockers.append(f"chain_proof_evidence_ready readiness_linkage object missing: {raw}")
    else:
        if not str(linkage.get("readiness_status") or "").strip():
            blockers.append(f"chain_proof_evidence_ready readiness_linkage.readiness_status missing: {raw}")
        failed_gates = linkage.get("failed_gates")
        if not isinstance(failed_gates, list):
            blockers.append(f"chain_proof_evidence_ready readiness_linkage.failed_gates must be an array: {raw}")

    does_not_claim = data.get("does_not_claim")
    if not isinstance(does_not_claim, list):
        blockers.append(f"chain_proof_evidence_ready does_not_claim must be an array: {raw}")
    else:
        required_denials = {
            "module_full",
            "integration_required",
            "release_full",
            "public_testnet ready",
            "ready_for_live_candidate",
            "mainnet-grade",
        }
        missing_denials = sorted(required_denials.difference(set(does_not_claim)))
        if missing_denials:
            blockers.append(
                "chain_proof_evidence_ready does_not_claim missing: "
                + ",".join(missing_denials)
                + f": {raw}"
            )

    residual_risk = data.get("residual_risk")
    if not isinstance(residual_risk, list) or not residual_risk:
        blockers.append(f"chain_proof_evidence_ready residual_risk must be a non-empty array: {raw}")
    return blockers


def escape_markdown_cell(raw: str) -> str:
    return raw.replace("\\", "\\\\").replace("|", "\\|").replace("\n", "<br>")


lanes = []
missing_required_lanes = list(required_lanes)
manifest_blockers = []
manifest_required_gates = list(data["promotion_policy"]["required_gates"])
missing_manifest_required_gates = [
    lane_id for lane_id in active_required_lanes if lane_id not in manifest_required_gates
]
unsupported_manifest_required_gates = [
    lane_id for lane_id in manifest_required_gates if lane_id not in active_required_lanes
]
if missing_manifest_required_gates:
    manifest_blockers.append(
        "manifest_missing_active_required_gates:"
        + ",".join(missing_manifest_required_gates)
    )
if unsupported_manifest_required_gates:
    manifest_blockers.append(
        "manifest_declares_unsupported_required_gates:"
        + ",".join(unsupported_manifest_required_gates)
    )
lanes_tsv_path = None
blocking_lanes = []
partial_lanes = []

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
    elif is_non_public_endpoint(raw):
        manifest_blockers.append(f"{endpoint_name}_non_public:{raw}")

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
            if status == "pass" and (
                is_placeholder_ref(evidence_path) or is_template_ref(evidence_path, evidence)
            ):
                raise SystemExit(
                    f"lane `{lane_id}` pass evidence cannot use placeholder/template ref: {evidence_path}"
                )
            if lane_id == "api_viewer_projection_ready" and status == "pass":
                projection_blockers = validate_api_viewer_projection_pass_evidence(
                    evidence_path, evidence
                )
                if projection_blockers:
                    raise SystemExit("; ".join(projection_blockers))
            if lane_id == "same_world_hosted_entry_ready" and status == "pass":
                hosted_entry_blockers = validate_same_world_hosted_entry_pass_evidence(
                    evidence_path, evidence, manifest_path, sys.argv[1], data
                )
                if hosted_entry_blockers:
                    raise SystemExit("; ".join(hosted_entry_blockers))
            if lane_id == "chain_proof_evidence_ready" and status == "pass":
                chain_proof_blockers = validate_chain_proof_evidence_pass_evidence(
                    evidence_path, evidence
                )
                if chain_proof_blockers:
                    raise SystemExit("; ".join(chain_proof_blockers))
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
    required_lane_set = set(required_lanes)
    required_lane_items = [
        item for item in lanes if item["lane_id"] in required_lane_set
    ]
    ignored_lanes = [
        item for item in lanes if item["lane_id"] not in required_lane_set
    ]
    blocking_lanes = [item for item in required_lane_items if item["status"] == "block"]
    partial_lanes = [item for item in required_lane_items if item["status"] == "partial"]
    if data["status"] == "specified_skeleton_only":
        manifest_blockers.append(
            "manifest_status_specified_skeleton_only_requires_rehearsal_or_live"
        )
    if missing_required_lanes or manifest_blockers:
        gate_result = "block"
    else:
        worst_rank = max(
            (status_rank[item["status"]] for item in required_lane_items),
            default=2,
        )
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
    "manifest_required_gates": manifest_required_gates,
    "missing_required_lanes": missing_required_lanes,
    "lanes_tsv_path": str(lanes_tsv_path) if lanes_tsv_path else None,
    "lane_count": len(lanes),
    "lanes": lanes,
    "blocking_lanes": blocking_lanes,
    "partial_lanes": partial_lanes,
    "ignored_lanes": ignored_lanes if lanes else [],
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
