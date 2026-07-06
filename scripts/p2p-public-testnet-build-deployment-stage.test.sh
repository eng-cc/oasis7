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

assert_fails_containing() {
  local expected="$1"
  shift
  local out="$TMP_DIR/fail-$RANDOM.log"
  if "$@" >"$out" 2>&1; then
    echo "expected command to fail: $*" >&2
    exit 1
  fi
  if ! grep -Fq -- "$expected" "$out"; then
    echo "expected failure output to contain: $expected" >&2
    echo "--- output ---" >&2
    cat "$out" >&2
    exit 1
  fi
}

"$ROOT_DIR/scripts/p2p-public-testnet-build-deployment-stage.sh" \
  --runtime-build-ref "$TMP_DIR/oasis7_chain_runtime" \
  --bootstrap-peers-file "$TMP_DIR/bootstrap-peers.txt" \
  --sequencer-finality-public-key 65c27d898af9c528ebd6a3762373faef110bb7bb515dfa88c447f292474aac16 \
  --storage-finality-public-key 858e97be96f238ef3f6e07ec36d4ba5f503755ecb232d06a80ef1ab8aaca44f6 \
  --extra-validator triad-testnet-fourth-local:f640bc1ceb82b261baf51ab1504a2dc4c10901873252e67551dcfe1f5b7b21af:100 \
  --out-dir "$TMP_DIR/stage" >/dev/null

test -f "$TMP_DIR/stage/config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json"
test -f "$TMP_DIR/stage/config/public-testnet-governance-public-signers-deployment-2026-06-06.json"
test -f "$TMP_DIR/stage/config/public-testnet-governed-bootstrap-bundle-2026-06-06.json"
test -f "$TMP_DIR/stage/generated-world/world/snapshot.json"
test -f "$TMP_DIR/stage/generated-world/generated-scenario-world/snapshot.json"
test -f "$TMP_DIR/stage/generated-world/generated-scenario-world/journal.json"
test -f "$TMP_DIR/stage/generated-world/world-generation-provenance.json"

jq -e '
  .validators[0].finality_signer_public_key == "65c27d898af9c528ebd6a3762373faef110bb7bb515dfa88c447f292474aac16"
	  and .validators[1].finality_signer_public_key == "858e97be96f238ef3f6e07ec36d4ba5f503755ecb232d06a80ef1ab8aaca44f6"
	  and .validators[2].node_id == "triad-testnet-fourth-local"
	  and .validators[2].finality_signer_public_key == "f640bc1ceb82b261baf51ab1504a2dc4c10901873252e67551dcfe1f5b7b21af"
	  and .threshold == 2
	' "$TMP_DIR/stage/config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json" >/dev/null

jq -e '
  ([.entries[] | select(.slot_id == "governance.finality.v1")] | length) == 3
  and ([.entries[] | select(.slot_id == "governance.finality.v1")][0].signer_id == "triad-testnet-sequencer")
  and ([.entries[] | select(.slot_id == "governance.finality.v1")][0].public_key_hex == "65c27d898af9c528ebd6a3762373faef110bb7bb515dfa88c447f292474aac16")
  and ([.entries[] | select(.slot_id == "governance.finality.v1")][1].signer_id == "triad-testnet-storage")
  and ([.entries[] | select(.slot_id == "governance.finality.v1")][1].public_key_hex == "858e97be96f238ef3f6e07ec36d4ba5f503755ecb232d06a80ef1ab8aaca44f6")
  and ([.entries[] | select(.slot_id == "governance.finality.v1")][2].signer_id == "triad-testnet-fourth-local")
  and ([.entries[] | select(.slot_id == "governance.finality.v1")][2].public_key_hex == "f640bc1ceb82b261baf51ab1504a2dc4c10901873252e67551dcfe1f5b7b21af")
  and .truth_kind == "deployment_public_signers"
' "$TMP_DIR/stage/config/public-testnet-governance-public-signers-deployment-2026-06-06.json" >/dev/null

jq -e '
  (.governance_bootstrap_refs.governance_public_manifest_ref | endswith("public-testnet-governance-public-signers-deployment-2026-06-06.json"))
  and (.governance_bootstrap_refs.genesis_validator_registry_ref | endswith("public-testnet-governed-bootstrap-validator-registry-2026-06-06.json"))
' "$TMP_DIR/stage/config/public-testnet-governed-bootstrap-genesis-2026-06-06.json" >/dev/null

jq -e '
  .runtime_build.sha256 != null
  and .governance_manifest.sha256 != null
  and .generated_world_sidecar.sha256_tree != null
  and (.generated_world_sidecar.ref | endswith("generated-world/generated-scenario-world"))
  and .world_generation_provenance.sha256 != null
  and (.world_generation_provenance.ref | endswith("generated-world/world-generation-provenance.json"))
' \
  "$TMP_DIR/stage/config/public-testnet-governed-bootstrap-bundle-2026-06-06.json" >/dev/null

jq -e '.scenario_id == "asteroid_fragment_bootstrap"' \
  "$TMP_DIR/stage/generated-world/world-generation-provenance.json" >/dev/null

