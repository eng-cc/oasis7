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
    "last_execution_height": 12
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
    "last_execution_height": 11
  },
  "replication": {
    "local_peer_id": "12D3KooWStoragePeer"
  }
}
JSON

printf 'runtime-sequencer\n' >"$TMP_DIR/sequencer-runtime"
printf 'runtime-storage\n' >"$TMP_DIR/storage-runtime"
printf 'key\n' >"$TMP_DIR/sequencer-node-keypair.toml"

"$ROOT_DIR/scripts/p2p-public-testnet-capture-truth.sh" \
  --bundle "$ROOT_DIR/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json" \
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
  and .validators.sequencer.node_keypair_present == true
  and .validators.storage.node_keypair_present == false
' "$TMP_DIR/out.json" >/dev/null
