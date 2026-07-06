#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage:
  ./scripts/network-tier-light-client-continuity-window.sh \
    --manifest <public-testnet-manifest.json> \
    --proof-window <window.json> \
    --world-id <id> \
    --expect-from-height <height> \
    --expect-to-height <height> \
    --expect-anchor-hash <hash> \
    --sample-started-at <iso8601> \
    --sample-ended-at <iso8601> \
    --out <evidence.json> \
    [--status-endpoint-ref <ref>]

Purpose:
  Verify a contiguous WorldHeadProofV1 window from an external process and emit
  a bounded continuity evidence lane. This strengthens light-client-lite
  sampled evidence but does not claim full light-client security, validator-set
  transition security, state/receipt proof, DA sampling, or mainnet-grade
  finality.
USAGE
}

manifest=""
proof_window=""
world_id=""
expect_from_height=""
expect_to_height=""
expect_anchor_hash=""
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
    --proof-window)
      proof_window=${2:-}
      shift 2
      ;;
    --world-id)
      world_id=${2:-}
      shift 2
      ;;
    --expect-from-height)
      expect_from_height=${2:-}
      shift 2
      ;;
    --expect-to-height)
      expect_to_height=${2:-}
      shift 2
      ;;
    --expect-anchor-hash)
      expect_anchor_hash=${2:-}
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
require_value "--proof-window" "$proof_window"
require_value "--world-id" "$world_id"
require_value "--expect-from-height" "$expect_from_height"
require_value "--expect-to-height" "$expect_to_height"
require_value "--expect-anchor-hash" "$expect_anchor_hash"
require_value "--sample-started-at" "$sample_started_at"
require_value "--sample-ended-at" "$sample_ended_at"
require_value "--out" "$out"

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT
verifier_summary="$tmpdir/proof-window-verifier-summary.json"

cargo run -p oasis7_proto --quiet --bin oasis7_world_head_proof_verify -- \
  --proof-window "$proof_window" \
  --expect-world-id "$world_id" \
  --expect-from-height "$expect_from_height" \
  --expect-to-height "$expect_to_height" \
  --expect-anchor-hash "$expect_anchor_hash" \
  --json >"$verifier_summary"

python3 - \
  "$manifest" \
  "$proof_window" \
  "$world_id" \
  "$expect_from_height" \
  "$expect_to_height" \
  "$expect_anchor_hash" \
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
proof_window_path = pathlib.Path(sys.argv[2])
world_id = sys.argv[3]
expect_from_height = int(sys.argv[4])
expect_to_height = int(sys.argv[5])
expect_anchor_hash = sys.argv[6]
sample_started_at = sys.argv[7]
sample_ended_at = sys.argv[8]
status_endpoint_ref = sys.argv[9]
verifier_summary_path = pathlib.Path(sys.argv[10])
out_path = pathlib.Path(sys.argv[11])

manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
verifier = json.loads(verifier_summary_path.read_text(encoding="utf-8"))
runtime_refs = manifest.get("runtime_refs") or {}
endpoint_policy = manifest.get("endpoint_policy") or {}

if manifest.get("tier") != "public_testnet":
    raise SystemExit("manifest tier must be public_testnet")
if verifier.get("status") != "pass":
    raise SystemExit("proof-window verifier summary status must be pass")
if verifier.get("verifier_mode") != "proof_window_continuity":
    raise SystemExit("proof-window verifier_mode must be proof_window_continuity")
if verifier.get("world_id") != world_id:
    raise SystemExit(
        f"proof-window world_id mismatch: expected={world_id} actual={verifier.get('world_id')}"
    )
if int(verifier.get("from_height") or 0) != expect_from_height:
    raise SystemExit("proof-window from_height mismatch")
if int(verifier.get("to_height") or 0) != expect_to_height:
    raise SystemExit("proof-window to_height mismatch")

trusted_anchor = verifier.get("trusted_anchor") or {}
if trusted_anchor.get("block_hash") != expect_anchor_hash:
    raise SystemExit("proof-window trusted anchor hash mismatch")

head = verifier.get("head") or {}
if int(head.get("height") or 0) != expect_to_height:
    raise SystemExit("proof-window observed head height mismatch")
if not str(head.get("block_hash") or "").strip():
    raise SystemExit("proof-window observed head hash missing")
if not str(head.get("state_root") or "").strip():
    raise SystemExit("proof-window observed state root missing")

proof_refs = verifier.get("proof_refs")
proof_hashes = verifier.get("proof_hashes")
if not isinstance(proof_refs, list) or len(proof_refs) == 0:
    raise SystemExit("proof-window proof_refs must be a non-empty array")
if not isinstance(proof_hashes, list) or len(proof_hashes) != len(proof_refs):
    raise SystemExit("proof-window proof_hashes must match proof_refs")

command_ref = (
    "cargo run -p oasis7_proto --bin oasis7_world_head_proof_verify -- "
    "--proof-window <window.json> "
    f"--expect-world-id {world_id} "
    f"--expect-from-height {expect_from_height} "
    f"--expect-to-height {expect_to_height} "
    "--expect-anchor-hash <hash> --json"
)
evidence = {
    "evidence_schema": "oasis7.light_client_continuity_window.v1",
    "status": "pass",
    "verifier_mode": "proof_window_continuity",
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
    "proof_window_ref": str(proof_window_path),
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
        "proof_count": verifier.get("proof_count"),
    },
    "continuity_result": "accepted",
    "fork_or_reorg_result": "none_observed",
    "proof_refs": proof_refs,
    "proof_hashes": proof_hashes,
    "window_verifier": verifier,
    "verified_at_unix_ms": int(time.time() * 1000),
    "node_db_access_used": False,
    "manual_checkpoint_or_data_copy_used": False,
    "privileged_internal_api_used": False,
    "does_not_claim": [
        "full light client security",
        "mainnet-grade finality",
        "trust-minimized validator transition",
        "state proof",
        "receipt proof",
        "DA sampling",
        "multi-client consensus equivalence",
        "ready_for_live_candidate",
    ],
    "residual_risk": [
        "continuity window verifies bounded head/proof linkage only",
        "validator-set transition and finality signature verification are not yet proven",
    ],
}

out_path.parent.mkdir(parents=True, exist_ok=True)
out_path.write_text(json.dumps(evidence, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
print(str(out_path))
PY
