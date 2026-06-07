#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-public-testnet-preflight-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

cat >"$TMP_DIR/sequencer-status.json" <<'JSON'
{
  "node_id": "triad-testnet-sequencer",
  "consensus": {
    "committed_height": 12,
    "last_execution_height": 12
  },
  "replication": {
    "local_peer_id": "12D3KooWMyPapumCaTABq27umWdHqXDr8AoTse21eMVnXeJEsbNp"
  }
}
JSON

cat >"$TMP_DIR/storage-status.json" <<'JSON'
{
  "node_id": "triad-testnet-storage",
  "consensus": {
    "committed_height": 12,
    "last_execution_height": 12
  },
  "replication": {
    "local_peer_id": "12D3KooWAuNCCEDu7CdUUDwALuAhuLekZHgVWxAYp4Ag5ti79fJj"
  }
}
JSON

cat >"$TMP_DIR/node.env" <<'EOF'
NODE_ID=test
REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV=/ip4/old/tcp/1/p2p/oldpeer
EOF

mkdir -p "$TMP_DIR/seed/world" "$TMP_DIR/seed/execution-records" "$TMP_DIR/seed/store/blobs"
cat >"$TMP_DIR/seed/world/snapshot.json" <<'JSON'
{"state":{}}
JSON
cat >"$TMP_DIR/seed/world/journal.json" <<'JSON'
[]
JSON
cat >"$TMP_DIR/seed/execution-records/latest.json" <<'JSON'
{
  "snapshot_ref": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "journal_ref": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
}
JSON
printf 'a\n' >"$TMP_DIR/seed/store/blobs/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.blob"
printf 'b\n' >"$TMP_DIR/seed/store/blobs/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.blob"

"$ROOT_DIR/scripts/p2p-public-testnet-preflight.sh" \
  --bundle "$ROOT_DIR/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json" \
  --sequencer-status-json "$TMP_DIR/sequencer-status.json" \
  --sequencer-ip 39.104.204.172 \
  --sequencer-port 6831 \
  --storage-status-json "$TMP_DIR/storage-status.json" \
  --storage-ip 39.104.205.67 \
  --storage-port 6832 \
  --observer-env "$TMP_DIR/node.env" \
  --seed-root "$TMP_DIR/seed" \
  --out-dir "$TMP_DIR/out" \
  >"$TMP_DIR/preflight.out"

test -f "$TMP_DIR/out/deployment-truth.json"
test -f "$TMP_DIR/out/preflight-summary.json"
test -f "$TMP_DIR/out/seed-closure-seed.json"

jq -e '.ok == true' "$TMP_DIR/out/preflight-summary.json" >/dev/null
grep -q '^REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV=/ip4/39.104.204.172/tcp/6831/p2p/12D3KooWMyPapumCaTABq27umWdHqXDr8AoTse21eMVnXeJEsbNp,/ip4/39.104.205.67/tcp/6832/p2p/12D3KooWAuNCCEDu7CdUUDwALuAhuLekZHgVWxAYp4Ag5ti79fJj$' "$TMP_DIR/node.env"
