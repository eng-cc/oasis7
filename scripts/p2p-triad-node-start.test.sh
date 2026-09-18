#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/config/chain-pos-defaults.env"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-triad-start-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

mkdir -p "$TMP_DIR/current/bin" "$TMP_DIR/config"
cat >"$TMP_DIR/current/bin/oasis7_chain_runtime" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$@"
EOF
chmod +x "$TMP_DIR/current/bin/oasis7_chain_runtime"

write_env() {
  local env_path=$1
  local manifest_path=${2:-}
  local adaptive_value=${3:-0}
  local inventory_path=${4:-}
  cat >"$env_path" <<EOF
STACK_ROOT=$TMP_DIR
NODE_ID=test-node
WORLD_ID=oasis7-public-testnet-governed-20260606
NODE_ROLE=observer
STORAGE_PROFILE=release_default
STATUS_BIND=127.0.0.1:19083
NODE_GOSSIP_BIND=127.0.0.1:19084
NODE_TICK_MS=200
POS_SLOT_DURATION_MS=${POS_SLOT_DURATION_MS}
POS_TICKS_PER_SLOT=${POS_TICKS_PER_SLOT}
POS_PROPOSAL_TICK_PHASE=${POS_PROPOSAL_TICK_PHASE}
POS_MAX_PAST_SLOT_LAG=${POS_MAX_PAST_SLOT_LAG}
POS_ADAPTIVE_TICK_SCHEDULER=${adaptive_value}
REWARD_RUNTIME_ENABLE=1
REWARD_RUNTIME_EPOCH_DURATION_SECS=60
REWARD_POINTS_PER_CREDIT=100
REWARD_RUNTIME_AUTO_REDEEM=0
CONFIG_PATH=$TMP_DIR/config/node-keypair.toml
EXECUTION_WORLD_DIR=$TMP_DIR/data/execution-world
EXECUTION_RECORDS_DIR=$TMP_DIR/data/execution-records
STORAGE_ROOT=$TMP_DIR/data/storage
REPLICATION_NETWORK_LISTEN_ADDRS_CSV=/ip4/127.0.0.1/tcp/19085
REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV=/ip4/127.0.0.1/tcp/19086/p2p/12D3KooWLegacyPeer
REPLICATION_REMOTE_WRITERS_CSV=
  TRAFFIC_MONITOR_ENABLE=0
EOF
  if [[ -n "$manifest_path" ]]; then
    printf 'NETWORK_TIER_MANIFEST_PATH=%s\n' "$manifest_path" >>"$env_path"
    printf 'GENESIS_VALIDATOR_REGISTRY_PATH=%s\n' "$TMP_DIR/config/registry.json" >>"$env_path"
    if [[ -n "$inventory_path" ]]; then
      printf 'DEPLOYMENT_INVENTORY_PATH=%s\n' "$inventory_path" >>"$env_path"
    fi
  fi
}

write_env_without_adaptive() {
  local env_path=$1
  local manifest_path=${2:-}
  write_env "$env_path" "$manifest_path" 0 "$TMP_DIR/config/deployment-inventory.json"
  python3 - "$env_path" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
lines = [
    line
    for line in path.read_text(encoding="utf-8").splitlines()
    if not line.startswith("POS_ADAPTIVE_TICK_SCHEDULER=")
]
path.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY
}

manifest_path="$TMP_DIR/config/network-tier.json"
printf '{}\n' >"$manifest_path"
inventory_path="$TMP_DIR/config/deployment-inventory.json"
printf '{}\n' >"$inventory_path"
printf '{}\n' >"$TMP_DIR/config/registry.json"

write_env "$TMP_DIR/manifest.env" "$manifest_path" 0 "$inventory_path"
manifest_output=$(APP_ROOT="$TMP_DIR" ENV_FILE="$TMP_DIR/manifest.env" OASIS7_NODE_START_DRY_RUN=1 "$ROOT_DIR/scripts/p2p-triad-node-start.sh")
grep -q -- "--network-tier-manifest" <<<"$manifest_output"
grep -q -- "--genesis-validator-registry" <<<"$manifest_output"
if grep -q -- "--deployment-inventory" <<<"$manifest_output"; then
  echo "non-managed public-testnet observer must not forward deployment inventory" >&2
  exit 1
fi
grep -q -- "--pos-no-adaptive-tick-scheduler" <<<"$manifest_output"
if grep -q -- "--replication-network-peer" <<<"$manifest_output"; then
  echo "manifest-backed start must not pass REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV" >&2
  exit 1
fi

