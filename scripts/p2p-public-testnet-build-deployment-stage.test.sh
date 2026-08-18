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

jq -e '
  ([.entries[] | select(.slot_id == "ops.rollback.on_call.v1")] | length) == 1
  and ([.entries[] | select(.slot_id == "ops.rollback.on_call.v1")][0] == {
    "slot_id": "ops.rollback.on_call.v1",
    "signer_id": "public-testnet-rollback-on-call-01",
    "scheme": "ed25519",
    "threshold": 1,
    "public_key_hex": "9dfad9943645344153bfd0efa982cf4dec8f09aa7d1a3146e65883fd4c997657"
  })
  and ([.entries[] | select(.slot_id == "governance.rollback.v1")] | length) == 1
  and ([.entries[] | select(.slot_id == "governance.rollback.v1")][0] == {
    "slot_id": "governance.rollback.v1",
    "signer_id": "public-testnet-rollback-governance-01",
    "scheme": "ed25519",
    "threshold": 1,
    "public_key_hex": "d9f35c8fc0e0e5df53475cc7059f2f38ab901ee39a5c9c464f65b09ef811bf4a"
  })
  and ([.entries[] | select(.slot_id == "ops.rollback.on_call.v1")][0].signer_id
    != [.entries[] | select(.slot_id == "governance.rollback.v1")][0].signer_id)
  and ([.entries[] | select(.slot_id == "ops.rollback.on_call.v1")][0].public_key_hex
    != [.entries[] | select(.slot_id == "governance.rollback.v1")][0].public_key_hex)
' "$ROOT_DIR/doc/testing/evidence/public-testnet-governance-public-signers-2026-06-05.json" >/dev/null

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
  and ([.entries[] | select(.slot_id == "ops.rollback.on_call.v1")] | length) == 1
  and ([.entries[] | select(.slot_id == "ops.rollback.on_call.v1")][0].signer_id == "public-testnet-rollback-on-call-01")
  and ([.entries[] | select(.slot_id == "ops.rollback.on_call.v1")][0].scheme == "ed25519")
  and ([.entries[] | select(.slot_id == "ops.rollback.on_call.v1")][0].threshold == 1)
  and ([.entries[] | select(.slot_id == "ops.rollback.on_call.v1")][0].public_key_hex == "9dfad9943645344153bfd0efa982cf4dec8f09aa7d1a3146e65883fd4c997657")
  and ([.entries[] | select(.slot_id == "governance.rollback.v1")] | length) == 1
  and ([.entries[] | select(.slot_id == "governance.rollback.v1")][0].signer_id == "public-testnet-rollback-governance-01")
  and ([.entries[] | select(.slot_id == "governance.rollback.v1")][0].scheme == "ed25519")
  and ([.entries[] | select(.slot_id == "governance.rollback.v1")][0].threshold == 1)
  and ([.entries[] | select(.slot_id == "governance.rollback.v1")][0].public_key_hex == "d9f35c8fc0e0e5df53475cc7059f2f38ab901ee39a5c9c464f65b09ef811bf4a")
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

# Optional pair provenance is absent in the first invocation above and must be
# forwarded/validated when explicitly supplied in this second invocation.
mkdir -p "$TMP_DIR/pair-package"
cp "$TMP_DIR/oasis7_chain_runtime" "$TMP_DIR/pair-package/oasis7_chain_runtime"
printf 'run_id=3218\ncommit=%s\npackage_version=0.0.0+testnet.261\n' \
  0123456789abcdef0123456789abcdef01234567 >"$TMP_DIR/pair-package/BUILDINFO"
