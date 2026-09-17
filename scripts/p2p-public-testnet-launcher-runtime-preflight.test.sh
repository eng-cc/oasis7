#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_STAGE="$ROOT_DIR/scripts/p2p-public-testnet-build-deployment-stage.sh"
START_NODE="$ROOT_DIR/scripts/p2p-triad-node-start.sh"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-launcher-runtime-preflight.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

printf 'runtime\n' >"$TMP_DIR/oasis7_chain_runtime"
chmod 644 "$TMP_DIR/oasis7_chain_runtime"
cat >"$TMP_DIR/bootstrap-peers.txt" <<'EOF'
/ip4/127.0.0.1/tcp/6831/p2p/12D3KooWTestSequencer
/ip4/127.0.0.1/tcp/6832/p2p/12D3KooWTestStorage
EOF

# Materialize an arbitrary extra-validator stage through the real generator.
# The stage gets its own generated registry but must not inherit the fixed
# managed-triad inventory authority.
"$BUILD_STAGE" \
  --runtime-build-ref "$TMP_DIR/oasis7_chain_runtime" \
  --bootstrap-peers-file "$TMP_DIR/bootstrap-peers.txt" \
  --sequencer-finality-public-key 65c27d898af9c528ebd6a3762373faef110bb7bb515dfa88c447f292474aac16 \
  --storage-finality-public-key 858e97be96f238ef3f6e07ec36d4ba5f503755ecb232d06a80ef1ab8aaca44f6 \
  --extra-validator triad-testnet-fourth-local:f640bc1ceb82b261baf51ab1504a2dc4c10901873252e67551dcfe1f5b7b21af:100 \
  --out-dir "$TMP_DIR/stage" >/dev/null

stage_manifest="$TMP_DIR/stage/config/public-testnet-governed-bootstrap-manifest-2026-06-06.json"
stage_registry="$TMP_DIR/stage/config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json"
test -f "$stage_manifest"
test -f "$stage_registry"

python3 - "$stage_manifest" "$stage_registry" <<'PY'
import hashlib
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
registry_path = pathlib.Path(sys.argv[2])
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
registry = json.loads(registry_path.read_text(encoding="utf-8"))

if manifest.get("deployment_inventory") is not None:
    raise SystemExit("arbitrary stage must not bind managed-triad deployment inventory")
binding = manifest.get("deployment_validator_registry")
if not isinstance(binding, dict):
    raise SystemExit("arbitrary stage must bind its generated validator registry")
expected_ref = f"config/{registry_path.name}"
if binding.get("ref") != expected_ref:
    raise SystemExit(
        f"arbitrary stage registry ref drifted: expected={expected_ref} actual={binding.get('ref')}"
    )
registry_sha256 = hashlib.sha256(registry_path.read_bytes()).hexdigest()
if binding.get("sha256") != registry_sha256:
    raise SystemExit("arbitrary stage registry digest binding drifted")
