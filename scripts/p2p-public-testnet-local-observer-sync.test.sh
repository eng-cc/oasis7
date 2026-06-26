#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/oasis7-local-observer-sync.XXXXXX")
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

cat >"$tmp_dir/local.env" <<EOF
HOST_LABEL=local-a
SERVICE_NAME=triad-testnet-local
STACK_ROOT=$tmp_dir/local-stack
NODE_ID=triad-testnet-local
WORLD_ID=oasis7-public-testnet-governed-20260606
NODE_ROLE=observer
STATUS_BIND=127.0.0.1:19082
NODE_GOSSIP_BIND=0.0.0.0:19385
NODE_AUTO_ATTEST_FLAG=--node-no-auto-attest-all
CONFIG_PATH=\$STACK_ROOT/config.toml
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

cat >"$tmp_dir/sequencer.env" <<'EOF'
WORLD_ID=oasis7-public-testnet-governed-20260606
NODE_ROLE=sequencer
STORAGE_PROFILE=dev_local
NODE_GOSSIP_PEERS_CSV=39.104.205.67:6732
REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV=/ip4/39.104.204.172/tcp/6831/p2p/12D3KooWMyPapumCaTABq27umWdHqXDr8AoTse21eMVnXeJEsbNp
REPLICATION_REMOTE_WRITERS_CSV=aa,bb
POS_SLOT_CLOCK_GENESIS_UNIX_MS=1779068751846
POS_ADAPTIVE_TICK_SCHEDULER=0
NODE_TICK_MS=200
POS_SLOT_DURATION_MS=12000
POS_TICKS_PER_SLOT=10
POS_PROPOSAL_TICK_PHASE=9
POS_MAX_PAST_SLOT_LAG=256
REWARD_RUNTIME_ENABLE=1
REWARD_RUNTIME_EPOCH_DURATION_SECS=60
REWARD_POINTS_PER_CREDIT=100
REWARD_RUNTIME_AUTO_REDEEM=0
NODE_VALIDATORS_CSV=triad-testnet-sequencer:100,triad-testnet-storage:50
NODE_VALIDATOR_SIGNERS_CSV=triad-testnet-sequencer:65c27d,triad-testnet-storage:858e97
EOF

cat >"$tmp_dir/storage.env" <<'EOF'
WORLD_ID=oasis7-public-testnet-governed-20260606
NODE_ROLE=storage
STORAGE_PROFILE=dev_local
NODE_GOSSIP_PEERS_CSV=39.104.204.172:6731
REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV=/ip4/39.104.205.67/tcp/6832/p2p/12D3KooWAuNCCEDu7CdUUDwALuAhuLekZHgVWxAYp4Ag5ti79fJj
REPLICATION_REMOTE_WRITERS_CSV=bb,cc
POS_SLOT_CLOCK_GENESIS_UNIX_MS=1779068751846
POS_ADAPTIVE_TICK_SCHEDULER=0
NODE_TICK_MS=200
POS_SLOT_DURATION_MS=12000
POS_TICKS_PER_SLOT=10
POS_PROPOSAL_TICK_PHASE=9
POS_MAX_PAST_SLOT_LAG=256
REWARD_RUNTIME_ENABLE=1
REWARD_RUNTIME_EPOCH_DURATION_SECS=60
REWARD_POINTS_PER_CREDIT=100
REWARD_RUNTIME_AUTO_REDEEM=0
NODE_VALIDATORS_CSV=triad-testnet-sequencer:100,triad-testnet-storage:50
NODE_VALIDATOR_SIGNERS_CSV=triad-testnet-sequencer:65c27d,triad-testnet-storage:858e97
EOF

cat >"$tmp_dir/manifest.json" <<'EOF'
{
  "network_id": "public_testnet"
}
EOF