write_env_without_adaptive "$TMP_DIR/manifest-default-adaptive.env" "$manifest_path"
manifest_default_adaptive_output=$(APP_ROOT="$TMP_DIR" ENV_FILE="$TMP_DIR/manifest-default-adaptive.env" OASIS7_NODE_START_DRY_RUN=1 "$ROOT_DIR/scripts/p2p-triad-node-start.sh")
grep -q -- "--network-tier-manifest" <<<"$manifest_default_adaptive_output"
grep -q -- "--pos-adaptive-tick-scheduler" <<<"$manifest_default_adaptive_output"

write_env "$TMP_DIR/non-triad-public.env" "$manifest_path"
non_triad_public_output=$(APP_ROOT="$TMP_DIR" ENV_FILE="$TMP_DIR/non-triad-public.env" OASIS7_NODE_START_DRY_RUN=1 "$ROOT_DIR/scripts/p2p-triad-node-start.sh")
grep -q -- "--network-tier-manifest" <<<"$non_triad_public_output"
grep -q -- "--genesis-validator-registry" <<<"$non_triad_public_output"
if grep -q -- "--deployment-inventory" <<<"$non_triad_public_output"; then
  echo "non-managed public-testnet observer must not forward deployment inventory" >&2
  exit 1
fi

managed_with_inventory_env="$TMP_DIR/managed-with-inventory.env"
sed 's/^NODE_ID=test-node$/NODE_ID=triad-testnet-storage/' "$TMP_DIR/manifest.env" >"$managed_with_inventory_env"
managed_with_inventory_output=$(APP_ROOT="$TMP_DIR" ENV_FILE="$managed_with_inventory_env" OASIS7_NODE_START_DRY_RUN=1 "$ROOT_DIR/scripts/p2p-triad-node-start.sh")
grep -q -- "--deployment-inventory" <<<"$managed_with_inventory_output"
grep -q -- "$inventory_path" <<<"$managed_with_inventory_output"

managed_missing_inventory_env="$TMP_DIR/managed-missing-inventory.env"
sed 's/^NODE_ID=test-node$/NODE_ID=triad-testnet-storage/' "$TMP_DIR/non-triad-public.env" >"$managed_missing_inventory_env"
if APP_ROOT="$TMP_DIR" ENV_FILE="$managed_missing_inventory_env" OASIS7_NODE_START_DRY_RUN=1 "$ROOT_DIR/scripts/p2p-triad-node-start.sh" >"$TMP_DIR/missing-inventory.out" 2>&1; then
  echo "managed triad startup must reject a missing deployment inventory" >&2
  exit 1
fi
grep -q -- "managed triad startup requires DEPLOYMENT_INVENTORY_PATH" "$TMP_DIR/missing-inventory.out"

ln -s "$inventory_path" "$TMP_DIR/config/symlink-inventory.json"
write_env "$TMP_DIR/symlink-inventory.env" "$manifest_path" 0 "$TMP_DIR/config/symlink-inventory.json"
sed -i.bak 's/^NODE_ID=test-node$/NODE_ID=triad-testnet-storage/' "$TMP_DIR/symlink-inventory.env"
rm -f "$TMP_DIR/symlink-inventory.env.bak"
if APP_ROOT="$TMP_DIR" ENV_FILE="$TMP_DIR/symlink-inventory.env" OASIS7_NODE_START_DRY_RUN=1 "$ROOT_DIR/scripts/p2p-triad-node-start.sh" >"$TMP_DIR/symlink-inventory.out" 2>&1; then
  echo "managed triad startup must reject a symlinked deployment inventory" >&2
  exit 1
fi
grep -q -- "deployment inventory must be a regular non-symlink file" "$TMP_DIR/symlink-inventory.out"

write_env "$TMP_DIR/legacy.env"
legacy_output=$(APP_ROOT="$TMP_DIR" ENV_FILE="$TMP_DIR/legacy.env" OASIS7_NODE_START_DRY_RUN=1 "$ROOT_DIR/scripts/p2p-triad-node-start.sh")
grep -q -- "--replication-network-peer" <<<"$legacy_output"
grep -q -- "12D3KooWLegacyPeer" <<<"$legacy_output"

override_output=$(APP_ROOT="$TMP_DIR" ENV_FILE="$TMP_DIR/manifest.env" OASIS7_NODE_START_DRY_RUN=1 ALLOW_NETWORK_TIER_REPLICATION_PEER_ENV_OVERRIDE=1 "$ROOT_DIR/scripts/p2p-triad-node-start.sh")
grep -q -- "--replication-network-peer" <<<"$override_output"
grep -q -- "12D3KooWLegacyPeer" <<<"$override_output"

echo "p2p-triad-node-start checks passed"
