#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-capture-truth-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

cat >"$TMP_DIR/sequencer-status.json" <<'JSON'
{
  "node_id": "triad-testnet-sequencer",
  "consensus": {
    "committed_height": 12,
    "last_block_hash": "seq-block-12",
    "last_execution_height": 12,
    "last_execution_block_hash": "seq-exec-block-12",
    "last_execution_state_root": "seq-state-root-12",
    "network_head": {
      "decision": "ready",
      "height": 12,
      "block_hash": "seq-block-12",
      "execution_block_hash": "seq-exec-block-12",
      "execution_state_root": "seq-state-root-12"
    }
  },
  "readiness": {
    "status": "ready",
    "failed_gates": []
  },
  "network_tier": {
    "tier": "public_testnet",
    "chain_id": "oasis7-public-testnet"
  },
  "chain_proof": {
    "schema_version": "oasis7.chain_proof_status.v1",
    "proof_contract": "WorldHeadProofV1",
    "claim_boundary": "head_execution_checkpoint_evidence_only_not_light_client_or_mainnet_readiness",
    "status": "available",
    "latest_world_head_proof": {
      "schema_version": 1,
      "world_id": "oasis7-public-testnet",
      "height": 12,
      "execution_block_hash": "seq-exec-block-12",
      "execution_state_root": "seq-state-root-12",
      "node_block_hash": "seq-block-12",
      "action_root": "seq-action-root-12",
      "world_head_proof_ref": "cas://seq-proof-12",
      "proof_hash": "seq-proof-hash-12",
      "checkpoint_ref": "00000000000000000012/manifest.json"
    },
    "source_record_path": "/var/lib/oasis7/execution-records/latest.json",
    "load_error": null,
    "does_not_claim": ["ready_for_live_candidate"]
  },
  "replication": {
    "local_peer_id": "12D3KooWSequencerPeer"
  }
}
JSON

cat >"$TMP_DIR/storage-status.json" <<'JSON'
{
  "node_id": "triad-testnet-storage",
  "consensus": {
    "committed_height": 11,
    "last_block_hash": "storage-block-11",
    "last_execution_height": 11,
    "last_execution_block_hash": "storage-exec-block-11",
    "last_execution_state_root": "storage-state-root-11",
    "network_head": {
      "decision": "blocked",
      "height": 12,
      "block_hash": "seq-block-12",
      "execution_block_hash": "seq-exec-block-12",
      "execution_state_root": "seq-state-root-12"
    }
  },
  "readiness": {
    "status": "blocked",
    "failed_gates": ["network_height_lag"]
  },
  "network_tier": {
    "tier": "public_testnet",
    "chain_id": "oasis7-public-testnet"
  },
  "chain_proof": {
    "schema_version": "oasis7.chain_proof_status.v1",
    "proof_contract": "WorldHeadProofV1",
    "claim_boundary": "head_execution_checkpoint_evidence_only_not_light_client_or_mainnet_readiness",
    "status": "stale_or_invalid",
    "latest_world_head_proof": null,
    "source_record_path": "/var/lib/oasis7/execution-records/latest.json",
    "load_error": "world_head_proof_ref missing",
    "does_not_claim": ["ready_for_live_candidate"]
  },
  "replication": {
    "local_peer_id": "12D3KooWStoragePeer"
  }
}
JSON

printf 'runtime-sequencer\n' >"$TMP_DIR/sequencer-runtime"
printf 'runtime-storage\n' >"$TMP_DIR/storage-runtime"
printf 'key\n' >"$TMP_DIR/sequencer-node-keypair.toml"
printf 'runtime-bundle\n' >"$TMP_DIR/runtime-bundle"
mkdir -p "$TMP_DIR/world"
printf '{"world":"snapshot"}\n' >"$TMP_DIR/world/snapshot.json"
printf '{"validators":[]}\n' >"$TMP_DIR/governance.json"
printf 'capture truth smoke evidence\n' >"$TMP_DIR/evidence.md"

"$ROOT_DIR/scripts/release-candidate-bundle.sh" create \
  --bundle "$TMP_DIR/bundle.json" \
  --candidate-id "capture-truth-smoke" \
  --track public_testnet_rehearsal \
  --runtime-build-ref "$TMP_DIR/runtime-bundle" \
  --world-snapshot-ref "$TMP_DIR/world" \
  --governance-manifest-ref "$TMP_DIR/governance.json" \
  --evidence-ref "$TMP_DIR/evidence.md" \
  --allow-dirty-worktree >/dev/null

"$ROOT_DIR/scripts/p2p-public-testnet-capture-truth.sh" \
  --bundle "$TMP_DIR/bundle.json" \
  --sequencer-status-json "$TMP_DIR/sequencer-status.json" \
  --storage-status-json "$TMP_DIR/storage-status.json" \
  --sequencer-runtime-path "$TMP_DIR/sequencer-runtime" \
  --storage-runtime-path "$TMP_DIR/storage-runtime" \
  --sequencer-node-keypair-path "$TMP_DIR/sequencer-node-keypair.toml" \
  --storage-node-keypair-path "$TMP_DIR/missing-node-keypair.toml" \
  --out "$TMP_DIR/out.json"

jq -e '
  .bundle_validate.ok == true
  and .validators.sequencer.node_id == "triad-testnet-sequencer"
  and .validators.storage.node_id == "triad-testnet-storage"
  and .validators.sequencer.local_peer_id == "12D3KooWSequencerPeer"
  and .validators.storage.local_peer_id == "12D3KooWStoragePeer"
  and .validators.sequencer.last_block_hash == "seq-block-12"
  and .validators.sequencer.last_execution_block_hash == "seq-exec-block-12"
  and .validators.sequencer.last_execution_state_root == "seq-state-root-12"
  and .validators.sequencer.network_head.execution_state_root == "seq-state-root-12"
  and .validators.sequencer.readiness.status == "ready"
  and .validators.sequencer.network_tier.tier == "public_testnet"
  and .validators.sequencer.chain_proof.status == "available"
  and .validators.sequencer.chain_proof.latest_world_head_proof.world_head_proof_ref == "cas://seq-proof-12"
  and .validators.sequencer.chain_proof.does_not_claim == ["ready_for_live_candidate"]
  and .validators.storage.last_execution_state_root == "storage-state-root-11"
  and .validators.storage.network_head.decision == "blocked"
  and .validators.storage.readiness.failed_gates == ["network_height_lag"]
  and .validators.storage.chain_proof.status == "stale_or_invalid"
  and .validators.storage.chain_proof.latest_world_head_proof == null
  and .validators.sequencer.node_keypair_present == true
  and .validators.storage.node_keypair_present == false
' "$TMP_DIR/out.json" >/dev/null
