#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage:
  ./scripts/network-tier-external-verifier-light-client-lite.sh \
    --manifest <public-testnet-manifest.json> \
    --proof <world-head-proof.cbor|json> \
    --proof-ref <ref> \
    --world-id <id> \
    --expect-height <height> \
    --observed-head-hash <hash> \
    --observed-state-root <root> \
    --from-height <height> \
    --sample-started-at <iso8601> \
    --sample-ended-at <iso8601> \
    --out <evidence.json> \
    [--proof-format cbor|json] \
    [--status-endpoint-ref <ref>]

Purpose:
  Run the repo-owned WorldHeadProofV1 verifier as an external/light-client-lite
  operator entrypoint and emit readiness-lane evidence. This is not a full
  light client, state proof, receipt proof, DA sampling proof, or mainnet-grade
  finality claim.
USAGE
}

manifest=""
proof=""
proof_ref=""
proof_format="cbor"
world_id=""
expect_height=""
observed_head_hash=""
observed_state_root=""
from_height=""
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
    --proof)
      proof=${2:-}
      shift 2
      ;;
    --proof-ref)
      proof_ref=${2:-}
      shift 2
      ;;
    --proof-format)
      proof_format=${2:-}
      shift 2
      ;;
    --world-id)
      world_id=${2:-}
      shift 2
      ;;
    --expect-height)
      expect_height=${2:-}
      shift 2
      ;;
    --observed-head-hash)
      observed_head_hash=${2:-}
      shift 2
      ;;
    --observed-state-root)
      observed_state_root=${2:-}
      shift 2
      ;;
    --from-height)
      from_height=${2:-}
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
require_value "--proof" "$proof"
require_value "--proof-ref" "$proof_ref"
require_value "--world-id" "$world_id"
require_value "--expect-height" "$expect_height"
require_value "--observed-head-hash" "$observed_head_hash"
require_value "--observed-state-root" "$observed_state_root"
require_value "--from-height" "$from_height"
require_value "--sample-started-at" "$sample_started_at"
require_value "--sample-ended-at" "$sample_ended_at"
require_value "--out" "$out"

if [[ "$proof_format" != "cbor" && "$proof_format" != "json" ]]; then
  echo "error: --proof-format must be cbor or json" >&2
  exit 2
fi

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT
verifier_summary="$tmpdir/verifier-summary.json"

cargo run -p oasis7_proto --quiet --bin oasis7_world_head_proof_verify -- \
  --proof "$proof" \
  --proof-ref "$proof_ref" \
  --format "$proof_format" \
  --expect-world-id "$world_id" \
  --expect-height "$expect_height" \
  --json >"$verifier_summary"

python3 - \
  "$manifest" \
  "$proof_ref" \
  "$world_id" \
  "$expect_height" \
  "$observed_head_hash" \
  "$observed_state_root" \
  "$from_height" \
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
proof_ref = sys.argv[2]
world_id = sys.argv[3]
expect_height = int(sys.argv[4])
observed_head_hash = sys.argv[5]
observed_state_root = sys.argv[6]
from_height = int(sys.argv[7])
sample_started_at = sys.argv[8]
sample_ended_at = sys.argv[9]
status_endpoint_ref = sys.argv[10]
verifier_summary_path = pathlib.Path(sys.argv[11])
out_path = pathlib.Path(sys.argv[12])

manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
verifier = json.loads(verifier_summary_path.read_text(encoding="utf-8"))
runtime_refs = manifest.get("runtime_refs") or {}
endpoint_policy = manifest.get("endpoint_policy") or {}

if from_height > expect_height:
    raise SystemExit(
        "verified range is inverted: "
        f"from_height={from_height} to_height={expect_height}"
    )
if manifest.get("tier") != "public_testnet":
    raise SystemExit("manifest tier must be public_testnet")
if verifier.get("status") != "pass":
    raise SystemExit("verifier summary status must be pass")
verified_head = verifier.get("head") or {}
if observed_head_hash != (verified_head.get("block_hash") or ""):
    raise SystemExit(
        "observed head hash mismatch: "
        f"observed={observed_head_hash} verified={verified_head.get('block_hash') or ''}"
    )
if observed_state_root != (verified_head.get("state_root") or ""):
    raise SystemExit(
        "observed state root mismatch: "
        f"observed={observed_state_root} verified={verified_head.get('state_root') or ''}"
    )

command_ref = (
    "cargo run -p oasis7_proto --bin oasis7_world_head_proof_verify -- "
    f"--proof <proof> --format <format> --expect-world-id {world_id} "
    f"--expect-height {expect_height} --json"
)
evidence = {
    "evidence_schema": "oasis7.external_verifier_light_client_lite.v1",
    "status": "pass",
    "verifier_mode": "external_light_client_lite",
    "independent_process": True,
    "implementation_ref": "crates/oasis7_proto/src/bin/oasis7_world_head_proof_verify.rs",
    "command_ref": command_ref,
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
    "sample_window": {
        "started_at": sample_started_at,
        "ended_at": sample_ended_at,
    },
    "observed_head": {
        "height": expect_height,
        "hash": observed_head_hash,
        "state_root": observed_state_root,
    },
    "verified_range": {
        "from_height": from_height,
        "to_height": expect_height,
    },
    "verification_result": "accepted",
    "proof_ref": proof_ref,
    "proof_hash": verifier.get("proof_hash") or "",
    "external_verifier": verifier,
    "verified_at_unix_ms": int(time.time() * 1000),
    "node_db_access_used": False,
    "manual_checkpoint_or_data_copy_used": False,
    "privileged_internal_api_used": False,
    "does_not_claim": [
        "mainnet-grade",
        "production OC settlement",
        "public validator onboarding open",
        "multi-client consensus equivalence",
        "full light client security",
        "ready_for_live_candidate",
    ],
    "residual_risk": [
        "external verifier is light-client-lite only and does not prove full consensus security",
    ],
}

out_path.parent.mkdir(parents=True, exist_ok=True)
out_path.write_text(json.dumps(evidence, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
print(str(out_path))
PY
