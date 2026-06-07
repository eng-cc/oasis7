#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-build-deployment-stage-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

printf 'runtime\n' >"$TMP_DIR/oasis7_chain_runtime"
cat >"$TMP_DIR/bootstrap-peers.txt" <<'EOF'
/ip4/127.0.0.1/tcp/6831/p2p/12D3KooWTestSequencer
/ip4/127.0.0.1/tcp/6832/p2p/12D3KooWTestStorage
EOF

"$ROOT_DIR/scripts/p2p-public-testnet-build-deployment-stage.sh" \
  --runtime-build-ref "$TMP_DIR/oasis7_chain_runtime" \
  --bootstrap-peers-file "$TMP_DIR/bootstrap-peers.txt" \
  --sequencer-public-key 65c27d898af9c528ebd6a3762373faef110bb7bb515dfa88c447f292474aac16 \
  --storage-public-key 858e97be96f238ef3f6e07ec36d4ba5f503755ecb232d06a80ef1ab8aaca44f6 \
  --out-dir "$TMP_DIR/stage" >/dev/null

test -f "$TMP_DIR/stage/config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json"
test -f "$TMP_DIR/stage/config/public-testnet-governed-bootstrap-bundle-2026-06-06.json"
test -f "$TMP_DIR/stage/generated-world/world/snapshot.json"

jq -e '
  .validators[0].finality_signer_public_key == "65c27d898af9c528ebd6a3762373faef110bb7bb515dfa88c447f292474aac16"
  and .validators[1].finality_signer_public_key == "858e97be96f238ef3f6e07ec36d4ba5f503755ecb232d06a80ef1ab8aaca44f6"
' "$TMP_DIR/stage/config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json" >/dev/null

jq -e '.runtime_build.sha256 != null and .governance_manifest.sha256 != null' \
  "$TMP_DIR/stage/config/public-testnet-governed-bootstrap-bundle-2026-06-06.json" >/dev/null
