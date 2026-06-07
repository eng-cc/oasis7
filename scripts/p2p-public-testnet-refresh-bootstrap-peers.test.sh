#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-refresh-bootstrap-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

cat >"$TMP_DIR/sequencer-status.json" <<'JSON'
{
  "replication": {
    "local_peer_id": "12D3KooWMyPapumCaTABq27umWdHqXDr8AoTse21eMVnXeJEsbNp"
  }
}
JSON

cat >"$TMP_DIR/storage-status.json" <<'JSON'
{
  "replication": {
    "local_peer_id": "12D3KooWAuNCCEDu7CdUUDwALuAhuLekZHgVWxAYp4Ag5ti79fJj"
  }
}
JSON

cat >"$TMP_DIR/node.env" <<'EOF'
NODE_ID=test
REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV=/ip4/old/tcp/1/p2p/oldpeer
EOF

csv=$("$ROOT_DIR/scripts/p2p-public-testnet-refresh-bootstrap-peers.sh" \
  --sequencer-status-json "$TMP_DIR/sequencer-status.json" \
  --sequencer-ip 39.104.204.172 \
  --sequencer-port 6831 \
  --storage-status-json "$TMP_DIR/storage-status.json" \
  --storage-ip 39.104.205.67 \
  --storage-port 6832 \
  --env-file "$TMP_DIR/node.env")

expected="/ip4/39.104.204.172/tcp/6831/p2p/12D3KooWMyPapumCaTABq27umWdHqXDr8AoTse21eMVnXeJEsbNp,/ip4/39.104.205.67/tcp/6832/p2p/12D3KooWAuNCCEDu7CdUUDwALuAhuLekZHgVWxAYp4Ag5ti79fJj"
test "$csv" = "$expected"
grep -q "^REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV=$expected$" "$TMP_DIR/node.env"
