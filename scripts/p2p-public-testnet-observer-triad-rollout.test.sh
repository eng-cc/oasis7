#!/usr/bin/env bash
set -euo pipefail

# Isolated observer-sync acceptance for a pair -> triad rollout. The test
# supplies the same three-validator truth to both ECS env adapters, binds that
# generated registry to an isolated inventory, and reaches the launcher with
# no service or runtime process started.
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SYNC="$ROOT_DIR/scripts/p2p-public-testnet-local-observer-sync.sh"
START_NODE="$ROOT_DIR/scripts/p2p-triad-node-start.sh"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-observer-triad-rollout.XXXXXX")"
trap 'rm -rf -- "$TMP_DIR"' EXIT

stack="$TMP_DIR/stack"
mkdir -p "$stack/config" "$TMP_DIR/config"

cat >"$TMP_DIR/local.env" <<EOF
HOST_LABEL=triad-local
SERVICE_NAME=triad-testnet-local
STACK_ROOT=$stack
NODE_ID=triad-testnet-local
NODE_ROLE=observer
STATUS_BIND=127.0.0.1:19082
NODE_GOSSIP_BIND=0.0.0.0:19385
NODE_AUTO_ATTEST_FLAG=--node-no-auto-attest-all
CONFIG_PATH=\$STACK_ROOT/config/node-keypair.toml
EXECUTION_WORLD_DIR=\$STACK_ROOT/world
EXECUTION_RECORDS_DIR=\$STACK_ROOT/execution-records
STORAGE_ROOT=\$STACK_ROOT/store
RUNTIME_ROOT=\$STACK_ROOT/runtime-root
REPLICATION_ROOT=\$STACK_ROOT/replication-root
REPLICATION_NETWORK_LISTEN_ADDRS_CSV=/ip4/0.0.0.0/tcp/19375
TRAFFIC_PROFILE=triad_low_traffic
TRAFFIC_MONITOR_ENABLE=0
TRAFFIC_MONITOR_INTERVAL_SECS=30
TRAFFIC_MONITOR_WINDOW_MINUTES=10
TRAFFIC_MONITOR_TOP_N=5
TRAFFIC_MONITOR_OUTPUT_DIR=\$STACK_ROOT/output/traffic-monitor
P2P_NODE_ROLE=observer_light
EOF

cat >"$TMP_DIR/sequencer.env" <<'EOF'
WORLD_ID=oasis7-public-testnet-governed-20260606
NODE_ROLE=sequencer
STORAGE_PROFILE=dev_local
NODE_GOSSIP_PEERS_CSV=sequencer-peer
REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV=sequencer-replication-peer
REPLICATION_REMOTE_WRITERS_CSV=sequencer-writer
POS_SLOT_CLOCK_GENESIS_UNIX_MS=1779068751846
POS_ADAPTIVE_TICK_SCHEDULER=0
NODE_TICK_MS=200
POS_SLOT_DURATION_MS=8000
POS_TICKS_PER_SLOT=10
POS_PROPOSAL_TICK_PHASE=9
POS_MAX_PAST_SLOT_LAG=256
REWARD_RUNTIME_ENABLE=1
REWARD_RUNTIME_EPOCH_DURATION_SECS=60
REWARD_POINTS_PER_CREDIT=100
REWARD_RUNTIME_AUTO_REDEEM=0
NODE_VALIDATORS_CSV=triad-testnet-sequencer:100,triad-testnet-storage:100,triad-testnet-validator-47:100
NODE_VALIDATOR_SIGNERS_CSV=triad-testnet-sequencer:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,triad-testnet-storage:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,triad-testnet-validator-47:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc
EOF
cp "$TMP_DIR/sequencer.env" "$TMP_DIR/storage.env"
sed -i.bak \
  -e 's/^NODE_ROLE=sequencer$/NODE_ROLE=storage/' \
  -e 's/sequencer-peer/storage-peer/' \
  -e 's/sequencer-replication-peer/storage-replication-peer/' \
  -e 's/sequencer-writer/storage-writer/' \
  "$TMP_DIR/storage.env"