canonical = {
    "signer_bindings": {
        f"governance.finality.v1.{item['node_id']}": str(
            item["finality_signer_public_key"]
        ).lower()
        for item in registry["validators"]
    },
    "slot_id": registry["slot_id"],
    "threshold": registry["threshold"],
    "threshold_bps": registry["threshold_bps"],
    "validator_stakes": {
        f"governance.finality.v1.{item['node_id']}": item["stake"]
        for item in registry["validators"]
    },
}
semantic_sha256 = hashlib.sha256(
    json.dumps(canonical, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
).hexdigest()
if binding.get("semantic_sha256") != semantic_sha256:
    raise SystemExit("arbitrary stage semantic registry binding drifted")
PY

for forbidden in \
  "$TMP_DIR/stage/config/public-testnet-validator-triad-inventory.v1.json" \
  "$TMP_DIR/stage/config/doc/testing/evidence/public-testnet-validator-triad-inventory.v1.json" \
  "$TMP_DIR/stage/config/doc/testing/evidence/public-testnet-governed-bootstrap-validator-triad-registry-2026-09-15.json"; do
  [[ ! -e "$forbidden" ]] || {
    echo "arbitrary stage emitted fixed managed-triad authority: $forbidden" >&2
    exit 1
  }
done

# Exercise the exact node-start contract against the generated stage with a
# fake runtime. The fake command proves the launcher reaches the runtime and
# receives the localized manifest-bound registry, without starting a node.
printf '[node]\nkey = "isolated-test-only"\n' >"$TMP_DIR/stage/config/node-keypair.toml"
cat >"$TMP_DIR/capture-runtime" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$@" >"$CAPTURE_PATH"
EOF
chmod 755 "$TMP_DIR/capture-runtime"
cat >"$TMP_DIR/observer.env" <<'EOF'
NODE_ID=triad-testnet-fourth-local
NODE_ROLE=observer
WORLD_ID=oasis7-public-testnet-governed-20260606
STORAGE_PROFILE=release_default
STATUS_BIND=127.0.0.1:0
NODE_GOSSIP_BIND=127.0.0.1:0
CONFIG_PATH=config/node-keypair.toml
EXECUTION_WORLD_DIR=data/execution-world
EXECUTION_RECORDS_DIR=data/execution-records
STORAGE_ROOT=data/storage
RUNTIME_ROOT=data/runtime-root
REPLICATION_ROOT=data/replication-root
NODE_TICK_MS=200
POS_SLOT_DURATION_MS=8000
POS_TICKS_PER_SLOT=10
POS_PROPOSAL_TICK_PHASE=9
POS_MAX_PAST_SLOT_LAG=256
REWARD_RUNTIME_ENABLE=0
REWARD_RUNTIME_EPOCH_DURATION_SECS=60
REWARD_POINTS_PER_CREDIT=100
REWARD_RUNTIME_AUTO_REDEEM=0
NETWORK_TIER_MANIFEST_PATH=config/public-testnet-governed-bootstrap-manifest-2026-06-06.json
GENESIS_VALIDATOR_REGISTRY_PATH=config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json
REPLICATION_NETWORK_LISTEN_ADDRS_CSV=/ip4/0.0.0.0/tcp/6834
EOF
capture_path="$TMP_DIR/runtime-args"
(
  cd "$TMP_DIR/stage"
  CAPTURE_PATH="$capture_path" \
    APP_ROOT="$PWD" ENV_FILE="$TMP_DIR/observer.env" BIN="$TMP_DIR/capture-runtime" \
    bash "$START_NODE"
)
test -s "$capture_path"
grep -Fx -- 'config/public-testnet-governed-bootstrap-manifest-2026-06-06.json' "$capture_path" >/dev/null
grep -Fx -- "$(python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$stage_registry")" "$capture_path" >/dev/null
grep -Fx -- '--genesis-validator-registry' "$capture_path" >/dev/null
grep -Fx -- '--network-tier-manifest' "$capture_path" >/dev/null
if grep -Fx -- '--deployment-inventory' "$capture_path" >/dev/null; then
  echo "non-managed arbitrary stage forwarded deployment inventory" >&2
  exit 1
fi

# The same generated authority must still fail closed when the node identity
# is one of the managed triad IDs and no inventory is provided.
sed 's/^NODE_ID=.*/NODE_ID=triad-testnet-validator-47/' \
  "$TMP_DIR/observer.env" >"$TMP_DIR/managed.env"
if (
  cd "$TMP_DIR/stage"
  CAPTURE_PATH="$TMP_DIR/managed-runtime-args" \
    APP_ROOT="$PWD" ENV_FILE="$TMP_DIR/managed.env" BIN="$TMP_DIR/capture-runtime" \
    bash "$START_NODE"
) >"$TMP_DIR/managed.stderr" 2>&1; then
  echo "managed triad startup unexpectedly passed without deployment inventory" >&2
  exit 1
fi
grep -Fq 'managed triad startup requires DEPLOYMENT_INVENTORY_PATH' "$TMP_DIR/managed.stderr"

# These focused tests exercise both supported launcher argument builders. They
# remain intentionally narrower than a complete launcher/gameplay integration.
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher observer_registry_launcher_tests -- --test-threads=1
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher observer_registry_launcher_tests -- --test-threads=1

printf '%s\n' 'ok: launcher authority, generated non-managed stage binding, and managed fail-closed preflight'
