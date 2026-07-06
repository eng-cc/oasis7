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
    resolved_refs: dict[str, pathlib.Path] = {}
    for key in required_refs:
        raw_ref = str(data.get(key) or "").strip()
        if not raw_ref:
            blockers.append(f"same_world_hosted_entry_ready {key} missing: {raw}")
            continue
        resolved = resolve_ref(raw_ref)
        if not resolved.is_file():
            blockers.append(f"same_world_hosted_entry_ready {key} file missing: {raw_ref}")
            continue
        resolved_refs[key] = resolved

    chain_status_ref = resolved_refs.get("chain_status_samples_ref")
    if chain_status_ref is not None:
        try:
            chain_status = json.loads(chain_status_ref.read_text(encoding="utf-8"))
        except Exception as exc:
            blockers.append(
                f"same_world_hosted_entry_ready chain_status_samples_ref must be JSON: {chain_status_ref} ({exc})"
            )
        else:
            if chain_status.get("ok") is not True:
                blockers.append(
                    f"same_world_hosted_entry_ready chain status ok must be true: {chain_status_ref}"
                )
            readiness = chain_status.get("readiness")
            if not isinstance(readiness, dict):
                blockers.append(
                    f"same_world_hosted_entry_ready chain readiness object missing: {chain_status_ref}"
                )
            else:
                if readiness.get("ready") is not True:
                    blockers.append(
                        f"same_world_hosted_entry_ready chain readiness.ready must be true: {chain_status_ref}"
                    )
                if readiness.get("failed_gates") != []:
                    blockers.append(
                        f"same_world_hosted_entry_ready chain readiness.failed_gates must be []: {chain_status_ref}"
                    )
            world_resource = chain_status.get("world_resource")
            if not isinstance(world_resource, dict):
                blockers.append(
                    f"same_world_hosted_entry_ready chain world_resource object missing: {chain_status_ref}"
                )
            else:
                if world_resource.get("readiness_status") != "ready":
                    blockers.append(
                        f"same_world_hosted_entry_ready world_resource.readiness_status must be ready: {chain_status_ref}"
                    )
                if world_resource.get("failed_gates") != []:
                    blockers.append(
                        f"same_world_hosted_entry_ready world_resource.failed_gates must be []: {chain_status_ref}"
                    )
                if isinstance(network_tier, dict):
                    if world_resource.get("world_id") != network_tier.get("world_id"):
                        blockers.append(
                            f"same_world_hosted_entry_ready world_resource.world_id must match network_tier.world_id: {chain_status_ref}"
                        )
                    if world_resource.get("chain_id") != network_tier.get("chain_id"):
                        blockers.append(
                            f"same_world_hosted_entry_ready world_resource.chain_id must match network_tier.chain_id: {chain_status_ref}"
                        )

    pure_api_snapshot_ref = ""
    raw_samples = data.get("raw_samples")
    if isinstance(raw_samples, dict):
        pure_api_snapshot_ref = str(raw_samples.get("pure_api_snapshot") or "").strip()
    pure_api_config_ref = resolved_refs.get("pure_api_config_ref")
    if pure_api_config_ref is not None and not pure_api_snapshot_ref:
        try:
            pure_api_config = json.loads(pure_api_config_ref.read_text(encoding="utf-8"))
        except Exception as exc:
            blockers.append(
                f"same_world_hosted_entry_ready pure_api_config_ref must be JSON: {pure_api_config_ref} ({exc})"
            )
        else:
            pure_api_snapshot_ref = str(pure_api_config.get("sample_path") or "").strip()
    if not pure_api_snapshot_ref:
        blockers.append(f"same_world_hosted_entry_ready raw pure API snapshot ref missing: {raw}")
    else:
        pure_api_snapshot_path = resolve_ref(pure_api_snapshot_ref)
        if not pure_api_snapshot_path.is_file():
            blockers.append(
                f"same_world_hosted_entry_ready pure API snapshot file missing: {pure_api_snapshot_ref}"
            )
        else:
            try:
                pure_api_snapshot = json.loads(
                    pure_api_snapshot_path.read_text(encoding="utf-8")
                )
            except Exception as exc:
                blockers.append(
                    f"same_world_hosted_entry_ready pure API snapshot must be JSON: {pure_api_snapshot_path} ({exc})"
                )
            else:
                manifest = pure_api_snapshot.get("chain_resource_manifest")
                runtime_manifest = (
                    pure_api_snapshot.get("runtime_snapshot", {}).get("chain_resource_manifest")
                    if isinstance(pure_api_snapshot.get("runtime_snapshot"), dict)
                    else None
                )
                delta = pure_api_snapshot.get("latest_chain_resource_delta")
                if not isinstance(manifest, dict):
                    blockers.append(
                        f"same_world_hosted_entry_ready pure API chain_resource_manifest missing: {pure_api_snapshot_path}"
                    )
                if not isinstance(runtime_manifest, dict):
                    blockers.append(
                        f"same_world_hosted_entry_ready pure API runtime_snapshot.chain_resource_manifest missing: {pure_api_snapshot_path}"
                    )
                if not isinstance(delta, dict):
                    blockers.append(
                        f"same_world_hosted_entry_ready pure API latest_chain_resource_delta missing: {pure_api_snapshot_path}"
                    )
                if isinstance(network_tier, dict):
                    for label, obj in (
                        ("pure API manifest", manifest),
                        ("runtime snapshot manifest", runtime_manifest),
                        ("pure API delta", delta),
                    ):
                        if isinstance(obj, dict):
                            if obj.get("world_id") != network_tier.get("world_id"):
                                blockers.append(
                                    f"same_world_hosted_entry_ready {label} world_id must match network_tier.world_id: {pure_api_snapshot_path}"
                                )
                            if obj.get("chain_id") != network_tier.get("chain_id"):
                                blockers.append(
                                    f"same_world_hosted_entry_ready {label} chain_id must match network_tier.chain_id: {pure_api_snapshot_path}"
                                )

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
    proof_hash = ""
    proof_ref = ""
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
        proof_hash = str(proof.get("proof_hash") or "").strip()
        proof_ref = str(proof.get("world_head_proof_ref") or "").strip()

    verifier = data.get("external_verifier")
    if not isinstance(verifier, dict):
        blockers.append(f"chain_proof_evidence_ready external_verifier object missing: {raw}")
    else:
        if verifier.get("schema_version") != "oasis7.world_head_proof_verifier.v1":
            blockers.append(f"chain_proof_evidence_ready external_verifier.schema_version mismatch: {raw}")
        if verifier.get("status") != "pass":
            blockers.append(f"chain_proof_evidence_ready external_verifier.status must be pass: {raw}")
        if verifier.get("proof_contract") != "WorldHeadProofV1":
            blockers.append(f"chain_proof_evidence_ready external_verifier.proof_contract must be WorldHeadProofV1: {raw}")
        if verifier.get("claim_boundary") != "head_execution_checkpoint_evidence_only_not_light_client_or_mainnet_readiness":
            blockers.append(f"chain_proof_evidence_ready external_verifier.claim_boundary mismatch: {raw}")
        if proof_hash and str(verifier.get("proof_hash") or "").strip() != proof_hash:
            blockers.append(f"chain_proof_evidence_ready external_verifier.proof_hash must match proof hash: {raw}")
        if proof_ref and str(verifier.get("proof_ref") or "").strip() != proof_ref:
            blockers.append(f"chain_proof_evidence_ready external_verifier.proof_ref must match proof ref: {raw}")
        for key in ("verifier_command", "verified_at_unix_ms"):
            if not str(verifier.get(key) or "").strip():
                blockers.append(f"chain_proof_evidence_ready external_verifier.{key} missing: {raw}")
        verifier_denials = verifier.get("does_not_claim")
        if not isinstance(verifier_denials, list):
            blockers.append(f"chain_proof_evidence_ready external_verifier.does_not_claim must be an array: {raw}")
        else:
            required_verifier_denials = {
                "mainnet-grade finality",
                "state proof",
                "receipt proof",
                "DA sampling",
                "full light client",
            }
            missing_verifier_denials = sorted(required_verifier_denials.difference(set(verifier_denials)))
            if missing_verifier_denials:
                blockers.append(
                    "chain_proof_evidence_ready external_verifier.does_not_claim missing: "
                    + ",".join(missing_verifier_denials)
                    + f": {raw}"
                )

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