rm -f "$TMP_DIR/storage.env.bak"

registry="$TMP_DIR/triad-registry.json"
inventory="$TMP_DIR/triad-inventory.json"
python3 - "$registry" "$inventory" <<'PY'
import hashlib
import json
import pathlib
import sys

registry_path = pathlib.Path(sys.argv[1])
inventory_path = pathlib.Path(sys.argv[2])
validators = [
    {"node_id": "triad-testnet-sequencer", "scheme": "ed25519", "finality_signer_public_key": "a" * 64, "stake": 100},
    {"node_id": "triad-testnet-storage", "scheme": "ed25519", "finality_signer_public_key": "b" * 64, "stake": 100},
    {"node_id": "triad-testnet-validator-47", "scheme": "ed25519", "finality_signer_public_key": "c" * 64, "stake": 100},
]
registry = {"slot_id": "governance.finality.v1", "threshold": 3, "threshold_bps": 0, "validators": validators}
registry_path.write_text(json.dumps(registry, ensure_ascii=True, indent=2, sort_keys=True) + "\n", encoding="utf-8")
canonical = {
    "signer_bindings": {
        f"governance.finality.v1.{item['node_id']}": item["finality_signer_public_key"]
        for item in validators
    },
    "slot_id": registry["slot_id"],
    "threshold": registry["threshold"],
    "threshold_bps": registry["threshold_bps"],
    "validator_stakes": {
        f"governance.finality.v1.{item['node_id']}": item["stake"]
        for item in validators
    },
}
inventory = {
    "schema_version": "oasis7.public_testnet_validator_triad_inventory.v1",
    "network_tier": "public_testnet",
    "topology": "three_equal_validator",
    "authority": {
        "generated_registry_sha256": hashlib.sha256(registry_path.read_bytes()).hexdigest(),
        "generated_registry_semantic_sha256": hashlib.sha256(
            json.dumps(canonical, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest(),
    },
    "nodes": {item["node_id"]: {"node_id": item["node_id"]} for item in validators},
}
inventory_path.write_text(json.dumps(inventory, ensure_ascii=True, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY

registry_sha=$(shasum -a 256 "$registry" | awk '{print $1}')
registry_semantic_sha=$(python3 - "$registry" <<'PY'
import hashlib
import json
import pathlib
import sys
value = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
canonical = {
    "signer_bindings": {
        f"governance.finality.v1.{item['node_id']}": item["finality_signer_public_key"]
        for item in value["validators"]
    },
    "slot_id": value["slot_id"],
    "threshold": value["threshold"],
    "threshold_bps": value["threshold_bps"],
    "validator_stakes": {
        f"governance.finality.v1.{item['node_id']}": item["stake"]
        for item in value["validators"]
    },
}
print(hashlib.sha256(json.dumps(canonical, sort_keys=True, separators=(",", ":")).encode()).hexdigest())
PY
)
manifest="$TMP_DIR/manifest.json"
python3 - "$manifest" "$registry_sha" "$registry_semantic_sha" <<'PY'
import json
import pathlib
import sys
path = pathlib.Path(sys.argv[1])
path.write_text(json.dumps({
    "schema_version": "oasis7.network_tier_manifest.v1",
    "tier": "public_testnet",
    "network_id": "oasis7-public-testnet-governed-20260606",
    "chain_id": "oasis7-public-testnet-governed-20260606",
    "validator_policy": {"target_validator_count": 3, "allow_observer_nodes": True},
    "deployment_validator_registry": {
        "ref": "config/triad-registry.json",
        "sha256": sys.argv[2],
        "semantic_sha256": sys.argv[3],
    },
    "runtime_refs": {},
}, ensure_ascii=True, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
cp "$registry" "$stack/config/triad-registry.json"
cp "$registry" "$TMP_DIR/config/triad-registry.json"

# The apply path is the pair -> triad adapter boundary: both sequencer and
# storage must expose the exact third-validator truth and bind count=3 to the
# supplied inventory. The resulting observer env remains registry-only.
"$SYNC" apply \
  --local-env "$TMP_DIR/local.env" \
  --sequencer-env "$TMP_DIR/sequencer.env" \
  --storage-env "$TMP_DIR/storage.env" \
  --manifest-path "$manifest" \
  --deployment-inventory "$inventory" \
  --start-script-dest "$stack/bin/start-node.sh" \
  --backup-dir "$TMP_DIR/apply-backup" >/dev/null

generated_registry="$stack/config/genesis-validator-registry.json"
test "$(jq '.validators | length' "$generated_registry")" -eq 3
jq -e '[.validators[].node_id] | sort == ["triad-testnet-sequencer", "triad-testnet-storage", "triad-testnet-validator-47"]' "$generated_registry" >/dev/null
cp "$generated_registry" "$TMP_DIR/config/genesis-validator-registry.json"
grep -q '^GENESIS_VALIDATOR_REGISTRY_PATH=.*/config/genesis-validator-registry.json$' "$TMP_DIR/local.env"
grep -q '^GENESIS_VALIDATOR_REGISTRY_SHA256=' "$TMP_DIR/local.env"
grep -q '^GENESIS_VALIDATOR_REGISTRY_SEMANTIC_SHA256=' "$TMP_DIR/local.env"
if grep -q '^DEPLOYMENT_INVENTORY_PATH=' "$TMP_DIR/local.env"; then
  echo "non-managed observer env must not serialize an inventory variable" >&2
  exit 1
fi

printf '[node]\nkey = "observer-triad-rollout-test-only"\n' >"$stack/config/node-keypair.toml"
cat >"$TMP_DIR/capture-runtime" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$@" >"$CAPTURE_PATH"
EOF
chmod 0755 "$TMP_DIR/capture-runtime"
CAPTURE_PATH="$TMP_DIR/observer-runtime.args" \
  APP_ROOT="$stack" ENV_FILE="$TMP_DIR/local.env" BIN="$TMP_DIR/capture-runtime" \
  bash "$START_NODE"
grep -Fx -- '--network-tier-manifest' "$TMP_DIR/observer-runtime.args" >/dev/null
grep -Fx -- '--genesis-validator-registry' "$TMP_DIR/observer-runtime.args" >/dev/null
grep -Fx -- "$(python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$generated_registry")" "$TMP_DIR/observer-runtime.args" >/dev/null
if grep -Fx -- '--deployment-inventory' "$TMP_DIR/observer-runtime.args" >/dev/null; then
  echo "non-managed observer launcher unexpectedly forwarded deployment inventory" >&2
  exit 1
fi

# Managed pair members need the same generated inventory binding when their
# node-specific launcher env is exercised. The fake runtime proves both
# sequencer and storage reach the launcher without starting a process.
inventory_sha=$(shasum -a 256 "$inventory" | awk '{print $1}')
canonical_inventory=$(python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$inventory")
for role in sequencer storage; do
  managed_env="$TMP_DIR/managed-$role.env"
  sed \
    -e "s/^NODE_ID=.*/NODE_ID=triad-testnet-$role/" \
    -e "s/^NODE_ROLE=.*/NODE_ROLE=$role/" \
    "$TMP_DIR/local.env" >"$managed_env"
  if [[ "$role" == storage ]]; then
    p2p_role=full_storage
  else
    p2p_role=sequencer
  fi
  # Re-establish a complete env-file record before appending managed-only
  # authority fields, independent of the source env's final-newline style.
  printf '\n' >>"$managed_env"
  printf '%s\n' \
    "P2P_NODE_ROLE=$p2p_role" \
    "DEPLOYMENT_INVENTORY_PATH=$inventory" \
    "DEPLOYMENT_INVENTORY_SHA256=$inventory_sha" >>"$managed_env"
  CAPTURE_PATH="$TMP_DIR/$role-runtime.args" \
    APP_ROOT="$stack" ENV_FILE="$managed_env" BIN="$TMP_DIR/capture-runtime" \
    bash "$START_NODE"
  python3 - "$TMP_DIR/$role-runtime.args" "$canonical_inventory" <<'PY'
import os
import pathlib
import sys

args = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
expected = sys.argv[2]
try:
    inventory_arg = args[args.index("--deployment-inventory") + 1]
except (ValueError, IndexError):
    raise SystemExit("managed triad launcher did not forward deployment inventory")
if os.path.realpath(inventory_arg) != expected:
    raise SystemExit(
        f"managed triad launcher forwarded the wrong inventory: {inventory_arg}"
    )
PY
done

# A triad manifest cannot be rendered without the explicit third-validator
# authority, and pair-only ECS input cannot be promoted against that manifest.
if "$SYNC" render \
  --local-env "$TMP_DIR/local.env" \
  --sequencer-env "$TMP_DIR/sequencer.env" \
  --storage-env "$TMP_DIR/storage.env" \
  --manifest-path "$manifest" >/dev/null 2>"$TMP_DIR/missing-inventory.stderr"; then
  echo "triad observer render unexpectedly accepted missing deployment inventory" >&2
  exit 1
fi
grep -Fq 'three-validator observer rollout requires --deployment-inventory' "$TMP_DIR/missing-inventory.stderr" || {
  cat "$TMP_DIR/missing-inventory.stderr" >&2
  exit 1
}

python3 - "$TMP_DIR/sequencer.env" "$TMP_DIR/pair-sequencer.env" <<'PY'
import pathlib
import sys

source, destination = map(pathlib.Path, sys.argv[1:3])
lines = source.read_text(encoding="utf-8").splitlines()
destination.write_text(
    "\n".join(
        line.replace(",triad-testnet-validator-47:100", "")
        .replace(",triad-testnet-validator-47:" + "c" * 64, "")
        for line in lines
    )
    + "\n",
    encoding="utf-8",
)
PY
python3 - "$TMP_DIR/storage.env" "$TMP_DIR/pair-storage.env" <<'PY'
import pathlib
import sys

source, destination = map(pathlib.Path, sys.argv[1:3])
lines = source.read_text(encoding="utf-8").splitlines()
destination.write_text(
    "\n".join(
        line.replace(",triad-testnet-validator-47:100", "")
        .replace(",triad-testnet-validator-47:" + "c" * 64, "")
        for line in lines
    )
    + "\n",
    encoding="utf-8",
)
PY
grep '^NODE_VALIDATORS_CSV=' "$TMP_DIR/pair-sequencer.env" >/dev/null || {
  cat "$TMP_DIR/pair-sequencer.env" >&2
  exit 1
}
grep '^NODE_VALIDATORS_CSV=' "$TMP_DIR/pair-storage.env" >/dev/null || {
  cat "$TMP_DIR/pair-storage.env" >&2
  exit 1
}
if "$SYNC" apply \
  --local-env "$TMP_DIR/local.env" \
  --sequencer-env "$TMP_DIR/pair-sequencer.env" \
  --storage-env "$TMP_DIR/pair-storage.env" \
  --manifest-path "$manifest" \
  --deployment-inventory "$inventory" \
  --start-script-dest "$stack/bin/start-node.sh" \
  --backup-dir "$TMP_DIR/pair-apply-backup" >/dev/null 2>"$TMP_DIR/pair.stderr"; then
  echo "pair-only ECS truth unexpectedly passed triad observer apply" >&2
  exit 1
fi
grep -Eiq 'count|registry|validator-47|three' "$TMP_DIR/pair.stderr" || {
  cat "$TMP_DIR/pair.stderr" >&2
  exit 1
}

printf '%s\n' 'ok: observer triad rollout binds both pair envs to explicit validator-47 truth and count=3'