rendered_env="$tmp_dir/rendered.env"
./scripts/p2p-public-testnet-local-observer-sync.sh render \
  --local-env "$tmp_dir/local.env" \
  --sequencer-env "$tmp_dir/sequencer.env" \
  --storage-env "$tmp_dir/storage.env" \
  --manifest-path "$tmp_dir/manifest.json" \
  --out "$rendered_env"

grep -q '^WORLD_ID=oasis7-public-testnet-governed-20260606$' "$rendered_env"
grep -q '^NODE_ROLE=observer$' "$rendered_env"
grep -Eq '^RUNTIME_ROOT=.*/runtime-root$' "$rendered_env"
grep -Eq '^REPLICATION_ROOT=.*/replication-root$' "$rendered_env"
grep -q '^NODE_GOSSIP_PEERS_CSV=39.104.204.172:6731,39.104.205.67:6732$' "$rendered_env"
grep -q '^REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV=/ip4/39.104.205.67/tcp/6832/p2p/12D3KooWAuNCCEDu7CdUUDwALuAhuLekZHgVWxAYp4Ag5ti79fJj,/ip4/39.104.204.172/tcp/6831/p2p/12D3KooWMyPapumCaTABq27umWdHqXDr8AoTse21eMVnXeJEsbNp$' "$rendered_env"
grep -q '^REPLICATION_REMOTE_WRITERS_CSV=bb,cc,aa$' "$rendered_env"

grep -q 'remote_optional_resolved_env_value.*REPLICATION_ROOT' scripts/p2p-public-testnet-local-observer-sync.sh
grep -q 'remote_replication_root="$remote_stack_root/output/node-distfs/$remote_node_id"' scripts/p2p-public-testnet-local-observer-sync.sh
grep -q 'REMOTE_EXECUTION_BRIDGE_STATE_REQUIRED' scripts/p2p-public-testnet-local-observer-sync.sh
grep -q 'execution_bridge_state_path_for_root "$remote_stack_root" "$remote_node_id" "$remote_runtime_root"' scripts/p2p-public-testnet-local-observer-sync.sh

local_stack="$tmp_dir/local-stack"
mkdir -p \
  "$local_stack/world" \
  "$local_stack/world-simulator-mirror" \
  "$local_stack/execution-records" \
  "$local_stack/store/blobs" \
  "$local_stack/runtime-root" \
  "$local_stack/replication-root"
printf '{"height":1233}\n' >"$local_stack/world/snapshot.json"
printf '{"mirror":"old"}\n' >"$local_stack/world-simulator-mirror/snapshot.json"
printf '{"height":1233}\n' >"$local_stack/execution-records/latest.json"
printf 'old blob\n' >"$local_stack/store/blobs/old"
printf '{"runtime":"old"}\n' >"$local_stack/runtime-root/reward-runtime-execution-bridge-state.json"
printf '{"committed_height":1233}\n' >"$local_stack/replication-root/node_pos_state.json"

reset_backup="$tmp_dir/reset-backup"
./scripts/p2p-public-testnet-local-observer-sync.sh reset-state \
  --local-env "$tmp_dir/local.env" \
  --backup-dir "$reset_backup"

test -f "$reset_backup/execution-world/snapshot.json"
test -f "$reset_backup/execution-world-simulator-mirror/snapshot.json"
test -f "$reset_backup/execution-records/latest.json"
test -f "$reset_backup/storage/blobs/old"
test -f "$reset_backup/runtime-root/reward-runtime-execution-bridge-state.json"
test -f "$reset_backup/replication-root/node_pos_state.json"
test ! -e "$local_stack/world/snapshot.json"
test ! -e "$local_stack/world-simulator-mirror/snapshot.json"
test ! -e "$local_stack/execution-records/latest.json"
test ! -e "$local_stack/store/blobs/old"
test ! -e "$local_stack/runtime-root/reward-runtime-execution-bridge-state.json"
test ! -e "$local_stack/replication-root/node_pos_state.json"

echo "ok: local observer sync accepts sequencer/storage validator env pair"
