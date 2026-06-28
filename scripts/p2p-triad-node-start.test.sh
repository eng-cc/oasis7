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
POS_ADAPTIVE_TICK_SCHEDULER=0
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
  fi
}

manifest_path="$TMP_DIR/config/network-tier.json"
printf '{}\n' >"$manifest_path"

write_env "$TMP_DIR/manifest.env" "$manifest_path"
manifest_output=$(APP_ROOT="$TMP_DIR" ENV_FILE="$TMP_DIR/manifest.env" OASIS7_NODE_START_DRY_RUN=1 "$ROOT_DIR/scripts/p2p-triad-node-start.sh")
grep -q -- "--network-tier-manifest" <<<"$manifest_output"
if grep -q -- "--replication-network-peer" <<<"$manifest_output"; then
  echo "manifest-backed start must not pass REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV" >&2
  exit 1
fi

write_env "$TMP_DIR/legacy.env"
legacy_output=$(APP_ROOT="$TMP_DIR" ENV_FILE="$TMP_DIR/legacy.env" OASIS7_NODE_START_DRY_RUN=1 "$ROOT_DIR/scripts/p2p-triad-node-start.sh")
grep -q -- "--replication-network-peer" <<<"$legacy_output"
grep -q -- "12D3KooWLegacyPeer" <<<"$legacy_output"

override_output=$(APP_ROOT="$TMP_DIR" ENV_FILE="$TMP_DIR/manifest.env" OASIS7_NODE_START_DRY_RUN=1 ALLOW_NETWORK_TIER_REPLICATION_PEER_ENV_OVERRIDE=1 "$ROOT_DIR/scripts/p2p-triad-node-start.sh")
grep -q -- "--replication-network-peer" <<<"$override_output"
grep -q -- "12D3KooWLegacyPeer" <<<"$override_output"

echo "p2p-triad-node-start checks passed"