def validate_state_resource_receipt_proof_pass_evidence(
    raw: str,
    evidence: pathlib.Path,
    manifest_path: pathlib.Path,
    manifest_ref: str,
    manifest_data: dict,
) -> list[str]:
    lane = "state_resource_receipt_proof_ready"
    blockers: list[str] = []
    try:
        data = json.loads(evidence.read_text(encoding="utf-8"))
    except Exception as exc:
        return [f"{lane} evidence must be JSON: {raw} ({exc})"]

    if data.get("evidence_schema") != "oasis7.state_resource_receipt_proof_evidence.v1":
        blockers.append(f"{lane} evidence_schema mismatch: {raw}")
    if data.get("status") != "pass":
        blockers.append(f"{lane} status must be pass: {raw}")
    if data.get("proof_contract") != "WorldStateReceiptProofV1":
        blockers.append(f"{lane} proof_contract must be WorldStateReceiptProofV1: {raw}")
    if data.get("claim_boundary") != "state_resource_receipt_inclusion_evidence_only_not_full_light_client_or_mainnet_readiness":
        blockers.append(f"{lane} claim_boundary mismatch: {raw}")
    if data.get("independent_process") is not True:
        blockers.append(f"{lane} independent_process must be true: {raw}")
    for key in ("implementation_ref", "command_ref", "state_receipt_proof_ref", "state_receipt_proof_hash"):
        if not str(data.get(key) or "").strip():
            blockers.append(f"{lane} {key} missing: {raw}")

    network_tier = data.get("network_tier")
    if not isinstance(network_tier, dict):
        blockers.append(f"{lane} network_tier object missing: {raw}")
    else:
        if network_tier.get("tier") != "public_testnet":
            blockers.append(f"{lane} network_tier.tier must be public_testnet: {raw}")
        if network_tier.get("network_id") != manifest_data.get("network_id"):
            blockers.append(f"{lane} network_tier.network_id must match manifest: {raw}")
        if network_tier.get("chain_id") != manifest_data.get("chain_id"):
            blockers.append(f"{lane} network_tier.chain_id must match manifest: {raw}")
        for key in ("network_id", "chain_id", "world_id"):
            if not str(network_tier.get(key) or "").strip():
                blockers.append(f"{lane} network_tier.{key} missing: {raw}")

    expected_refs = [
        ("manifest_ref", manifest_ref, manifest_path),
        ("genesis_ref", manifest_data["runtime_refs"]["genesis_ref"], resolve_ref(manifest_data["runtime_refs"]["genesis_ref"])),
        ("bootstrap_peer_ref", manifest_data["runtime_refs"]["bootstrap_peer_ref"], resolve_ref(manifest_data["runtime_refs"]["bootstrap_peer_ref"])),
    ]
    for key, expected_raw, expected_resolved in expected_refs:
        actual = str(data.get(key) or "").strip()
        if not actual:
            blockers.append(f"{lane} {key} missing: {raw}")
        elif not ref_matches(actual, expected_raw, expected_resolved):
            blockers.append(f"{lane} {key} must match manifest: {raw}")

    expected_rpc_ref = str(manifest_data.get("endpoint_policy", {}).get("rpc_ref") or "").strip()
    rpc_ref = str(data.get("rpc_ref") or "").strip()
    status_endpoint_ref = str(data.get("status_endpoint_ref") or "").strip()
    if not rpc_ref and not status_endpoint_ref:
        blockers.append(f"{lane} rpc_ref or status_endpoint_ref missing: {raw}")
    if rpc_ref and rpc_ref != expected_rpc_ref:
        blockers.append(f"{lane} rpc_ref must match manifest endpoint_policy.rpc_ref: {raw}")

    observed_head = data.get("observed_head")
    observed_height = 0
    observed_state_root = ""
    observed_receipts_root = ""
    if not isinstance(observed_head, dict):
        blockers.append(f"{lane} observed_head object missing: {raw}")
    else:
        try:
            observed_height = int(observed_head.get("height") or 0)
        except (TypeError, ValueError):
            observed_height = 0
        if observed_height <= 0:
            blockers.append(f"{lane} observed_head.height must be positive: {raw}")
        for key in ("hash", "state_root", "receipts_root"):
            if not str(observed_head.get(key) or "").strip():
                blockers.append(f"{lane} observed_head.{key} missing: {raw}")
        observed_state_root = str(observed_head.get("state_root") or "").strip()
        observed_receipts_root = str(observed_head.get("receipts_root") or "").strip()

    verifier = data.get("external_verifier")
    proof_kind = ""
    proof_status = ""
    root_hash = ""
    verifier_subject = None
    if not isinstance(verifier, dict):
        blockers.append(f"{lane} external_verifier object missing: {raw}")
    else:
        if verifier.get("schema_version") != "oasis7.world_state_receipt_proof_verifier.v1":
            blockers.append(f"{lane} external_verifier.schema_version mismatch: {raw}")
        if verifier.get("status") != "pass":
            blockers.append(f"{lane} external_verifier.status must be pass: {raw}")
        if verifier.get("proof_contract") != "WorldStateReceiptProofV1":
            blockers.append(f"{lane} external_verifier.proof_contract must be WorldStateReceiptProofV1: {raw}")
        if verifier.get("hash_domain") != "oasis7.world_state_receipt_proof.v1":
            blockers.append(f"{lane} external_verifier.hash_domain mismatch: {raw}")
        if verifier.get("claim_boundary") != "state_resource_receipt_inclusion_evidence_only_not_full_light_client_or_mainnet_readiness":
            blockers.append(f"{lane} external_verifier.claim_boundary mismatch: {raw}")
        if str(verifier.get("proof_ref") or "").strip() != str(data.get("state_receipt_proof_ref") or "").strip():
            blockers.append(f"{lane} external_verifier.proof_ref must match evidence state_receipt_proof_ref: {raw}")
        if str(verifier.get("proof_hash") or "").strip() != str(data.get("state_receipt_proof_hash") or "").strip():
            blockers.append(f"{lane} external_verifier.proof_hash must match evidence state_receipt_proof_hash: {raw}")
        if str(verifier.get("world_id") or "").strip() != str(network_tier.get("world_id") if isinstance(network_tier, dict) else "").strip():
            blockers.append(f"{lane} external_verifier.world_id must match network_tier.world_id: {raw}")
        try:
            verifier_height = int(verifier.get("height") or 0)
        except (TypeError, ValueError):
            verifier_height = 0
        if observed_height and verifier_height != observed_height:
            blockers.append(f"{lane} external_verifier.height must match observed_head.height: {raw}")
        head = verifier.get("head")
        if not isinstance(head, dict):
            blockers.append(f"{lane} external_verifier.head object missing: {raw}")
        else:
            if str(head.get("block_hash") or "").strip() != str(observed_head.get("hash") if isinstance(observed_head, dict) else "").strip():
                blockers.append(f"{lane} external_verifier.head.block_hash must match observed_head.hash: {raw}")
            if observed_state_root and str(head.get("state_root") or "").strip() != observed_state_root:
                blockers.append(f"{lane} external_verifier.head.state_root must match observed_head.state_root: {raw}")
            if observed_receipts_root and str(head.get("receipts_root") or "").strip() != observed_receipts_root:
                blockers.append(f"{lane} external_verifier.head.receipts_root must match observed_head.receipts_root: {raw}")
        proof_kind = str(verifier.get("proof_kind") or "").strip()
        root_hash = str(verifier.get("root_hash") or "").strip()
        if proof_kind not in {"resource_state", "query_result", "receipt"}:
            blockers.append(f"{lane} external_verifier.proof_kind unsupported: {raw}")
        proof_status = str(verifier.get("proof_status") or "").strip()
        if proof_kind in {"resource_state", "query_result"}:
            if proof_status not in {"included", "absent"}:
                blockers.append(f"{lane} external_verifier.proof_status must be included or absent for state/query proofs: {raw}")
        elif proof_kind == "receipt" and proof_status != "included":
            blockers.append(f"{lane} external_verifier.proof_status must be included for receipt proofs: {raw}")
        verifier_subject = verifier.get("subject")
        if not isinstance(verifier_subject, dict):
            blockers.append(f"{lane} external_verifier.subject object missing: {raw}")
        if proof_kind in {"resource_state", "query_result"} and observed_state_root and root_hash != observed_state_root:
            blockers.append(f"{lane} external_verifier.root_hash must match observed_head.state_root for state/query proofs: {raw}")
        if proof_kind == "receipt" and observed_receipts_root and root_hash != observed_receipts_root:
            blockers.append(f"{lane} external_verifier.root_hash must match observed_head.receipts_root for receipt proofs: {raw}")
        for key in ("head_proof_hash", "root_hash", "leaf_hash"):
            if not str(verifier.get(key) or "").strip():
                blockers.append(f"{lane} external_verifier.{key} missing: {raw}")
        if int(verifier.get("proof_path_nodes") or 0) <= 0:
            blockers.append(f"{lane} external_verifier.proof_path_nodes must be positive: {raw}")
        verifier_denials = verifier.get("does_not_claim")
        if not isinstance(verifier_denials, list):
            blockers.append(f"{lane} external_verifier.does_not_claim must be an array: {raw}")
        else:
            required_verifier_denials = {
                "mainnet-grade finality",
                "full light client",
                "validator-set finality",
                "DA sampling",
                "multi-client consensus equivalence",
                "live runtime arbitrary state proof availability",
            }
            missing_verifier_denials = sorted(required_verifier_denials.difference(set(verifier_denials)))
            if missing_verifier_denials:
                blockers.append(
                    f"{lane} external_verifier.does_not_claim missing: "
                    + ",".join(missing_verifier_denials)
                    + f": {raw}"
                )

    proof_targets = data.get("proof_targets")
    if not isinstance(proof_targets, dict):
        blockers.append(f"{lane} proof_targets object missing: {raw}")
    else:
        state_or_query = proof_targets.get("state_or_query")
        if proof_kind in {"resource_state", "query_result"} and not isinstance(state_or_query, dict):
            blockers.append(f"{lane} proof_targets.state_or_query object missing: {raw}")
        elif isinstance(state_or_query, dict):
            for key in ("proof_kind", "namespace", "root_hash", "leaf_hash", "proof_status"):
                if not str(state_or_query.get(key) or "").strip():
                    blockers.append(f"{lane} proof_targets.state_or_query.{key} missing: {raw}")
            target_kind = str(state_or_query.get("proof_kind") or "").strip()
            if target_kind not in {"resource_state", "query_result"}:
                blockers.append(f"{lane} proof_targets.state_or_query.proof_kind unsupported: {raw}")
            target_status = str(state_or_query.get("proof_status") or "").strip()
            if target_status not in {"included", "absent"}:
                blockers.append(f"{lane} proof_targets.state_or_query.proof_status unsupported: {raw}")
            if target_kind == "resource_state" and not str(state_or_query.get("resource_id") or "").strip():
                blockers.append(f"{lane} proof_targets.state_or_query.resource_id missing: {raw}")
            if target_kind == "query_result" and not str(state_or_query.get("query_id") or "").strip():
                blockers.append(f"{lane} proof_targets.state_or_query.query_id missing: {raw}")
            if observed_state_root and str(state_or_query.get("root_hash") or "").strip() != observed_state_root:
                blockers.append(f"{lane} proof_targets.state_or_query.root_hash must match observed_head.state_root: {raw}")
            if proof_kind in {"resource_state", "query_result"}:
                if str(state_or_query.get("proof_kind") or "").strip() != proof_kind:
                    blockers.append(f"{lane} proof_targets.state_or_query.proof_kind must match external_verifier.proof_kind: {raw}")
                if root_hash and str(state_or_query.get("root_hash") or "").strip() != root_hash:
                    blockers.append(f"{lane} proof_targets.state_or_query.root_hash must match external_verifier.root_hash: {raw}")
                if str(verifier.get("leaf_hash") or "").strip() and str(state_or_query.get("leaf_hash") or "").strip() != str(verifier.get("leaf_hash") or "").strip():
                    blockers.append(f"{lane} proof_targets.state_or_query.leaf_hash must match external_verifier.leaf_hash: {raw}")
                if proof_status and target_status and target_status != proof_status:
                    blockers.append(f"{lane} proof_targets.state_or_query.proof_status must match external_verifier.proof_status: {raw}")
                if isinstance(verifier_subject, dict):
                    if str(state_or_query.get("namespace") or "").strip() != str(verifier_subject.get("namespace") or "").strip():
                        blockers.append(f"{lane} proof_targets.state_or_query.namespace must match external_verifier.subject.namespace: {raw}")
                    if proof_kind == "resource_state" and str(state_or_query.get("resource_id") or "").strip() != str(verifier_subject.get("resource_id") or "").strip():
                        blockers.append(f"{lane} proof_targets.state_or_query.resource_id must match external_verifier.subject.resource_id: {raw}")
                    if proof_kind == "query_result":
                        for key in ("query_id", "query_hash"):
                            if str(state_or_query.get(key) or "").strip() != str(verifier_subject.get(key) or "").strip():
                                blockers.append(f"{lane} proof_targets.state_or_query.{key} must match external_verifier.subject.{key}: {raw}")
        resource = proof_targets.get("resource")
        resource_content_required = proof_kind == "resource_state" and proof_status != "absent"
        if resource_content_required and not isinstance(resource, dict):
            blockers.append(f"{lane} proof_targets.resource object missing: {raw}")
        elif isinstance(resource, dict):
            for key in ("resource_manifest_ref", "resource_delta_ref", "content_hash", "commit_hash"):
                if not str(resource.get(key) or "").strip():
                    blockers.append(f"{lane} proof_targets.resource.{key} missing: {raw}")
            try:
                commit_height = int(resource.get("commit_height") or 0)
            except (TypeError, ValueError):
                commit_height = 0
            if commit_height <= 0:
                blockers.append(f"{lane} proof_targets.resource.commit_height must be positive: {raw}")
            if observed_height and commit_height != observed_height:
                blockers.append(f"{lane} proof_targets.resource.commit_height must match observed_head.height: {raw}")
            if str(resource.get("commit_hash") or "").strip() != str(observed_head.get("hash") if isinstance(observed_head, dict) else "").strip():
                blockers.append(f"{lane} proof_targets.resource.commit_hash must match observed_head.hash: {raw}")
        receipt = proof_targets.get("receipt")
        if proof_kind == "receipt" and not isinstance(receipt, dict):
            blockers.append(f"{lane} proof_targets.receipt object missing: {raw}")
        elif isinstance(receipt, dict):
            for key in ("action_id", "receipt_hash", "execution_status", "result_hash", "root_hash", "leaf_hash"):
                if not str(receipt.get(key) or "").strip():
                    blockers.append(f"{lane} proof_targets.receipt.{key} missing: {raw}")
            if observed_receipts_root and str(receipt.get("root_hash") or "").strip() != observed_receipts_root:
                blockers.append(f"{lane} proof_targets.receipt.root_hash must match observed_head.receipts_root: {raw}")
            if proof_kind == "receipt":
                if root_hash and str(receipt.get("root_hash") or "").strip() != root_hash:
                    blockers.append(f"{lane} proof_targets.receipt.root_hash must match external_verifier.root_hash: {raw}")
                if str(verifier.get("leaf_hash") or "").strip() and str(receipt.get("leaf_hash") or "").strip() != str(verifier.get("leaf_hash") or "").strip():
                    blockers.append(f"{lane} proof_targets.receipt.leaf_hash must match external_verifier.leaf_hash: {raw}")
                if isinstance(verifier_subject, dict):
                    subject_map = {
                        "action_id": "action_id",
                        "receipt_hash": "receipt_hash",
                        "execution_status": "status",
                        "result_hash": "result_hash",
                    }
                    for target_key, subject_key in subject_map.items():
                        if str(receipt.get(target_key) or "").strip() != str(verifier_subject.get(subject_key) or "").strip():
                            blockers.append(f"{lane} proof_targets.receipt.{target_key} must match external_verifier.subject.{subject_key}: {raw}")

    for key in (
        "node_db_access_used",
        "manual_checkpoint_or_data_copy_used",
        "privileged_internal_api_used",
    ):
        if data.get(key) is not False:
            blockers.append(f"{lane} {key} must be false: {raw}")

    does_not_claim = data.get("does_not_claim")
    if not isinstance(does_not_claim, list):
        blockers.append(f"{lane} does_not_claim must be an array: {raw}")
    else:
        required_denials = {
            "ready_for_live_candidate",
            "mainnet-grade",
            "full light client security",
            "validator-set finality",
            "multi-client consensus equivalence",
            "production OC settlement",
            "live runtime arbitrary state proof availability",
        }
        missing_denials = sorted(required_denials.difference(set(does_not_claim)))
        if missing_denials:
            blockers.append(
                f"{lane} does_not_claim missing: "
                + ",".join(missing_denials)
                + f": {raw}"
            )

    residual_risk = data.get("residual_risk")
    if not isinstance(residual_risk, list) or not residual_risk:
        blockers.append(f"{lane} residual_risk must be a non-empty array: {raw}")
    return blockers