pair_runtime_sha=$(shasum -a 256 "$TMP_DIR/pair-package/oasis7_chain_runtime" | awk '{print $1}')
printf '%s  oasis7_chain_runtime\n' "$pair_runtime_sha" >"$TMP_DIR/pair-package/SHA256SUMS"
openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out "$TMP_DIR/attestor-key.pem" >/dev/null 2>&1
openssl pkey -in "$TMP_DIR/attestor-key.pem" -pubout -out "$TMP_DIR/attestor-public.pem" >/dev/null 2>&1
python3 "$ROOT_DIR/scripts/p2p-public-testnet-validator-pair-provenance.py" create \
  --package-dir "$TMP_DIR/pair-package" \
  --manifest "$ROOT_DIR/doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json" \
  --genesis "$ROOT_DIR/doc/testing/evidence/public-testnet-governed-bootstrap-genesis-2026-06-06.json" \
  --registry "$ROOT_DIR/doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json" \
  --bootstrap "$TMP_DIR/bootstrap-peers.txt" \
  --world "$ROOT_DIR/doc/testing/evidence/public-testnet-governed-bootstrap-genesis-2026-06-06.json" \
  --network-id oasis7-public-testnet-governed-20260606 \
  --chain-id oasis7-public-testnet-governed-20260606 \
  --output "$TMP_DIR/pair-provenance.json" \
  --signer-id testnet-package-attestor \
  --signature-ref "$TMP_DIR/pair-signature.bin" \
  --public-key-ref "$TMP_DIR/attestor-public.pem" >/dev/null
python3 - "$TMP_DIR/pair-provenance.json" "$TMP_DIR/attestor-key.pem" "$TMP_DIR/pair-signature.bin" <<'PY'
import hashlib
import json
import subprocess
import sys
from pathlib import Path

receipt_path = Path(sys.argv[1])
private_key = sys.argv[2]
signature_path = sys.argv[3]
receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
receipt["signature"]["status"] = "verified"
body = {key: value for key, value in receipt.items() if key != "binding_digest"}
receipt["binding_digest"] = hashlib.sha256(
    json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode("utf-8")
).hexdigest()
receipt_path.write_text(json.dumps(receipt, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
payload_path = receipt_path.with_suffix(".payload")
payload_path.write_bytes(json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode("utf-8"))
subprocess.run(
    ["openssl", "dgst", "-sha256", "-sign", private_key, "-out", signature_path, str(payload_path)],
    check=True,
)
PY

"$ROOT_DIR/scripts/p2p-public-testnet-build-deployment-stage.sh" \
  --runtime-build-ref "$TMP_DIR/oasis7_chain_runtime" \
  --bootstrap-peers-file "$TMP_DIR/bootstrap-peers.txt" \
  --sequencer-finality-public-key 65c27d898af9c528ebd6a3762373faef110bb7bb515dfa88c447f292474aac16 \
  --storage-finality-public-key 858e97be96f238ef3f6e07ec36d4ba5f503755ecb232d06a80ef1ab8aaca44f6 \
  --validator-pair-provenance-ref "$TMP_DIR/pair-provenance.json" \
  --out-dir "$TMP_DIR/stage-with-provenance" >/dev/null
jq -e '.validator_pair_provenance.sha256 != null' \
  "$TMP_DIR/stage-with-provenance/config/public-testnet-governed-bootstrap-bundle-2026-06-06.json" >/dev/null
jq -e '.validator_pair_provenance.resolved_path | contains("stage-with-provenance/config/doc/testing/evidence/")' \
  "$TMP_DIR/stage-with-provenance/config/public-testnet-governed-bootstrap-bundle-2026-06-06.json" >/dev/null
grep -q 'Validator pair provenance:' "$TMP_DIR/stage-with-provenance/deployment-truth.md"
# The staged receipt must retain detached verification after its source receipt
# and detached files leave the temporary input directory.
rm -f "$TMP_DIR/pair-provenance.json" "$TMP_DIR/pair-signature.bin" "$TMP_DIR/attestor-public.pem"
bash "$ROOT_DIR/scripts/release-candidate-bundle.sh" validate \
  --bundle "$TMP_DIR/stage-with-provenance/config/public-testnet-governed-bootstrap-bundle-2026-06-06.json" >/dev/null

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