jq -e '.track == "public_testnet_rehearsal"' \
  "$TMP_DIR/stage/config/public-testnet-governed-bootstrap-bundle-2026-06-06.json" >/dev/null

jq -e '
  .runtime_refs.release_candidate_bundle_ref == "public-testnet-governed-bootstrap-bundle-2026-06-06.json"
  and .runtime_refs.genesis_ref == "public-testnet-governed-bootstrap-genesis-2026-06-06.json"
  and .runtime_refs.bootstrap_peer_ref == "public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt"
  and .runtime_refs.generated_world_sidecar_ref == "generated-world/generated-scenario-world"
  and .runtime_refs.world_generation_provenance_ref == "generated-world/world-generation-provenance.json"
  and .validator_policy.target_validator_count == 3
' "$TMP_DIR/stage/config/public-testnet-governed-bootstrap-manifest-2026-06-06.json" >/dev/null

grep -q 'triad-testnet-fourth-local' "$TMP_DIR/stage/deployment-truth.md"
grep -q 'Generated map sidecar: `generated-world/generated-scenario-world`' "$TMP_DIR/stage/deployment-truth.md"
grep -q 'Generated map provenance: `generated-world/world-generation-provenance.json`' "$TMP_DIR/stage/deployment-truth.md"

"$ROOT_DIR/scripts/p2p-public-testnet-build-deployment-stage.sh" \
  --runtime-build-ref "$TMP_DIR/oasis7_chain_runtime" \
  --bootstrap-peers-file "$TMP_DIR/bootstrap-peers.txt" \
  --sequencer-public-key 65c27d898af9c528ebd6a3762373faef110bb7bb515dfa88c447f292474aac16 \
  --storage-public-key 858e97be96f238ef3f6e07ec36d4ba5f503755ecb232d06a80ef1ab8aaca44f6 \
  --extra-validator triad-testnet-fourth-local:f640bc1ceb82b261baf51ab1504a2dc4c10901873252e67551dcfe1f5b7b21af:100 \
  --extra-validator triad-testnet-fifth-local:99aabbccddeeff00112233445566778899aabbccddeeff001122334455667788:100 \
  --out-dir "$TMP_DIR/stage-four-validators" >/dev/null

jq -e '
  (.validators | length) == 4
  and .threshold == 3
' "$TMP_DIR/stage-four-validators/config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json" >/dev/null

assert_fails_containing \
  'duplicate validator node_id `triad-testnet-sequencer`' \
  "$ROOT_DIR/scripts/p2p-public-testnet-build-deployment-stage.sh" \
    --runtime-build-ref "$TMP_DIR/oasis7_chain_runtime" \
    --bootstrap-peers-file "$TMP_DIR/bootstrap-peers.txt" \
    --sequencer-public-key 65c27d898af9c528ebd6a3762373faef110bb7bb515dfa88c447f292474aac16 \
    --storage-public-key 858e97be96f238ef3f6e07ec36d4ba5f503755ecb232d06a80ef1ab8aaca44f6 \
    --extra-validator triad-testnet-sequencer:f640bc1ceb82b261baf51ab1504a2dc4c10901873252e67551dcfe1f5b7b21af:100 \
    --out-dir "$TMP_DIR/stage-duplicate"

assert_fails_containing \
  'validator finality public key must be 32-byte hex for node_id=triad-testnet-bad-key' \
  "$ROOT_DIR/scripts/p2p-public-testnet-build-deployment-stage.sh" \
    --runtime-build-ref "$TMP_DIR/oasis7_chain_runtime" \
    --bootstrap-peers-file "$TMP_DIR/bootstrap-peers.txt" \
    --sequencer-public-key 65c27d898af9c528ebd6a3762373faef110bb7bb515dfa88c447f292474aac16 \
    --storage-public-key 858e97be96f238ef3f6e07ec36d4ba5f503755ecb232d06a80ef1ab8aaca44f6 \
    --extra-validator triad-testnet-bad-key:not-hex:100 \
    --out-dir "$TMP_DIR/stage-bad-key"

printf '[node]\npublic_key = "e57c3c343887766b09fb247a9373a6db5c77e41b5fe69584573bdbe000ab220e"\n' >"$TMP_DIR/node-keypair.toml"
assert_fails_containing \
  '--sequencer-node-keypair is not supported for deployment signer truth; pass --sequencer-finality-public-key' \
  "$ROOT_DIR/scripts/p2p-public-testnet-build-deployment-stage.sh" \
    --runtime-build-ref "$TMP_DIR/oasis7_chain_runtime" \
    --bootstrap-peers-file "$TMP_DIR/bootstrap-peers.txt" \
    --sequencer-node-keypair "$TMP_DIR/node-keypair.toml" \
    --storage-finality-public-key 858e97be96f238ef3f6e07ec36d4ba5f503755ecb232d06a80ef1ab8aaca44f6 \
    --out-dir "$TMP_DIR/stage-node-keypair"