def validate_external_verifier_light_client_lite_pass_evidence(
    raw: str,
    evidence: pathlib.Path,
    manifest_path: pathlib.Path,
    manifest_ref: str,
    manifest_data: dict,
) -> list[str]:
    lane = "external_verifier_light_client_lite_ready"
    blockers: list[str] = []
    try:
        data = json.loads(evidence.read_text(encoding="utf-8"))
    except Exception as exc:
        return [f"{lane} evidence must be JSON: {raw} ({exc})"]

    if data.get("evidence_schema") != "oasis7.external_verifier_light_client_lite.v1":
        blockers.append(f"{lane} evidence_schema mismatch: {raw}")
    if data.get("status") != "pass":
        blockers.append(f"{lane} status must be pass: {raw}")
    if data.get("verifier_mode") != "external_light_client_lite":
        blockers.append(f"{lane} verifier_mode must be external_light_client_lite: {raw}")
    if data.get("independent_process") is not True:
        blockers.append(f"{lane} independent_process must be true: {raw}")
    for key in ("implementation_ref", "command_ref"):
        if not str(data.get(key) or "").strip():
            blockers.append(f"{lane} {key} missing: {raw}")

    network_tier = data.get("network_tier")
    if not isinstance(network_tier, dict):
        blockers.append(f"{lane} network_tier object missing: {raw}")
    else:
        if network_tier.get("tier") != "public_testnet":
            blockers.append(f"{lane} network_tier.tier must be public_testnet: {raw}")
        if network_tier.get("network_id") != manifest_data.get("network_id"):
            blockers.append(f"{lane} network_tier.network_id must match manifest: {raw}")
        if network_tier.get("chain_id") != manifest_data.get("chain_id"):
            blockers.append(f"{lane} network_tier.chain_id must match manifest: {raw}")
        for key in ("network_id", "chain_id", "world_id"):
            if not str(network_tier.get(key) or "").strip():
                blockers.append(f"{lane} network_tier.{key} missing: {raw}")

    expected_refs = [
        ("manifest_ref", manifest_ref, manifest_path),
        ("genesis_ref", manifest_data["runtime_refs"]["genesis_ref"], resolve_ref(manifest_data["runtime_refs"]["genesis_ref"])),
        ("bootstrap_peer_ref", manifest_data["runtime_refs"]["bootstrap_peer_ref"], resolve_ref(manifest_data["runtime_refs"]["bootstrap_peer_ref"])),
    ]
    for key, expected_raw, expected_resolved in expected_refs:
        actual = str(data.get(key) or "").strip()
        if not actual:
            blockers.append(f"{lane} {key} missing: {raw}")
        elif not ref_matches(actual, expected_raw, expected_resolved):
            blockers.append(f"{lane} {key} must match manifest: {raw}")

    rpc_ref = str(data.get("rpc_ref") or "").strip()
    status_endpoint_ref = str(data.get("status_endpoint_ref") or "").strip()
    expected_rpc_ref = str(manifest_data.get("endpoint_policy", {}).get("rpc_ref") or "").strip()
    if not rpc_ref and not status_endpoint_ref:
        blockers.append(f"{lane} rpc_ref or status_endpoint_ref missing: {raw}")
    if rpc_ref and rpc_ref != expected_rpc_ref:
        blockers.append(f"{lane} rpc_ref must match manifest endpoint_policy.rpc_ref: {raw}")

    sample_window = data.get("sample_window")
    if not isinstance(sample_window, dict):
        blockers.append(f"{lane} sample_window object missing: {raw}")
    else:
        for key in ("started_at", "ended_at"):
            if not str(sample_window.get(key) or "").strip():
                blockers.append(f"{lane} sample_window.{key} missing: {raw}")

    observed_head = data.get("observed_head")
    observed_height = 0
    if not isinstance(observed_head, dict):
        blockers.append(f"{lane} observed_head object missing: {raw}")
    else:
        try:
            observed_height = int(observed_head.get("height") or 0)
        except (TypeError, ValueError):
            observed_height = 0
        if observed_height <= 0:
            blockers.append(f"{lane} observed_head.height must be positive: {raw}")
        for key in ("hash", "state_root"):
            if not str(observed_head.get(key) or "").strip():
                blockers.append(f"{lane} observed_head.{key} missing: {raw}")

    verified_range = data.get("verified_range")
    if not isinstance(verified_range, dict):
        blockers.append(f"{lane} verified_range object missing: {raw}")
    else:
        try:
            from_height = int(verified_range.get("from_height") or 0)
            to_height = int(verified_range.get("to_height") or 0)
            observed_height = int(observed_head.get("height") or 0) if isinstance(observed_head, dict) else 0
        except (TypeError, ValueError):
            from_height = 0
            to_height = 0
            observed_height = 0
        if from_height <= 0:
            blockers.append(f"{lane} verified_range.from_height must be positive: {raw}")
        if to_height < observed_height:
            blockers.append(f"{lane} verified_range.to_height must be >= observed_head.height: {raw}")

    if data.get("verification_result") != "accepted":
        blockers.append(f"{lane} verification_result must be accepted: {raw}")
    proof_ref = str(data.get("proof_ref") or "").strip()
    proof_hash = str(data.get("proof_hash") or "").strip()
    if not proof_ref:
        blockers.append(f"{lane} proof_ref missing: {raw}")
    if not proof_hash:
        blockers.append(f"{lane} proof_hash missing: {raw}")
    verifier = data.get("external_verifier")
    if not isinstance(verifier, dict):
        blockers.append(f"{lane} external_verifier object missing: {raw}")
    else:
        if verifier.get("schema_version") != "oasis7.world_head_proof_verifier.v1":
            blockers.append(f"{lane} external_verifier.schema_version mismatch: {raw}")
        if verifier.get("status") != "pass":
            blockers.append(f"{lane} external_verifier.status must be pass: {raw}")
        if verifier.get("proof_contract") != "WorldHeadProofV1":
            blockers.append(f"{lane} external_verifier.proof_contract must be WorldHeadProofV1: {raw}")
        if verifier.get("claim_boundary") != "head_execution_checkpoint_evidence_only_not_light_client_or_mainnet_readiness":
            blockers.append(f"{lane} external_verifier.claim_boundary mismatch: {raw}")
        if str(verifier.get("proof_ref") or "").strip() != proof_ref:
            blockers.append(f"{lane} external_verifier.proof_ref must match evidence proof_ref: {raw}")
        if str(verifier.get("proof_hash") or "").strip() != proof_hash:
            blockers.append(f"{lane} external_verifier.proof_hash must match evidence proof_hash: {raw}")
        if str(verifier.get("world_id") or "").strip() != str(network_tier.get("world_id") if isinstance(network_tier, dict) else "").strip():
            blockers.append(f"{lane} external_verifier.world_id must match network_tier.world_id: {raw}")
        try:
            verifier_height = int(verifier.get("height") or 0)
        except (TypeError, ValueError):
            verifier_height = 0
        if observed_height and verifier_height != observed_height:
            blockers.append(f"{lane} external_verifier.height must match observed_head.height: {raw}")
        verifier_head = verifier.get("head")
        if not isinstance(verifier_head, dict):
            blockers.append(f"{lane} external_verifier.head object missing: {raw}")
        elif isinstance(observed_head, dict):
            if str(verifier_head.get("block_hash") or "").strip() != str(observed_head.get("hash") or "").strip():
                blockers.append(f"{lane} external_verifier.head.block_hash must match observed_head.hash: {raw}")
            if str(verifier_head.get("state_root") or "").strip() != str(observed_head.get("state_root") or "").strip():
                blockers.append(f"{lane} external_verifier.head.state_root must match observed_head.state_root: {raw}")
        verifier_denials = verifier.get("does_not_claim")
        if not isinstance(verifier_denials, list):
            blockers.append(f"{lane} external_verifier.does_not_claim must be an array: {raw}")
        else:
            required_verifier_denials = {
                "mainnet-grade finality",
                "state proof",
                "receipt proof",
                "DA sampling",
                "full light client",
            }
            missing_verifier_denials = sorted(required_verifier_denials.difference(set(verifier_denials)))
            if missing_verifier_denials:
                blockers.append(
                    f"{lane} external_verifier.does_not_claim missing: "
                    + ",".join(missing_verifier_denials)
                    + f": {raw}"
                )
    for key in (
        "node_db_access_used",
        "manual_checkpoint_or_data_copy_used",
        "privileged_internal_api_used",
    ):
        if data.get(key) is not False:
            blockers.append(f"{lane} {key} must be false: {raw}")

    does_not_claim = data.get("does_not_claim")
    if not isinstance(does_not_claim, list):
        blockers.append(f"{lane} does_not_claim must be an array: {raw}")
    else:
        required_denials = {
            "mainnet-grade",
            "production OC settlement",
            "public validator onboarding open",
            "multi-client consensus equivalence",
            "full light client security",
            "ready_for_live_candidate",
        }
        missing_denials = sorted(required_denials.difference(set(does_not_claim)))
        if missing_denials:
            blockers.append(
                f"{lane} does_not_claim missing: "
                + ",".join(missing_denials)
                + f": {raw}"
            )

    residual_risk = data.get("residual_risk")
    if not isinstance(residual_risk, list) or not residual_risk:
        blockers.append(f"{lane} residual_risk must be a non-empty array: {raw}")

    return blockers


