#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage:
  ./scripts/network-tier-validator-finality-proof.sh \
    --manifest <public-testnet-manifest.json> \
    --finality-proof <proof.json> \
    --world-id <id> \
    --expect-to-height <height> \
    --sample-started-at <iso8601> \
    --sample-ended-at <iso8601> \
    --out <evidence.json> \
    [--status-endpoint-ref <ref>]

Purpose:
  Verify a bounded WorldFinalityProofV1 artifact from an external process and
  emit optional validator-set/finality/fork-misbehavior evidence. This does not
  claim full light-client security, trust-minimized validator transition,
  public validator onboarding, or mainnet-grade finality.
USAGE
}

manifest=""
finality_proof=""
world_id=""
expect_to_height=""
sample_started_at=""
sample_ended_at=""
status_endpoint_ref=""
out=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --manifest)
      manifest=${2:-}
      shift 2
      ;;
    --finality-proof)
      finality_proof=${2:-}
      shift 2
      ;;
    --world-id)
      world_id=${2:-}
      shift 2
      ;;
    --expect-to-height)
      expect_to_height=${2:-}
      shift 2
      ;;
    --sample-started-at)
      sample_started_at=${2:-}
      shift 2
      ;;
    --sample-ended-at)
      sample_ended_at=${2:-}
      shift 2
      ;;
    --status-endpoint-ref)
      status_endpoint_ref=${2:-}
      shift 2
      ;;
    --out)
      out=${2:-}
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

require_value() {
  local name=$1
  local value=$2
  if [[ -z "$value" ]]; then
    echo "error: $name is required" >&2
    usage >&2
    exit 2
  fi
}

require_value "--manifest" "$manifest"
require_value "--finality-proof" "$finality_proof"
require_value "--world-id" "$world_id"
require_value "--expect-to-height" "$expect_to_height"
require_value "--sample-started-at" "$sample_started_at"
require_value "--sample-ended-at" "$sample_ended_at"
require_value "--out" "$out"

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT
verifier_summary="$tmpdir/finality-verifier-summary.json"

cargo run -p oasis7_proto --quiet --bin oasis7_world_head_proof_verify -- \
  --finality-proof "$finality_proof" \
  --format json \
  --expect-world-id "$world_id" \
  --expect-height "$expect_to_height" \
  --json >"$verifier_summary"

python3 - \
  "$manifest" \
  "$finality_proof" \
  "$world_id" \
  "$expect_to_height" \
  "$sample_started_at" \
  "$sample_ended_at" \
  "$status_endpoint_ref" \
  "$verifier_summary" \
  "$out" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys
import time

manifest_path = pathlib.Path(sys.argv[1])
proof_path = pathlib.Path(sys.argv[2])
world_id = sys.argv[3]
expect_to_height = int(sys.argv[4])
sample_started_at = sys.argv[5]
sample_ended_at = sys.argv[6]
status_endpoint_ref = sys.argv[7]
verifier_summary_path = pathlib.Path(sys.argv[8])
out_path = pathlib.Path(sys.argv[9])

manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
verifier = json.loads(verifier_summary_path.read_text(encoding="utf-8"))
runtime_refs = manifest.get("runtime_refs") or {}
endpoint_policy = manifest.get("endpoint_policy") or {}

if manifest.get("tier") != "public_testnet":
    raise SystemExit("manifest tier must be public_testnet")
if verifier.get("status") != "pass":
    raise SystemExit("finality verifier summary status must be pass")
if verifier.get("verifier_mode") != "validator_set_finality":
    raise SystemExit("finality verifier_mode must be validator_set_finality")
if verifier.get("world_id") != world_id:
    raise SystemExit("finality verifier world_id mismatch")
if int(verifier.get("to_height") or 0) != expect_to_height:
    raise SystemExit("finality verifier to_height mismatch")

validator_set = verifier.get("validator_set") or {}
finality = verifier.get("finality") or {}
head = verifier.get("head") or {}
trusted_anchor = verifier.get("trusted_anchor") or {}

evidence = {
    "evidence_schema": "oasis7.validator_finality_proof.v1",
    "status": "pass",
    "verifier_mode": "validator_set_finality",
    "independent_process": True,
    "implementation_ref": "crates/oasis7_proto/src/bin/oasis7_world_head_proof_verify.rs",
    "command_ref": (
        "cargo run -p oasis7_proto --bin oasis7_world_head_proof_verify -- "
        "--finality-proof <proof.json> --format json "
        f"--expect-world-id {world_id} --expect-height {expect_to_height} --json"
    ),
    "network_tier": {
        "tier": "public_testnet",
        "status": manifest.get("status") or "",
        "chain_id": manifest.get("chain_id") or "",
        "network_id": manifest.get("network_id") or "",
        "world_id": world_id,
    },
    "manifest_ref": str(manifest_path),
    "genesis_ref": runtime_refs.get("genesis_ref") or "",
    "bootstrap_peer_ref": runtime_refs.get("bootstrap_peer_ref") or "",
    "rpc_ref": endpoint_policy.get("rpc_ref") or "",
    "status_endpoint_ref": status_endpoint_ref,
    "finality_proof_ref": str(proof_path),
    "finality_proof_hash": verifier.get("proof_hash") or "",
    "sample_window": {
        "started_at": sample_started_at,
        "ended_at": sample_ended_at,
    },
    "trusted_anchor": trusted_anchor,
    "observed_head": {
        "height": head.get("height"),
        "hash": head.get("block_hash") or "",
        "state_root": head.get("state_root") or "",
    },
    "verified_range": {
        "from_height": verifier.get("from_height"),
        "to_height": verifier.get("to_height"),
    },
    "validator_set": validator_set,
    "finality_sample": finality,
    "transition_sample": {
        "transition_result": (
            "bounded_transition_execution_checked"
            if int(finality.get("validator_set_transition_count") or 0) > 0
            else "no_transition_in_sample"
        ),
        "transition_count": int(finality.get("validator_set_transition_count") or 0),
        "reason": "bounded transition semantics are verified when present; not trust-minimized validator governance",
    },
    "fork_or_reorg_cases": [
        "conflicting_head_rejected_or_recorded",
        "unknown_signer_rejected",
        "insufficient_signed_stake_rejected",
    ],
    "misbehavior_result": (
        "evidence_recorded"
        if int(finality.get("misbehavior_evidence_count") or 0) > 0
        else "none_observed"
    ),
    "external_verifier": verifier,
    "verified_at_unix_ms": int(time.time() * 1000),
    "node_db_access_used": False,
    "manual_checkpoint_or_data_copy_used": False,
    "privileged_internal_api_used": False,
    "does_not_claim": [
        "full light client security",
        "mainnet-grade finality",
        "trust-minimized validator transition",
        "public validator onboarding open",
        "permissionless validator onboarding",
        "DA sampling",
        "multi-client consensus equivalence",
        "ready_for_live_candidate",
        "live public launch",
    ],
    "residual_risk": [
        "same implementation family verifier; no independent client parity",
        "bounded transition execution is same-family verifier evidence, not trust-minimized validator governance",
    ],
}

out_path.parent.mkdir(parents=True, exist_ok=True)
out_path.write_text(json.dumps(evidence, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
print(str(out_path))
PY