def validate_light_client_continuity_window_pass_evidence(
    raw: str,
    evidence: pathlib.Path,
    manifest_path: pathlib.Path,
    manifest_ref: str,
    manifest_data: dict,
) -> list[str]:
    lane = "light_client_continuity_window_ready"
    blockers: list[str] = []
    try:
        data = json.loads(evidence.read_text(encoding="utf-8"))
    except Exception as exc:
        return [f"{lane} evidence must be JSON: {raw} ({exc})"]

    if data.get("evidence_schema") != "oasis7.light_client_continuity_window.v1":
        blockers.append(f"{lane} evidence_schema mismatch: {raw}")
    if data.get("status") != "pass":
        blockers.append(f"{lane} status must be pass: {raw}")
    if data.get("verifier_mode") != "proof_window_continuity":
        blockers.append(f"{lane} verifier_mode must be proof_window_continuity: {raw}")
    if data.get("independent_process") is not True:
        blockers.append(f"{lane} independent_process must be true: {raw}")
    for key in ("implementation_ref", "command_ref", "proof_window_ref"):
        if not str(data.get(key) or "").strip():
            blockers.append(f"{lane} {key} missing: {raw}")

    network_tier = data.get("network_tier")
    if not isinstance(network_tier, dict):
        blockers.append(f"{lane} network_tier object missing: {raw}")
    else:
        if network_tier.get("tier") != "public_testnet":
            blockers.append(f"{lane} network_tier.tier must be public_testnet: {raw}")
        if network_tier.get("network_id") != manifest_data.get("network_id"):
            blockers.append(f"{lane} network_tier.network_id must match manifest: {raw}")
        if network_tier.get("chain_id") != manifest_data.get("chain_id"):
            blockers.append(f"{lane} network_tier.chain_id must match manifest: {raw}")
        for key in ("network_id", "chain_id", "world_id"):
            if not str(network_tier.get(key) or "").strip():
                blockers.append(f"{lane} network_tier.{key} missing: {raw}")

    expected_refs = [
        ("manifest_ref", manifest_ref, manifest_path),
        ("genesis_ref", manifest_data["runtime_refs"]["genesis_ref"], resolve_ref(manifest_data["runtime_refs"]["genesis_ref"])),
        ("bootstrap_peer_ref", manifest_data["runtime_refs"]["bootstrap_peer_ref"], resolve_ref(manifest_data["runtime_refs"]["bootstrap_peer_ref"])),
    ]
    for key, expected_raw, expected_resolved in expected_refs:
        actual = str(data.get(key) or "").strip()
        if not actual:
            blockers.append(f"{lane} {key} missing: {raw}")
        elif not ref_matches(actual, expected_raw, expected_resolved):
            blockers.append(f"{lane} {key} must match manifest: {raw}")

    rpc_ref = str(data.get("rpc_ref") or "").strip()
    status_endpoint_ref = str(data.get("status_endpoint_ref") or "").strip()
    expected_rpc_ref = str(manifest_data.get("endpoint_policy", {}).get("rpc_ref") or "").strip()
    if not rpc_ref and not status_endpoint_ref:
        blockers.append(f"{lane} rpc_ref or status_endpoint_ref missing: {raw}")
    if rpc_ref and rpc_ref != expected_rpc_ref:
        blockers.append(f"{lane} rpc_ref must match manifest endpoint_policy.rpc_ref: {raw}")

    sample_window = data.get("sample_window")
    if not isinstance(sample_window, dict):
        blockers.append(f"{lane} sample_window object missing: {raw}")
    else:
        for key in ("started_at", "ended_at"):
            if not str(sample_window.get(key) or "").strip():
                blockers.append(f"{lane} sample_window.{key} missing: {raw}")

    trusted_anchor = data.get("trusted_anchor")
    if not isinstance(trusted_anchor, dict):
        blockers.append(f"{lane} trusted_anchor object missing: {raw}")
    else:
        try:
            anchor_height = int(trusted_anchor.get("height") or 0)
        except (TypeError, ValueError):
            anchor_height = 0
        if anchor_height <= 0:
            blockers.append(f"{lane} trusted_anchor.height must be positive: {raw}")
        if not str(trusted_anchor.get("block_hash") or "").strip():
            blockers.append(f"{lane} trusted_anchor.block_hash missing: {raw}")

    observed_head = data.get("observed_head")
    observed_height = 0
    if not isinstance(observed_head, dict):
        blockers.append(f"{lane} observed_head object missing: {raw}")
    else:
        try:
            observed_height = int(observed_head.get("height") or 0)
        except (TypeError, ValueError):
            observed_height = 0
        if observed_height <= 0:
            blockers.append(f"{lane} observed_head.height must be positive: {raw}")
        for key in ("hash", "state_root"):
            if not str(observed_head.get(key) or "").strip():
                blockers.append(f"{lane} observed_head.{key} missing: {raw}")

    verified_range = data.get("verified_range")
    from_height = 0
    to_height = 0
    proof_count = 0
    if not isinstance(verified_range, dict):
        blockers.append(f"{lane} verified_range object missing: {raw}")
    else:
        try:
            from_height = int(verified_range.get("from_height") or 0)
            to_height = int(verified_range.get("to_height") or 0)
            proof_count = int(verified_range.get("proof_count") or 0)
        except (TypeError, ValueError):
            from_height = 0
            to_height = 0
            proof_count = 0
        if from_height <= 0:
            blockers.append(f"{lane} verified_range.from_height must be positive: {raw}")
        if to_height < from_height:
            blockers.append(f"{lane} verified_range.to_height must be >= from_height: {raw}")
        if observed_height and to_height != observed_height:
            blockers.append(f"{lane} verified_range.to_height must match observed_head.height: {raw}")
        if proof_count != (to_height - from_height + 1):
            blockers.append(f"{lane} verified_range.proof_count must match height span: {raw}")

    if data.get("continuity_result") != "accepted":
        blockers.append(f"{lane} continuity_result must be accepted: {raw}")
    if data.get("fork_or_reorg_result") not in {"none_observed", "rejected"}:
        blockers.append(f"{lane} fork_or_reorg_result must be none_observed or rejected: {raw}")

    proof_refs = data.get("proof_refs")
    proof_hashes = data.get("proof_hashes")
    if not isinstance(proof_refs, list) or not proof_refs:
        blockers.append(f"{lane} proof_refs must be a non-empty array: {raw}")
    elif any(not str(item).strip() for item in proof_refs):
        blockers.append(f"{lane} proof_refs cannot contain empty values: {raw}")
    if not isinstance(proof_hashes, list) or not proof_hashes:
        blockers.append(f"{lane} proof_hashes must be a non-empty array: {raw}")
    elif any(not str(item).strip() for item in proof_hashes):
        blockers.append(f"{lane} proof_hashes cannot contain empty values: {raw}")
    if isinstance(proof_refs, list) and isinstance(proof_hashes, list) and len(proof_refs) != len(proof_hashes):
        blockers.append(f"{lane} proof_hashes length must match proof_refs: {raw}")

    verifier = data.get("window_verifier")
    if not isinstance(verifier, dict):
        blockers.append(f"{lane} window_verifier object missing: {raw}")
    else:
        if verifier.get("schema_version") != "oasis7.world_head_proof_window_verifier.v1":
            blockers.append(f"{lane} window_verifier.schema_version mismatch: {raw}")
        if verifier.get("status") != "pass":
            blockers.append(f"{lane} window_verifier.status must be pass: {raw}")
        if verifier.get("verifier_mode") != "proof_window_continuity":
            blockers.append(f"{lane} window_verifier.verifier_mode mismatch: {raw}")
        if verifier.get("window_contract") != "WorldHeadProofWindowV1":
            blockers.append(f"{lane} window_verifier.window_contract must be WorldHeadProofWindowV1: {raw}")
        if verifier.get("claim_boundary") != "proof_window_continuity_evidence_only_not_full_light_client_or_mainnet_readiness":
            blockers.append(f"{lane} window_verifier.claim_boundary mismatch: {raw}")
        if verifier.get("world_id") != (network_tier.get("world_id") if isinstance(network_tier, dict) else None):
            blockers.append(f"{lane} window_verifier.world_id must match network_tier.world_id: {raw}")
        if int(verifier.get("from_height") or 0) != from_height:
            blockers.append(f"{lane} window_verifier.from_height must match verified_range: {raw}")
        if int(verifier.get("to_height") or 0) != to_height:
            blockers.append(f"{lane} window_verifier.to_height must match verified_range: {raw}")
        if int(verifier.get("proof_count") or 0) != proof_count:
            blockers.append(f"{lane} window_verifier.proof_count must match verified_range: {raw}")
        verifier_anchor = verifier.get("trusted_anchor")
        if isinstance(trusted_anchor, dict):
            if not isinstance(verifier_anchor, dict):
                blockers.append(f"{lane} window_verifier.trusted_anchor object missing: {raw}")
            elif verifier_anchor.get("block_hash") != trusted_anchor.get("block_hash"):
                blockers.append(f"{lane} window_verifier.trusted_anchor must match evidence: {raw}")
        verifier_head = verifier.get("head")
        if isinstance(observed_head, dict) and isinstance(verifier_head, dict):
            if verifier_head.get("block_hash") != observed_head.get("hash"):
                blockers.append(f"{lane} window_verifier.head.block_hash must match observed_head.hash: {raw}")
            if verifier_head.get("state_root") != observed_head.get("state_root"):
                blockers.append(f"{lane} window_verifier.head.state_root must match observed_head.state_root: {raw}")
        else:
            blockers.append(f"{lane} window_verifier.head object missing: {raw}")

    for key in (
        "node_db_access_used",
        "manual_checkpoint_or_data_copy_used",
        "privileged_internal_api_used",
    ):
        if data.get(key) is not False:
            blockers.append(f"{lane} {key} must be false: {raw}")

    does_not_claim = data.get("does_not_claim")
    if not isinstance(does_not_claim, list):
        blockers.append(f"{lane} does_not_claim must be an array: {raw}")
    else:
        required_denials = {
            "full light client security",
            "mainnet-grade finality",
            "trust-minimized validator transition",
            "state proof",
            "receipt proof",
            "DA sampling",
            "multi-client consensus equivalence",
            "ready_for_live_candidate",
        }
        missing_denials = sorted(required_denials.difference(set(does_not_claim)))
        if missing_denials:
            blockers.append(
                f"{lane} does_not_claim missing: "
                + ",".join(missing_denials)
                + f": {raw}"
            )

    residual_risk = data.get("residual_risk")
    if not isinstance(residual_risk, list) or not residual_risk:
        blockers.append(f"{lane} residual_risk must be a non-empty array: {raw}")

    return blockers


def validate_validator_finality_proof_pass_evidence(
    raw: str,
    evidence: pathlib.Path,
    manifest_path: pathlib.Path,
    manifest_ref: str,
    manifest_data: dict,
) -> list[str]:
    lane = "validator_finality_proof_ready"
    blockers: list[str] = []
    try:
        data = json.loads(evidence.read_text(encoding="utf-8"))
    except Exception as exc:
        return [f"{lane} evidence must be JSON: {raw} ({exc})"]

    if data.get("evidence_schema") != "oasis7.validator_finality_proof.v1":
        blockers.append(f"{lane} evidence_schema mismatch: {raw}")
    if data.get("status") != "pass":
        blockers.append(f"{lane} status must be pass: {raw}")
    if data.get("verifier_mode") != "validator_set_finality":
        blockers.append(f"{lane} verifier_mode must be validator_set_finality: {raw}")
    if data.get("independent_process") is not True:
        blockers.append(f"{lane} independent_process must be true: {raw}")
    for key in ("implementation_ref", "command_ref", "finality_proof_ref", "finality_proof_hash"):
        if not str(data.get(key) or "").strip():
            blockers.append(f"{lane} {key} missing: {raw}")

    network_tier = data.get("network_tier")
    if not isinstance(network_tier, dict):
        blockers.append(f"{lane} network_tier object missing: {raw}")
    else:
        if network_tier.get("tier") != "public_testnet":
            blockers.append(f"{lane} network_tier.tier must be public_testnet: {raw}")
        if network_tier.get("network_id") != manifest_data.get("network_id"):
            blockers.append(f"{lane} network_tier.network_id must match manifest: {raw}")
        if network_tier.get("chain_id") != manifest_data.get("chain_id"):
            blockers.append(f"{lane} network_tier.chain_id must match manifest: {raw}")
        for key in ("network_id", "chain_id", "world_id"):
            if not str(network_tier.get(key) or "").strip():
                blockers.append(f"{lane} network_tier.{key} missing: {raw}")

    expected_refs = [
        ("manifest_ref", manifest_ref, manifest_path),
        ("genesis_ref", manifest_data["runtime_refs"]["genesis_ref"], resolve_ref(manifest_data["runtime_refs"]["genesis_ref"])),
        ("bootstrap_peer_ref", manifest_data["runtime_refs"]["bootstrap_peer_ref"], resolve_ref(manifest_data["runtime_refs"]["bootstrap_peer_ref"])),
    ]
    for key, expected_raw, expected_resolved in expected_refs:
        actual = str(data.get(key) or "").strip()
        if not actual:
            blockers.append(f"{lane} {key} missing: {raw}")
        elif not ref_matches(actual, expected_raw, expected_resolved):
            blockers.append(f"{lane} {key} must match manifest: {raw}")

    rpc_ref = str(data.get("rpc_ref") or "").strip()
    status_endpoint_ref = str(data.get("status_endpoint_ref") or "").strip()
    expected_rpc_ref = str(manifest_data.get("endpoint_policy", {}).get("rpc_ref") or "").strip()
    if not rpc_ref and not status_endpoint_ref:
        blockers.append(f"{lane} rpc_ref or status_endpoint_ref missing: {raw}")
    if rpc_ref and rpc_ref != expected_rpc_ref:
        blockers.append(f"{lane} rpc_ref must match manifest endpoint_policy.rpc_ref: {raw}")

    sample_window = data.get("sample_window")
    if not isinstance(sample_window, dict):
        blockers.append(f"{lane} sample_window object missing: {raw}")
    else:
        for key in ("started_at", "ended_at"):
            if not str(sample_window.get(key) or "").strip():
                blockers.append(f"{lane} sample_window.{key} missing: {raw}")

    observed_head = data.get("observed_head")
    observed_height = 0
    observed_hash = ""
    observed_state_root = ""
    if not isinstance(observed_head, dict):
        blockers.append(f"{lane} observed_head object missing: {raw}")
    else:
        try:
            observed_height = int(observed_head.get("height") or 0)
        except (TypeError, ValueError):
            observed_height = 0
        if observed_height <= 0:
            blockers.append(f"{lane} observed_head.height must be positive: {raw}")
        observed_hash = str(observed_head.get("hash") or "").strip()
        observed_state_root = str(observed_head.get("state_root") or "").strip()
        if not observed_hash:
            blockers.append(f"{lane} observed_head.hash missing: {raw}")
        if not observed_state_root:
            blockers.append(f"{lane} observed_head.state_root missing: {raw}")

    verified_range = data.get("verified_range")
    from_height = 0
    to_height = 0
    if not isinstance(verified_range, dict):
        blockers.append(f"{lane} verified_range object missing: {raw}")
    else:
        try:
            from_height = int(verified_range.get("from_height") or 0)
            to_height = int(verified_range.get("to_height") or 0)
        except (TypeError, ValueError):
            from_height = 0
            to_height = 0
        if from_height <= 0:
            blockers.append(f"{lane} verified_range.from_height must be positive: {raw}")
        if to_height < from_height:
            blockers.append(f"{lane} verified_range.to_height must be >= from_height: {raw}")
        if observed_height and to_height != observed_height:
            blockers.append(f"{lane} verified_range.to_height must match observed_head.height: {raw}")

    validator_set = data.get("validator_set")
    if not isinstance(validator_set, dict):
        blockers.append(f"{lane} validator_set object missing: {raw}")
    else:
        for key in ("validator_set_id", "validator_set_hash", "quorum_threshold_bps"):
            if not str(validator_set.get(key) or "").strip():
                blockers.append(f"{lane} validator_set.{key} missing: {raw}")
        if int(validator_set.get("validator_count") or 0) <= 0:
            blockers.append(f"{lane} validator_set.validator_count must be positive: {raw}")

    finality_sample = data.get("finality_sample")
    if not isinstance(finality_sample, dict):
        blockers.append(f"{lane} finality_sample object missing: {raw}")
    else:
        for key in (
            "commitment_count",
            "vote_count",
            "stake_threshold_checked",
            "validator_set_hash_checked",
            "consensus_approver_subset_checked",
        ):
            if key not in finality_sample:
                blockers.append(f"{lane} finality_sample.{key} missing: {raw}")
        for key in ("stake_threshold_checked", "validator_set_hash_checked", "consensus_approver_subset_checked"):
            if finality_sample.get(key) is not True:
                blockers.append(f"{lane} finality_sample.{key} must be true: {raw}")
        if int(finality_sample.get("commitment_count") or 0) <= 0:
            blockers.append(f"{lane} finality_sample.commitment_count must be positive: {raw}")
        if int(finality_sample.get("vote_count") or 0) <= 0:
            blockers.append(f"{lane} finality_sample.vote_count must be positive: {raw}")

    if data.get("misbehavior_result") not in {"none_observed", "rejected", "evidence_recorded"}:
        blockers.append(f"{lane} misbehavior_result unsupported: {raw}")
    fork_cases = data.get("fork_or_reorg_cases")
    if not isinstance(fork_cases, list) or not fork_cases:
        blockers.append(f"{lane} fork_or_reorg_cases must be a non-empty array: {raw}")

    verifier = data.get("external_verifier")
    if not isinstance(verifier, dict):
        blockers.append(f"{lane} external_verifier object missing: {raw}")
    else:
        if verifier.get("schema_version") != "oasis7.world_finality_proof_verifier.v1":
            blockers.append(f"{lane} external_verifier.schema_version mismatch: {raw}")
        if verifier.get("status") != "pass":
            blockers.append(f"{lane} external_verifier.status must be pass: {raw}")
        if verifier.get("verifier_mode") != "validator_set_finality":
            blockers.append(f"{lane} external_verifier.verifier_mode mismatch: {raw}")
        if verifier.get("proof_contract") != "WorldFinalityProofV1":
            blockers.append(f"{lane} external_verifier.proof_contract must be WorldFinalityProofV1: {raw}")
        if verifier.get("claim_boundary") != "validator_set_finality_evidence_only_not_full_light_client_or_mainnet_readiness":
            blockers.append(f"{lane} external_verifier.claim_boundary mismatch: {raw}")
        if str(verifier.get("proof_hash") or "").strip() != str(data.get("finality_proof_hash") or "").strip():
            blockers.append(f"{lane} external_verifier.proof_hash must match evidence finality_proof_hash: {raw}")
        if str(verifier.get("world_id") or "").strip() != str(network_tier.get("world_id") if isinstance(network_tier, dict) else "").strip():
            blockers.append(f"{lane} external_verifier.world_id must match network_tier.world_id: {raw}")
        if int(verifier.get("from_height") or 0) != from_height:
            blockers.append(f"{lane} external_verifier.from_height must match verified_range: {raw}")
        if int(verifier.get("to_height") or 0) != to_height:
            blockers.append(f"{lane} external_verifier.to_height must match verified_range: {raw}")
        verifier_head = verifier.get("head")
        if not isinstance(verifier_head, dict):
            blockers.append(f"{lane} external_verifier.head object missing: {raw}")
        else:
            if str(verifier_head.get("block_hash") or "").strip() != observed_hash:
                blockers.append(f"{lane} external_verifier.head.block_hash must match observed_head.hash: {raw}")
            if str(verifier_head.get("state_root") or "").strip() != observed_state_root:
                blockers.append(f"{lane} external_verifier.head.state_root must match observed_head.state_root: {raw}")
        verifier_set = verifier.get("validator_set")
        if isinstance(validator_set, dict) and isinstance(verifier_set, dict):
            if verifier_set.get("validator_set_hash") != validator_set.get("validator_set_hash"):
                blockers.append(f"{lane} external_verifier.validator_set_hash must match evidence: {raw}")
        else:
            blockers.append(f"{lane} external_verifier.validator_set object missing: {raw}")

    for key in (
        "node_db_access_used",
        "manual_checkpoint_or_data_copy_used",
        "privileged_internal_api_used",
    ):
        if data.get(key) is not False:
            blockers.append(f"{lane} {key} must be false: {raw}")

    does_not_claim = data.get("does_not_claim")
    if not isinstance(does_not_claim, list):
        blockers.append(f"{lane} does_not_claim must be an array: {raw}")
    else:
        required_denials = {
            "full light client security",
            "mainnet-grade finality",
            "trust-minimized validator transition",
            "cryptographic signature verification",
            "public validator onboarding open",
            "permissionless validator onboarding",
            "DA sampling",
            "multi-client consensus equivalence",
            "ready_for_live_candidate",
        }
        missing_denials = sorted(required_denials.difference(set(does_not_claim)))
        if missing_denials:
            blockers.append(
                f"{lane} does_not_claim missing: "
                + ",".join(missing_denials)
                + f": {raw}"
            )

    residual_risk = data.get("residual_risk")
    if not isinstance(residual_risk, list) or not residual_risk:
        blockers.append(f"{lane} residual_risk must be a non-empty array: {raw}")
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
            if lane_id == "state_resource_receipt_proof_ready" and status == "pass":
                state_receipt_blockers = validate_state_resource_receipt_proof_pass_evidence(
                    evidence_path, evidence, manifest_path, sys.argv[1], data
                )
                if state_receipt_blockers:
                    raise SystemExit("; ".join(state_receipt_blockers))
            if lane_id == "external_verifier_light_client_lite_ready" and status == "pass":
                verifier_blockers = validate_external_verifier_light_client_lite_pass_evidence(
                    evidence_path, evidence, manifest_path, sys.argv[1], data
                )
                if verifier_blockers:
                    raise SystemExit("; ".join(verifier_blockers))
            if lane_id == "light_client_continuity_window_ready" and status == "pass":
                window_blockers = validate_light_client_continuity_window_pass_evidence(
                    evidence_path, evidence, manifest_path, sys.argv[1], data
                )
                if window_blockers:
                    raise SystemExit("; ".join(window_blockers))
            if lane_id == "validator_finality_proof_ready" and status == "pass":
                finality_blockers = validate_validator_finality_proof_pass_evidence(
                    evidence_path, evidence, manifest_path, sys.argv[1], data
                )
                if finality_blockers:
                    raise SystemExit("; ".join(finality_blockers))
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
